use std::fmt::Display;
use std::io::Write;
use std::ops::Try;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use failure::Fail;

use crate::plugin_support::flow::{FlowError, Value};
use crate::plugin_support::keys::{GIT_BRANCH, GIT_REMOTE_URL, NEXT_VERSION};
use crate::plugin_support::proto::response::{self, PluginResponse};
use crate::plugin_support::{PluginInterface, PluginStep};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct DockerPlugin {
    config: Config,
    state: Option<State>,
}

impl DockerPlugin {
    pub fn new() -> Self {
        DockerPlugin::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    repo_url: Value<String>,
    repo_branch: Value<String>,
    next_version: Value<semver::Version>,
    images: Value<Vec<Image>>,
    docker_user: Value<String>,
    docker_password: Value<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            repo_url: Value::builder(GIT_REMOTE_URL).required_at(PluginStep::Publish).build(),
            repo_branch: Value::builder(GIT_BRANCH).required_at(PluginStep::Publish).build(),
            next_version: Value::builder(NEXT_VERSION).required_at(PluginStep::Publish).build(),
            images: Value::builder("images").default_value().build(),
            docker_user: Value::builder("DOCKER_USER").load_from_env().build(),
            docker_password: Value::builder("DOCKER_PASSWORD").load_from_env().build(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Image {
    registry: Registry,
    dockerfile: PathBuf,
    namespace: Option<String>,
    name: String,
    tag: String,
    binary_path: String,
    build_cmd: String,
    exec_cmd: String,
    cleanup: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum Registry {
    Dockerhub,
}

struct State {
    credentials: Option<Credentials>,
}

struct Credentials {
    username: String,
    password: String,
}

impl PluginInterface for DockerPlugin {
    fn name(&self) -> response::Name {
        PluginResponse::from_ok("docker".into())
    }

    fn methods(&self) -> response::Methods {
        PluginResponse::from_ok(vec![PluginStep::PreFlight, PluginStep::Publish])
    }

    fn provision_capabilities(&self) -> response::ProvisionCapabilities {
        PluginResponse::from_ok(vec![])
    }

    fn get_value(&self, key: &str) -> response::GetValue {
        PluginResponse::from_error(FlowError::KeyNotSupported(key.to_owned()).into())
    }

    fn get_config(&self) -> response::Config {
        PluginResponse::from_ok(serde_json::to_value(&self.config)?)
    }

    fn set_config(&mut self, config: serde_json::Value) -> response::Null {
        self.config = serde_json::from_value(config)?;
        PluginResponse::from_ok(())
    }

    fn pre_flight(&mut self) -> response::Null {
        let mut response = PluginResponse::builder();

        let credentials = {
            let username = self.config.docker_user.as_value().clone();
            let password = self.config.docker_password.as_value().clone();
            Some(Credentials { username, password })
        };

        log::info!("Checking that docker daemon is running...");
        if let Err(err) = docker_info() {
            response.error(err);
        }

        self.state.replace(State { credentials });

        response.body(())
    }

    fn publish(&mut self) -> response::Null {
        let config = &self.config;
        let state = self.state.as_ref().ok_or(Error::MissingState)?;

        let credentials = state.credentials.as_ref().ok_or(Error::CredentialsUndefined)?;

        let version = config.next_version.as_value();
        let version = format!("{}", version);

        for image in config.images.as_value() {
            let registry_url = match image.registry {
                Registry::Dockerhub => None,
            };

            login(registry_url, &credentials)?;

            build_image(&config, image)?;

            // Tag as namespace/name/tag and namespace/name/version
            let from = format!("{}:{}", image.name, image.tag);
            tag_image(&from, &get_image_path(image, &image.tag))?;
            tag_image(&from, &get_image_path(image, &version))?;

            // Publish namespace/name/tag and namespace/name/version
            push_image(image, &image.tag)?;
            push_image(image, &version)?;
        }

        PluginResponse::from_ok(())
    }
}

fn get_image_path(image: &Image, tag: &str) -> String {
    if let Some(namespace) = image.namespace.as_ref() {
        format!("{}/{}:{}", namespace, image.name, tag)
    } else {
        format!("{}:{}", image.name, tag)
    }
}

fn docker_info() -> Result<(), failure::Error> {
    let status = Command::new("docker")
        .arg("info")
        .status()
        .map_err(|_| Error::DockerNotFound)?;

    if !status.success() {
        return Err(Error::DockerCommandFailed(status.code()).into());
    }

    Ok(())
}

fn build_image(config: &Config, image: &Image) -> Result<(), failure::Error> {
    let mut cmd = Command::new("docker");

    cmd.arg("build").arg(".docker").arg("--no-cache");

    // Set filename of Dockerfile
    cmd.arg("-f").arg(&image.dockerfile.display().to_string());

    // Set name and tag
    cmd.arg("-t").arg(&format!("{}:{}", image.name, image.tag));

    let mut set_env_var = |k, v: &dyn Display| {
        cmd.arg("--build-arg").arg(format!("{}={}", k, v));
    };

    // Set env vars
    set_env_var("REPO_URL", &config.repo_url.as_value());
    set_env_var("REPO_BRANCH", &config.repo_branch.as_value());
    set_env_var("BUILD_CMD", &image.build_cmd);
    set_env_var("BINARY_PATH", &image.binary_path);
    set_env_var("CLEANUP", &image.cleanup);
    set_env_var("EXEC_CMD", &image.exec_cmd);

    log::debug!("exec {:?}", cmd);

    let status = cmd.status()?;
    if !status.success() {
        return Err(Error::DockerCommandFailed(status.code()).into());
    }

    log::info!("Built image {}:{}", image.name, image.tag);

    Ok(())
}

fn tag_image(from: &str, to: &str) -> Result<(), failure::Error> {
    log::info!("tagging image {} as {}", from, to);

    let mut cmd = Command::new("docker");

    let status = cmd.arg("tag").arg(from).arg(to).status()?;

    if !status.success() {
        return Err(Error::DockerCommandFailed(status.code()).into());
    }

    Ok(())
}

fn login(registry_url: Option<&str>, credentials: &Credentials) -> Result<(), failure::Error> {
    log::info!("logging in as {}", credentials.username);

    let mut cmd = Command::new("docker");

    cmd.arg("login")
        .arg("--username")
        .arg(&credentials.username)
        .arg("--password-stdin");

    if let Some(url) = registry_url {
        cmd.arg(url);
    }

    let mut child = cmd.stdin(Stdio::piped()).spawn()?;

    {
        let stdin = child.stdin.as_mut().ok_or(Error::StdioPasswordPassingFailed)?;
        stdin.write_all(credentials.password.as_bytes())?;
    }

    let status = child.wait()?;

    if !status.success() {
        return Err(Error::DockerCommandFailed(status.code()).into());
    }

    Ok(())
}

fn push_image(image: &Image, tag: &str) -> Result<(), failure::Error> {
    let mut cmd = Command::new("docker");

    cmd.arg("push");

    let path = get_image_path(image, tag);
    log::info!("Publishing image {}", path);
    cmd.arg(path);

    let status = cmd.status()?;

    if !status.success() {
        return Err(Error::DockerCommandFailed(status.code()).into());
    }

    Ok(())
}

#[derive(Fail, Debug)]
enum Error {
    #[fail(display = "DOCKER_USER or DOCKER_PASSWORD are not set, cannot push the image.")]
    CredentialsUndefined,
    #[fail(display = "state is missing: forgot to call pre_flight?")]
    MissingState,
    #[fail(display = "docker command exited with error {:?}", _0)]
    DockerCommandFailed(Option<i32>),
    #[fail(display = "stdio error: failed to pass password to docker process via stdin")]
    StdioPasswordPassingFailed,
    #[fail(display = "'docker' not found in PATH: make sure you have the docker client installed")]
    DockerNotFound,
}
