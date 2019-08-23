use super::ProvisionRequest;
use crate::config::Map;
use crate::plugin_support::PluginStep;
use pest::Parser;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use std::mem;
use std::ops::{Deref, DerefMut};

pub type Key = String;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Value<T> {
    /// Whether user can override this value in releaserc.toml
    #[serde(default)]
    pub protected: bool,
    pub key: Key,
    pub state: ValueState<T>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum ValueState<T> {
    NeedsProvision(ProvisionRequest),
    // Data is available (either provisioned or defined in releaserc.toml)
    Ready(T),
}

impl<T> Value<T> {
    pub fn builder(key: &str) -> ValueBuilder<T> {
        ValueBuilder::new(key)
    }

    pub fn as_value(&self) -> &T {
        match &self.state {
            ValueState::Ready(v) => v,
            ValueState::NeedsProvision(pr) =>
                panic!("Value for key {:?} was requested, but haven't yet been provisioned (request: {:?}). \n \
                        This is a data flow manager bug, please consider opening an issue at https://github.com/etclabscore/semantic-rs/issues/new", self.key, pr),
        }
    }

    #[allow(dead_code)]
    pub fn as_value_mut(&mut self) -> &mut T {
        match &mut self.state {
            ValueState::Ready(v) => v,
            ValueState::NeedsProvision(pr) =>
                panic!("Value for key {:?} was requested, but haven't yet been provisioned (request: {:?}). \n \
                        This is a data flow manager bug, please consider opening an issue at https://github.com/etclabscore/semantic-rs/issues/new", self.key, pr),
        }
    }

    pub fn is_ready(&self) -> bool {
        match &self.state {
            ValueState::Ready(_) => true,
            ValueState::NeedsProvision(_) => false,
        }
    }
}

pub struct ValueBuilder<T> {
    protected: bool,
    key: String,
    value: Option<T>,
    from_env: bool,
    required_at: Option<PluginStep>,
}

impl<T> ValueBuilder<T> {
    pub fn new(key: &str) -> Self {
        ValueBuilder {
            protected: false,
            key: key.to_owned(),
            value: None,
            from_env: false,
            required_at: None,
        }
    }

    pub fn protected(&mut self) -> &mut Self {
        self.protected = true;
        self
    }

    pub fn value(&mut self, value: T) -> &mut Self {
        self.value = Some(value);
        self
    }

    pub fn required_at(&mut self, step: PluginStep) -> &mut Self {
        self.required_at = Some(step);
        self
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn load_from_env(&mut self) -> &mut Self {
        self.from_env = true;
        self
    }

    pub fn build(&mut self) -> Value<T> {
        let key = mem::replace(&mut self.key, String::new());

        if let Some(value) = self.value.take() {
            Value {
                protected: self.protected,
                key,
                state: ValueState::Ready(value),
            }
        } else {
            Value {
                protected: self.protected,
                key: key.clone(),
                state: ValueState::NeedsProvision(ProvisionRequest {
                    required_at: self.required_at.take(),
                    from_env: self.from_env,
                    key,
                }),
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ValueDefinitionMap(Map<String, ValueDefinition>);

impl Deref for ValueDefinitionMap {
    type Target = Map<String, ValueDefinition>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ValueDefinitionMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Into<Map<String, Value<serde_json::Value>>> for ValueDefinitionMap {
    fn into(self) -> Map<String, Value<serde_json::Value>> {
        let mut map = Map::new();
        for (key, value) in self.0 {
            let kv = match value {
                ValueDefinition::Value(v) => Value::builder(&key).value(v).build(),
                ValueDefinition::From {
                    required_at,
                    from_env,
                    key,
                } => {
                    let mut kv = Value::builder(&key);
                    if let Some(step) = required_at {
                        kv.required_at(step);
                    }
                    if from_env {
                        kv.load_from_env();
                    }
                    kv.build()
                }
            };
            map.insert(key, kv);
        }
        map
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueDefinition {
    From {
        required_at: Option<PluginStep>,
        from_env: bool,
        key: String,
    },
    Value(serde_json::Value),
}

impl ValueDefinition {
    pub fn is_value(&self) -> bool {
        match self {
            ValueDefinition::Value(_) => true,
            ValueDefinition::From { .. } => false,
        }
    }

    pub fn as_value(&self) -> &serde_json::Value {
        match self {
            ValueDefinition::Value(v) => &v,
            ValueDefinition::From { .. } => panic!("ValueDefinition is not in Value state."),
        }
    }
}

impl<'de> Deserialize<'de> for ValueDefinitionMap {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_map: Map<String, serde_json::Value> = Deserialize::deserialize(de)?;
        let mut map = Map::new();

        for (key, value) in raw_map {
            if let Some(value) = value.as_str() {
                let parsed = parse_value_definition(value).map_err(D::Error::custom)?;
                map.insert(key, parsed);
            } else {
                map.insert(key, ValueDefinition::Value(value));
            }
        }

        Ok(ValueDefinitionMap(map))
    }
}

#[derive(Parser)]
#[grammar = "../grammar/dataflow.pest"]
struct ValueDefinitionParser;

fn parse_value_definition(value: &str) -> Result<ValueDefinition, failure::Error> {
    use std::str::FromStr;

    let pairs = ValueDefinitionParser::parse(Rule::value_def, value)
        .map_err(|e| failure::err_msg(format!("{}", e)))?
        .next()
        .unwrap();

    let mut required_at = None;
    let mut from_env = false;
    let mut key = String::new();

    for pair in pairs.into_inner() {
        log::trace!("{:#?}", pair);
        match pair.as_rule() {
            Rule::value => return Ok(ValueDefinition::Value(serde_json::Value::String(pair.as_str().into()))),
            Rule::required_at_step => {
                required_at = Some(PluginStep::from_str(pair.as_str())?);
            }
            Rule::from_env => {
                from_env = true;
            }
            Rule::key => {
                key = pair.as_str().into();
            }
            _ => (),
        }
    }

    Ok(ValueDefinition::From {
        required_at,
        from_env,
        key,
    })
}

impl<T: Default> ValueBuilder<T> {
    pub fn default_value(&mut self) -> &mut Self {
        self.value = Some(Default::default());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Display;

    #[test]
    fn build() {
        let kv: Value<()> = Value::builder("key").build();
        assert_eq!(kv.protected, false);
        assert_eq!(kv.key, "key");
        assert_eq!(
            kv.state,
            ValueState::NeedsProvision(ProvisionRequest {
                required_at: None,
                from_env: false,
                key: "key".to_string()
            })
        );
    }

    #[test]
    fn build_protected() {
        let kv: Value<()> = Value::builder("key").protected().build();
        assert_eq!(kv.protected, true);
        assert_eq!(kv.key, "key");
        assert_eq!(
            kv.state,
            ValueState::NeedsProvision(ProvisionRequest {
                required_at: None,
                from_env: false,
                key: "key".to_string()
            })
        );
    }

    #[test]
    fn build_required_at() {
        let kv: Value<()> = Value::builder("key").required_at(PluginStep::Commit).build();
        assert_eq!(kv.protected, false);
        assert_eq!(kv.key, "key");
        assert_eq!(
            kv.state,
            ValueState::NeedsProvision(ProvisionRequest {
                required_at: Some(PluginStep::Commit),
                from_env: false,
                key: "key".to_string()
            })
        );
    }

    #[test]
    fn build_ready_default_value() {
        let kv: Value<bool> = Value::builder("key").default_value().build();
        assert_eq!(kv.protected, false);
        assert_eq!(kv.key, "key");
        assert_eq!(kv.state, ValueState::Ready(false));
    }

    #[test]
    fn build_ready_custom_value() {
        let kv = Value::builder("key").value("value").build();
        assert_eq!(kv.protected, false);
        assert_eq!(kv.key, "key");
        assert_eq!(kv.state, ValueState::Ready("value"));
    }

    #[test]
    fn build_from_env() {
        let kv: Value<()> = Value::builder("key").load_from_env().build();
        assert_eq!(kv.protected, false);
        assert_eq!(kv.key, "key");
        assert_eq!(
            kv.state,
            ValueState::NeedsProvision(ProvisionRequest {
                required_at: None,
                from_env: true,
                key: "key".to_string()
            })
        );
    }

    #[test]
    fn as_value() {
        let kv = Value::builder("key").value("value").build();
        kv.as_value();
    }

    #[test]
    #[should_panic]
    fn as_value_needs_provision() {
        let kv: Value<()> = Value::builder("key").build();
        kv.as_value();
    }

    #[test]
    fn as_value_mut() {
        let mut kv = Value::builder("key").value("value").build();
        kv.as_value_mut();
    }

    #[test]
    #[should_panic]
    fn as_value_mut_needs_provision() {
        let mut kv: Value<()> = Value::builder("key").build();
        kv.as_value_mut();
    }

    #[test]
    fn serialize_deserialize_ready() {
        let kv = Value {
            protected: false,
            key: "key".into(),
            state: ValueState::Ready("value"),
        };

        let serialized = serde_json::to_string(&kv).unwrap();
        let deserialized: Value<&str> = serde_json::from_str(&serialized).unwrap();

        assert_eq!(kv, deserialized);
    }

    fn pretty_print_error_and_panic(err: impl Display) {
        eprintln!("{}", err);
        panic!("test failed");
    }

    #[test]
    fn parse_value_definition_value() {
        let v: ValueDefinition = parse_value_definition(r#"false"#)
            .map_err(pretty_print_error_and_panic)
            .unwrap();

        assert_eq!(v, ValueDefinition::Value(serde_json::Value::String("false".into())));
    }

    #[test]
    fn parse_value_definition_from_key() {
        let v: ValueDefinition = parse_value_definition(r#"from:key"#)
            .map_err(pretty_print_error_and_panic)
            .unwrap();

        assert_eq!(
            v,
            ValueDefinition::From {
                required_at: None,
                from_env: false,
                key: "key".into()
            }
        );
    }

    #[test]
    fn parse_value_definition_from_env() {
        let v: ValueDefinition = parse_value_definition(r#"from:env:key"#)
            .map_err(pretty_print_error_and_panic)
            .unwrap();

        assert_eq!(
            v,
            ValueDefinition::From {
                required_at: None,
                from_env: true,
                key: "key".into()
            }
        );
    }

    #[test]
    fn parse_value_definition_from_env_required_at() {
        let v: ValueDefinition = parse_value_definition(r#"from:env:required_at=commit:key"#)
            .map_err(pretty_print_error_and_panic)
            .unwrap();

        assert_eq!(
            v,
            ValueDefinition::From {
                required_at: Some(PluginStep::Commit),
                from_env: true,
                key: "key".into()
            }
        );
    }

    #[test]
    fn parse_value_definition_from_full() {
        let v: ValueDefinition = parse_value_definition(r#"from:required_at=commit:key"#)
            .map_err(pretty_print_error_and_panic)
            .unwrap();

        assert_eq!(
            v,
            ValueDefinition::From {
                required_at: Some(PluginStep::Commit),
                from_env: false,
                key: "key".into()
            }
        );
    }

    #[test]
    #[should_panic]
    fn parse_value_definition_unknown_meta_keys() {
        let _v: ValueDefinition = parse_value_definition(r#"from:required_at=commit:unknown_meta:key"#)
            .map_err(pretty_print_error_and_panic)
            .unwrap();
    }

    #[test]
    fn deserialize_value_definition_string() {
        let toml = r#"key = "false""#;
        let kvmap: ValueDefinitionMap = toml::from_str(toml).unwrap();
        assert_eq!(kvmap.0.len(), 1);
        let v = kvmap.0.values().next().unwrap();

        assert_eq!(v, &ValueDefinition::Value(serde_json::Value::String("false".into())));
    }

    #[test]
    fn deserialize_value_definition_not_string() {
        let toml = r#"key = false"#;
        let kvmap: ValueDefinitionMap = toml::from_str(toml).unwrap();
        assert_eq!(kvmap.0.len(), 1);
        let v = kvmap.0.values().next().unwrap();

        assert_eq!(v, &ValueDefinition::Value(serde_json::Value::Bool(false)));
    }

    #[test]
    fn deserialize_value_definition_complex_value() {
        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
        struct Value {
            one: i32,
            two: bool,
            three: String,
            four: Vec<u32>,
        }

        let value = Value {
            one: 1,
            two: true,
            three: "three".to_owned(),
            four: vec![1, 2, 3, 4],
        };

        let value_toml = r#"key = { one = 1, two = true, three = "three", four = [1, 2, 3, 4] }"#;

        let kvmap: ValueDefinitionMap = toml::from_str(value_toml).unwrap();
        assert_eq!(kvmap.0.len(), 1);
        let v = kvmap.0.values().next().unwrap();

        let parsed: Value = match v {
            ValueDefinition::From { .. } => panic!("expected Value, got From"),
            ValueDefinition::Value(value) => serde_json::from_value(value.clone()).unwrap(),
        };

        assert_eq!(value, parsed);
    }
}
