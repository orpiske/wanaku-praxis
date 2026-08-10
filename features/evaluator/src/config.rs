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

/// Reference to an action: either a WASM file path or "pass" (no-op).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActionRef {
    Pass(PassAction),
    Wasm(WasmAction),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassAction {
    // Matches the literal string "pass" in YAML
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmAction {
    pub path: PathBuf,
}

impl ActionRef {
    #[must_use]
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass(_))
    }

    pub fn parse(s: &str) -> Self {
        if s == "pass" {
            Self::Pass(PassAction {})
        } else {
            Self::Wasm(WasmAction {
                path: PathBuf::from(s),
            })
        }
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
