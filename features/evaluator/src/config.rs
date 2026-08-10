use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Top-level evaluator configuration containing multiple evaluator definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatorsConfig {
    #[serde(default)]
    pub evaluators: Vec<EvaluatorDef>,
}

/// A single evaluator definition: trigger + LLM operation + rules + action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatorDef {
    pub name: String,
    pub trigger: TriggerDef,
    pub llm: LlmDef,
    #[serde(default)]
    pub rules: HashMap<String, ActionRef>,
    #[serde(default)]
    pub action: Option<ActionRef>,
    #[serde(default = "default_on_error")]
    pub on_error: ErrorPolicy,
}

/// What triggers this evaluator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDef {
    pub method: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub binding: Option<String>,
}

/// LLM operation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmDef {
    pub operation: LlmOperation,
    pub prompt: String,
    pub model: String,
    pub url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub labels: Vec<String>,
}

/// The type of cognitive operation the LLM performs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmOperation {
    Classify,
    Filter,
    Augment,
}

/// Reference to an action: either "pass" (no-op) or a WASM file path.
/// Accepts both string `"pass"` and object `{"path": "/path/to/action.wasm"}`.
#[derive(Debug, Clone)]
pub enum ActionRef {
    Pass,
    Wasm { path: PathBuf },
}

impl ActionRef {
    #[must_use]
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }

    pub fn parse(s: &str) -> Self {
        if s == "pass" {
            Self::Pass
        } else {
            Self::Wasm {
                path: PathBuf::from(s),
            }
        }
    }
}

impl Serialize for ActionRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Pass => serializer.serialize_str("pass"),
            Self::Wasm { path } => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("path", path)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ActionRef {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de;

        struct ActionRefVisitor;

        impl<'de> de::Visitor<'de> for ActionRefVisitor {
            type Value = ActionRef;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(r#""pass" or {"path": "/path/to/action.wasm"}"#)
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<ActionRef, E> {
                Ok(ActionRef::parse(value))
            }

            fn visit_map<M: de::MapAccess<'de>>(self, mut map: M) -> Result<ActionRef, M::Error> {
                let mut path: Option<PathBuf> = None;
                while let Some(key) = map.next_key::<String>()? {
                    if key == "path" {
                        path = Some(map.next_value()?);
                    } else {
                        let _: serde::de::IgnoredAny = map.next_value()?;
                    }
                }
                match path {
                    Some(p) => Ok(ActionRef::Wasm { path: p }),
                    None => Err(de::Error::missing_field("path")),
                }
            }
        }

        deserializer.deserialize_any(ActionRefVisitor)
    }
}

/// What to do when a WASM action fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorPolicy {
    Continue,
    Block,
}

fn default_on_error() -> ErrorPolicy {
    ErrorPolicy::Continue
}

impl TriggerDef {
    pub fn matches(&self, method: &str, namespace: &str) -> bool {
        if self.method != method {
            return false;
        }
        if let Some(ref ns) = self.namespace {
            if ns != namespace {
                return false;
            }
        }
        true
    }
}
