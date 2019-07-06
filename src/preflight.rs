use crate::config::Config;

pub fn check(config: &Config) -> Vec<String> {
    let mut warnings = vec![];

    if config.gh_token.is_none() {
        warnings.push("The GH_TOKEN environment variable is not configured".into());
    }

    if config.cargo_token.is_none() {
        warnings.push("The CARGO_TOKEN environment variable is not configured. Cannot create release on crates.io".into());
    }

    if let Err(ref err) = config.remote {
        warnings.push(format!("Could not determine the origin remote: {:?}", err));
        warnings.push("semantic-rs can't push changes or create a release on GitHub".into());
    } else {
        let remote_name = config.remote.as_ref().unwrap();
        let remote_url = crate::git::get_remote_url(config, remote_name);
        match remote_url {
            Ok(Some(remote_url)) => log::info!("Current remote: {}({})", remote_name, remote_url),
            Ok(None) => log::info!("Current remote: {}", remote_name),
            Err(err) => warnings.push(format!(
                "Could not determine the origin remote url: {}",
                err
            )),
        }
    }

    warnings
}
