use failure::Fail;

use crate::plugin_support::{PluginInterface, RawPlugin, RawPluginState, ResolvedPlugin, UnresolvedPlugin};

pub struct PluginResolver {
    builtin: BuiltinResolver,
    cargo: CargoResolver,
}

impl PluginResolver {
    pub fn new() -> Self {
        PluginResolver {
            builtin: BuiltinResolver::new(),
            cargo: CargoResolver::new(),
        }
    }

    pub fn resolve(&self, plugin: RawPlugin) -> Result<RawPlugin, failure::Error> {
        if plugin.state().is_resolved() {
            return Ok(plugin);
        }

        let (name, state) = plugin.decompose();
        let meta = state.as_unresolved().unwrap();

        let new_meta = match meta {
            UnresolvedPlugin::Builtin => self.builtin.resolve(&name, &meta)?,
            UnresolvedPlugin::Cargo { .. } => self.cargo.resolve(&name, &meta)?,
        };

        Ok(RawPlugin::new(name, RawPluginState::Resolved(new_meta)))
    }
}

trait Resolver {
    fn resolve(&self, name: &str, meta: &UnresolvedPlugin) -> Result<ResolvedPlugin, failure::Error>;
}

struct BuiltinResolver;

impl BuiltinResolver {
    pub fn new() -> Self {
        BuiltinResolver
    }
}

impl Resolver for BuiltinResolver {
    fn resolve(&self, name: &str, _meta: &UnresolvedPlugin) -> Result<ResolvedPlugin, failure::Error> {
        use crate::builtin_plugins::{ClogPlugin, DockerPlugin, GitPlugin, GithubPlugin, RustPlugin};
        let plugin: Box<dyn PluginInterface> = match name {
            "git" => Box::new(GitPlugin::new()),
            "clog" => Box::new(ClogPlugin::new()),
            "github" => Box::new(GithubPlugin::new()),
            "rust" => Box::new(RustPlugin::new()),
            "docker" => Box::new(DockerPlugin::new()),
            other => return Err(ResolverError::BuiltinNotRegistered(other.to_string()).into()),
        };
        Ok(ResolvedPlugin::Builtin(plugin))
    }
}

struct CargoResolver;

impl CargoResolver {
    pub fn new() -> CargoResolver {
        CargoResolver
    }
}

impl Resolver for CargoResolver {
    fn resolve(&self, _name: &str, _meta: &UnresolvedPlugin) -> Result<ResolvedPlugin, failure::Error> {
        unimplemented!()
    }
}

#[derive(Fail, Debug)]
pub enum ResolverError {
    #[fail(display = "{} is not registered as built-in plugin", _0)]
    BuiltinNotRegistered(String),
}
