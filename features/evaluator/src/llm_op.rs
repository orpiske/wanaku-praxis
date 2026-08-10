use std::collections::HashMap;

use wanaku_praxis_apis::interactions::Interaction;
use wanaku_praxis_apis::llm::{self, LlmClient};
use wanaku_praxis_apis::registry::ToolEntry;

use crate::config::LlmDef;

/// Execute the LLM operation and return the raw result string.
pub async fn run_llm_operation(
    llm_def: &LlmDef,
    method: &str,
    tool_name: Option<&str>,
    arguments: &HashMap<String, String>,
    tools: &[ToolEntry],
    history: &[Interaction],
) -> Option<String> {
    let client = LlmClient::new(&llm_def.url, &llm_def.model, &llm_def.api_key)?;

    let user_prompt = build_context_prompt(method, tool_name, arguments, tools, history);

    let content = client.chat(&llm_def.prompt, &user_prompt).await?;

    tracing::debug!(llm_response = %content, "evaluator LLM response");

    Some(content)
}

/// Resolve the rule key from an LLM result for classify operations.
/// Tries to extract the classification label from the LLM response.
pub fn extract_classification(content: &str, labels: &[String]) -> String {
    let stripped = llm::strip_markdown_fences(content);

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(stripped) {
        if let Some(level) = parsed.get("level").and_then(serde_json::Value::as_str) {
            let lower = level.trim().to_lowercase();
            if labels.iter().any(|l| l == &lower) {
                return lower;
            }
        }
    }

    let lower = stripped.to_lowercase();
    for label in labels {
        if lower.contains(label.as_str()) {
            return label.clone();
        }
    }

    labels.first().cloned().unwrap_or_default()
}

fn build_context_prompt(
    method: &str,
    tool_name: Option<&str>,
    arguments: &HashMap<String, String>,
    tools: &[ToolEntry],
    history: &[Interaction],
) -> String {
    let mut prompt = String::with_capacity(4096);

    if !history.is_empty() {
        prompt.push_str("## Conversation Context\n\n");
        let capped = if history.len() > 10 {
            &history[history.len() - 10..]
        } else {
            history
        };
        for interaction in capped {
            if let Some(messages) = interaction.request_body.get("messages") {
                if let Some(arr) = messages.as_array() {
                    for msg in arr {
                        let role = msg
                            .get("role")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("unknown");
                        let content = msg
                            .get("content")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("");
                        if !content.is_empty() {
                            prompt.push_str(&format!(
                                "[{role}]: {}\n",
                                llm::sanitize(content, 1000)
                            ));
                        }
                    }
                }
            }
            prompt.push('\n');
        }
    }

    prompt.push_str(&format!("## Request: {method}\n\n"));

    if let Some(name) = tool_name {
        prompt.push_str(&format!("Tool: {name}\n"));
    }

    if !arguments.is_empty() {
        prompt.push_str("Arguments:\n");
        for (key, value) in arguments {
            prompt.push_str(&format!(
                "  {}: {}\n",
                llm::sanitize(key, 500),
                llm::sanitize(value, 500)
            ));
        }
    }

    if !tools.is_empty() {
        prompt.push_str("\n## Available Tools\n\n");
        for tool in tools {
            prompt.push_str(&format!(
                "- {}: {}\n",
                tool.name,
                llm::sanitize(&tool.description, 200)
            ));
        }
    }

    prompt
}
