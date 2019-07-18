use failure::Fail;

use crate::plugin::{
    Plugin, PluginInterface, PluginName, PluginState, ResolvedPlugin, UnresolvedPlugin,
};

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

    pub fn resolve(&self, plugin: Plugin) -> Result<Plugin, failure::Error> {
        if plugin.state.is_resolved() {
            return Ok(plugin);
        }

        let (name, state) = plugin.decompose();
        let meta = state.as_unresolved().unwrap();

        let new_meta = match meta {
            UnresolvedPlugin::Builtin => self.builtin.resolve(&name, &meta)?,
            UnresolvedPlugin::Cargo { .. } => self.cargo.resolve(&name, &meta)?,
        };

        Ok(Plugin::new(name, PluginState::Resolved(new_meta)))
    }
}

trait Resolver {
    fn resolve(
        &self,
        name: &PluginName,
        meta: &UnresolvedPlugin,
    ) -> Result<ResolvedPlugin, failure::Error>;
}

struct BuiltinResolver;

impl BuiltinResolver {
    pub fn new() -> Self {
        BuiltinResolver
    }
}

impl Resolver for BuiltinResolver {
    fn resolve(
        &self,
        name: &PluginName,
        _meta: &UnresolvedPlugin,
    ) -> Result<ResolvedPlugin, failure::Error> {
        use crate::builtin_plugins::{ClogPlugin, GitPlugin, GithubPlugin, RustPlugin};
        let plugin: Box<dyn PluginInterface> = match name.as_str() {
            "git" => Box::new(GitPlugin::new()),
            "github" => Box::new(GithubPlugin::new()),
            "clog" => Box::new(ClogPlugin::new()),
            "rust" => Box::new(RustPlugin::new()),
            other => Err(ResolverError::BuiltinNotRegistered(other.to_string()))?,
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
    fn resolve(
        &self,
        name: &PluginName,
        meta: &UnresolvedPlugin,
    ) -> Result<ResolvedPlugin, failure::Error> {
        unimplemented!()
    }
}

#[derive(Fail, Debug)]
pub enum ResolverError {
    #[fail(display = "{} is not registered as built-in plugin", _0)]
    BuiltinNotRegistered(String),
}
