use crate::asset::Asset;
use git2::{Repository, Signature};

pub struct Config {
    pub user: Option<String>,
    pub repository_name: Option<String>,

    pub branch: String,

    pub remote: Result<String, String>,

    pub repository_path: String,

    pub write_mode: bool,
    pub release_mode: bool,
    pub force_https: bool,

    pub repository: Repository,
    pub signature: Signature<'static>,

    pub gh_token: Option<String>,
    pub cargo_token: Option<String>,

    pub assets: Vec<Asset>,
}

impl Config {
    pub fn can_push(&self) -> bool {
        self.user.is_some() && self.repository_name.is_some()
    }

    pub fn can_release_to_github(&self) -> bool {
        self.can_push() && self.gh_token.is_some()
    }

    pub fn can_release_to_cratesio(&self) -> bool {
        self.cargo_token.is_some()
    }
}

pub struct ConfigBuilder {
    user: Option<String>,
    repository_name: Option<String>,

    branch: Option<String>,

    repository_path: Option<String>,

    remote: Option<Result<String, String>>,

    write_mode: bool,
    release_mode: bool,
    force_https: bool,

    repository: Option<Repository>,
    signature: Option<Signature<'static>>,

    gh_token: Option<String>,
    cargo_token: Option<String>,

    assets: Vec<Asset>,
}

impl ConfigBuilder {
    pub fn new() -> ConfigBuilder {
        ConfigBuilder {
            user: None,
            repository_name: None,
            branch: None,
            repository_path: None,
            write_mode: false,
            release_mode: false,
            force_https: false,
            repository: None,
            signature: None,
            gh_token: None,
            cargo_token: None,
            remote: None,
            assets: vec![],
        }
    }

    pub fn user(&mut self, user: String) -> &mut Self {
        self.user = Some(user);
        self
    }

    pub fn repository_name(&mut self, name: String) -> &mut Self {
        self.repository_name = Some(name);
        self
    }

    pub fn branch(&mut self, branch: String) -> &mut Self {
        self.branch = Some(branch);
        self
    }

    pub fn repository_path(&mut self, path: String) -> &mut Self {
        self.repository_path = Some(path);
        self
    }

    pub fn repository(&mut self, repository: Repository) -> &mut Self {
        self.repository = Some(repository);
        self
    }

    pub fn write(&mut self, mode: bool) -> &mut Self {
        self.write_mode = mode;
        self
    }

    pub fn force_https(&mut self, mode: bool) -> &mut Self {
        self.write_mode = mode;
        self
    }

    pub fn release(&mut self, mode: bool) -> &mut Self {
        self.release_mode = mode;
        self
    }

    pub fn signature(&mut self, sig: Signature<'static>) -> &mut Self {
        self.signature = Some(sig);
        self
    }

    pub fn gh_token(&mut self, token: String) -> &mut Self {
        self.gh_token = Some(token);
        self
    }

    pub fn cargo_token(&mut self, token: String) -> &mut Self {
        self.cargo_token = Some(token);
        self
    }

    pub fn remote(&mut self, remote: Result<String, String>) -> &mut Self {
        self.remote = Some(remote);
        self
    }

    pub fn asset(&mut self, asset: Asset) -> &mut Self {
        self.assets.push(asset);
        self
    }

    pub fn build(self) -> Config {
        Config {
            user: self.user,
            repository_name: self.repository_name,
            branch: self.branch.unwrap_or("master".into()),
            repository_path: self.repository_path.unwrap(),
            write_mode: self.write_mode,
            release_mode: self.release_mode,
            force_https: self.force_https,
            repository: self.repository.unwrap(),
            signature: self.signature.unwrap(),
            gh_token: self.gh_token,
            cargo_token: self.cargo_token,
            remote: self.remote.unwrap_or(Err("No remote found".into())),
            assets: self.assets,
        }
    }
}

impl Default for ConfigBuilder {
    fn default() -> ConfigBuilder {
        ConfigBuilder::new()
    }
}
