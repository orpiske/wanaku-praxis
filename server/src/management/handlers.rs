use http::Response;
use tracing::{info, warn};

use wanaku_praxis_apis::registry::{
    ForwardEntry, ForwardRegistry, InMemoryRegistry, NamespaceEntry, NamespaceRegistry,
    PromptEntry, PromptRegistry, ResourceEntry, ResourceRegistry, ServiceEntry, ServiceRegistry,
    ToolEntry, ToolRegistry, MCP_FORWARD_TYPE,
};
use wanaku_praxis_apis::safety::{SafetyConfig, SafetyState};

use super::response::{json_ok, json_err, raw_json_response};

pub(super) fn handle_tool_list(registry: &InMemoryRegistry) -> Response<Vec<u8>> {
    let tools = registry.list_tools();
    json_ok(&serde_json::json!(tools))
}

pub(super) fn handle_tool_get(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    match registry.get_tool(name) {
        Some(tool) => json_ok(&serde_json::json!(tool)),
        None => json_err(404, &format!("tool not found: {name}")),
    }
}

pub(super) fn handle_tool_create(registry: &InMemoryRegistry, body: &str) -> Response<Vec<u8>> {
    tracing::debug!(body = %body, "tool create request body");
    let tool: ToolEntry = match serde_json::from_str(body) {
        Ok(t) => t,
        Err(e) => {
            warn!(error = %e, "invalid tool JSON");
            return json_err(400, &format!("invalid tool JSON: {e}"));
        }
    };

    let name = tool.name.clone();
    registry.register_tool(tool);
    info!(tool = %name, "registered tool via management API");
    match registry.get_tool(&name) {
        Some(entry) => json_ok(&serde_json::json!(entry)),
        None => json_err(404, &format!("tool not found after registration: {name}")),
    }
}

pub(super) fn handle_tool_delete(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    if registry.remove_tool(name) {
        info!(tool = %name, "removed tool via management API");
        json_ok(&serde_json::json!({"removed": name}))
    } else {
        json_err(404, &format!("tool not found: {name}"))
    }
}

pub(super) fn handle_resource_list(registry: &InMemoryRegistry) -> Response<Vec<u8>> {
    let resources = registry.list_resources();
    json_ok(&serde_json::json!(resources))
}

pub(super) fn handle_resource_get(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    match registry.get_resource(name) {
        Some(resource) => json_ok(&serde_json::json!(resource)),
        None => json_err(404, &format!("resource not found: {name}")),
    }
}

pub(super) fn handle_resource_create(registry: &InMemoryRegistry, body: &str) -> Response<Vec<u8>> {
    tracing::debug!(body = %body, "resource create request body");
    let resource: ResourceEntry = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "invalid resource JSON");
            return json_err(400, &format!("invalid resource JSON: {e}"));
        }
    };

    let name = resource.name.clone();
    registry.register_resource(resource);
    info!(resource = %name, "registered resource via management API");
    match registry.get_resource(&name) {
        Some(entry) => json_ok(&serde_json::json!(entry)),
        None => json_err(404, &format!("resource not found after registration: {name}")),
    }
}

pub(super) fn handle_resource_delete(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    if registry.remove_resource(name) {
        info!(resource = %name, "removed resource via management API");
        json_ok(&serde_json::json!({"removed": name}))
    } else {
        json_err(404, &format!("resource not found: {name}"))
    }
}

pub(super) fn handle_prompt_list(registry: &InMemoryRegistry) -> Response<Vec<u8>> {
    let prompts = registry.list_prompts();
    json_ok(&serde_json::json!(prompts))
}

pub(super) fn handle_prompt_get(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    match registry.get_prompt(name) {
        Some(prompt) => json_ok(&serde_json::json!(prompt)),
        None => json_err(404, &format!("prompt not found: {name}")),
    }
}

pub(super) fn handle_prompt_create(registry: &InMemoryRegistry, body: &str) -> Response<Vec<u8>> {
    tracing::debug!(body = %body, "prompt create request body");
    let prompt: PromptEntry = match serde_json::from_str(body) {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "invalid prompt JSON");
            return json_err(400, &format!("invalid prompt JSON: {e}"));
        }
    };

    let name = prompt.name.clone();
    registry.register_prompt(prompt);
    info!(prompt = %name, "registered prompt via management API");
    match registry.get_prompt(&name) {
        Some(entry) => json_ok(&serde_json::json!(entry)),
        None => json_err(404, &format!("prompt not found after registration: {name}")),
    }
}

pub(super) fn handle_prompt_delete(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    if registry.remove_prompt(name) {
        info!(prompt = %name, "removed prompt via management API");
        json_ok(&serde_json::json!({"removed": name}))
    } else {
        json_err(404, &format!("prompt not found: {name}"))
    }
}

pub(super) fn handle_namespace_list(registry: &InMemoryRegistry) -> Response<Vec<u8>> {
    let namespaces = registry.list_namespaces();
    json_ok(&serde_json::json!(namespaces))
}

pub(super) fn handle_namespace_get(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    match registry.get_namespace(name) {
        Some(ns) => json_ok(&serde_json::json!(ns)),
        None => json_err(404, &format!("namespace not found: {name}")),
    }
}

pub(super) fn handle_namespace_create(registry: &InMemoryRegistry, body: &str) -> Response<Vec<u8>> {
    let namespace: NamespaceEntry = match serde_json::from_str(body) {
        Ok(n) => n,
        Err(e) => {
            warn!(error = %e, "invalid namespace JSON");
            return json_err(400, &format!("invalid namespace JSON: {e}"));
        }
    };

    let name = namespace.name.clone();
    registry.register_namespace(namespace);
    info!(namespace = %name, "registered namespace via management API");
    match registry.get_namespace(&name) {
        Some(entry) => json_ok(&serde_json::json!(entry)),
        None => json_err(404, &format!("namespace not found after registration: {name}")),
    }
}

pub(super) fn handle_namespace_update(registry: &InMemoryRegistry, path_name: &str, body: &str) -> Response<Vec<u8>> {
    let mut namespace: NamespaceEntry = match serde_json::from_str(body) {
        Ok(n) => n,
        Err(e) => {
            warn!(error = %e, "invalid namespace JSON");
            return json_err(400, &format!("invalid namespace JSON: {e}"));
        }
    };

    namespace.name = path_name.to_owned();
    namespace.id = None;
    registry.register_namespace(namespace);
    info!(namespace = %path_name, "updated namespace via management API");
    match registry.get_namespace(path_name) {
        Some(entry) => json_ok(&serde_json::json!(entry)),
        None => json_err(404, &format!("namespace not found after update: {path_name}")),
    }
}

pub(super) fn handle_namespace_delete(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    if registry.remove_namespace(name) {
        info!(namespace = %name, "removed namespace via management API");
        json_ok(&serde_json::json!({"removed": name}))
    } else {
        json_err(404, &format!("namespace not found: {name}"))
    }
}

pub(super) fn handle_service_list(registry: &InMemoryRegistry) -> Response<Vec<u8>> {
    let services = registry.list_services();
    json_ok(&serde_json::json!(services))
}

pub(super) fn handle_service_get(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    let services: Vec<ServiceEntry> = registry
        .list_services()
        .into_iter()
        .filter(|s| s.name == name)
        .collect();

    if services.is_empty() {
        json_err(404, &format!("service not found: {name}"))
    } else {
        json_ok(&serde_json::json!(services))
    }
}

pub(super) fn handle_service_create(registry: &InMemoryRegistry, body: &str) -> Response<Vec<u8>> {
    tracing::debug!(body = %body, "service create request body");
    let service: ServiceEntry = match serde_json::from_str(body) {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "invalid service JSON");
            return json_err(400, &format!("invalid service JSON: {e}"));
        }
    };

    let name = service.name.clone();
    let svc_type = service.service_type.clone();
    registry.register_service(service);
    info!(service = %name, service_type = %svc_type, "registered service via management API");
    match registry.get_service(&name, &svc_type) {
        Some(entry) => json_ok(&serde_json::json!(entry)),
        None => json_err(404, &format!("service not found after registration: {name}")),
    }
}

pub(super) fn handle_service_delete(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    let services: Vec<ServiceEntry> = registry
        .list_services()
        .into_iter()
        .filter(|s| s.name == name)
        .collect();

    if services.is_empty() {
        return json_err(404, &format!("service not found: {name}"));
    }

    let mut removed_count = 0;
    for svc in &services {
        if registry.remove_service(&svc.name, &svc.service_type) {
            removed_count += 1;
        }
    }

    info!(service = %name, count = removed_count, "removed service(s) via management API");
    json_ok(&serde_json::json!({"removed": name, "count": removed_count}))
}

pub(super) fn handle_forward_list(registry: &InMemoryRegistry) -> Response<Vec<u8>> {
    let forwards = registry.list_forwards();
    json_ok(&serde_json::json!(forwards))
}

pub(super) fn handle_forward_get(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    match registry.get_forward(name) {
        Some(forward) => json_ok(&serde_json::json!(forward)),
        None => json_err(404, &format!("forward not found: {name}")),
    }
}

pub(super) async fn handle_forward_create(registry: &InMemoryRegistry, body: &str) -> Response<Vec<u8>> {
    tracing::debug!(body = %body, "forward create request body");
    let forward: ForwardEntry = match serde_json::from_str(body) {
        Ok(f) => f,
        Err(e) => {
            warn!(error = %e, "invalid forward JSON");
            return json_err(400, &format!("invalid forward JSON: {e}"));
        }
    };

    info!(forward = %forward.name, address = %forward.address, "registered forward via management API");
    registry.register_forward(forward.clone());

    let count = discover_tools_from_forward(registry, &forward).await;

    json_ok(&serde_json::json!({
        "forward": &forward,
        "tools_discovered": count,
    }))
}

pub(super) fn handle_forward_delete(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    let forward = registry.get_forward(name);

    if !registry.remove_forward(name) {
        return json_err(404, &format!("forward not found: {name}"));
    }

    if let Some(fwd) = forward {
        remove_forwarded_tools(registry, &fwd.address);
    }

    info!(forward = %name, "removed forward via management API");
    json_ok(&serde_json::json!({"removed": name}))
}

pub(super) async fn handle_forward_refresh(registry: &InMemoryRegistry, name: &str) -> Response<Vec<u8>> {
    let forward = match registry.get_forward(name) {
        Some(f) => f,
        None => return json_err(404, &format!("forward not found: {name}")),
    };

    remove_forwarded_tools(registry, &forward.address);
    let count = discover_tools_from_forward(registry, &forward).await;

    info!(forward = %name, tools_discovered = count, "refreshed forward");
    json_ok(&serde_json::json!({"refreshed": name, "tools_discovered": count}))
}

pub async fn discover_tools_from_forward(registry: &InMemoryRegistry, forward: &ForwardEntry) -> usize {
    let tools = match wanaku_praxis_apis::mcp_client::list_tools(&forward.address).await {
        Ok(t) => t,
        Err(e) => {
            warn!(forward = %forward.name, error = %e, "failed to discover tools from forward");
            return 0;
        }
    };

    let namespace = forward.namespace.as_deref().unwrap_or(wanaku_praxis_apis::registry::DEFAULT_NAMESPACE);
    let mut count = 0;

    for tool_json in &tools {
        let name = match tool_json.get("name").and_then(|n| n.as_str()).map(str::trim) {
            Some(n) if !n.is_empty() => n,
            _ => {
                warn!(forward = %forward.name, "skipping forwarded tool with missing or empty name");
                continue;
            }
        };
        let description = tool_json
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or_default();
        let input_schema = tool_json
            .get("inputSchema")
            .cloned()
            .unwrap_or(serde_json::json!({"type": "object"}));

        let tool = ToolEntry {
            name: name.to_owned(),
            description: description.to_owned(),
            uri: forward.address.clone(),
            type_: MCP_FORWARD_TYPE.to_owned(),
            input_schema,
            labels: std::collections::HashMap::new(),
            id: None,
            namespace: Some(namespace.to_owned()),
            configuration_uri: None,
            secrets_uri: None,
            skip_safety_check: false,
        };

        info!(tool = %name, forward = %forward.name, "discovered forwarded tool");
        registry.register_tool(tool);
        count += 1;
    }

    count
}

fn remove_forwarded_tools(registry: &InMemoryRegistry, address: &str) {
    let forwarded: Vec<String> = registry
        .list_tools()
        .iter()
        .filter(|t| t.is_mcp_forward() && t.uri == address)
        .map(|t| t.name.clone())
        .collect();

    registry.remove_tools_batch(&forwarded);
}

pub(super) fn handle_capability_list(registry: &InMemoryRegistry) -> Response<Vec<u8>> {
    let services = registry.list_services();
    let targets: Vec<serde_json::Value> = services
        .iter()
        .map(|s| {
            let (host, port) = s
                .address
                .rsplit_once(':')
                .map(|(h, p)| (h.to_owned(), p.parse::<u16>().unwrap_or(0)))
                .unwrap_or_else(|| (s.address.clone(), 0));

            serde_json::json!({
                "id": format!("{}:{}", s.name, s.service_type),
                "serviceName": s.name,
                "host": host,
                "port": port,
                "serviceType": s.service_type,
            })
        })
        .collect();
    json_ok(&serde_json::json!(targets))
}

pub(super) fn handle_capability_state() -> Response<Vec<u8>> {
    let empty: std::collections::HashMap<String, Vec<serde_json::Value>> =
        std::collections::HashMap::new();
    json_ok(&serde_json::json!(empty))
}

pub(super) fn handle_statistics(registry: &InMemoryRegistry) -> Response<Vec<u8>> {
    let tools_count = registry.tool_count() as i64;
    let resources_count = registry.resource_count() as i64;
    let prompts_count = registry.prompt_count() as i64;
    let forwards_count = registry.list_forwards().len() as i64;

    json_ok(&serde_json::json!({
        "toolsCount": tools_count,
        "resourcesCount": resources_count,
        "promptsCount": prompts_count,
        "forwardsCount": forwards_count,
        "dataStoresCount": 0,
        "toolCapabilities": {
            "total": 0,
            "healthy": 0,
            "unhealthy": 0,
            "down": 0,
            "pending": 0
        },
        "resourceCapabilities": {
            "total": 0,
            "healthy": 0,
            "unhealthy": 0,
            "down": 0,
            "pending": 0
        }
    }))
}

pub(super) fn handle_safety_get(state: &SafetyState) -> Response<Vec<u8>> {
    json_ok(&serde_json::json!(state.current_config()))
}

pub(super) fn handle_safety_update(state: &SafetyState, body: &str) -> Response<Vec<u8>> {
    let config: SafetyConfig = match serde_json::from_str(body) {
        Ok(c) => c,
        Err(e) => return json_err(400, &format!("invalid safety config: {e}")),
    };

    info!(model = %config.llm_model, url = %config.llm_url, "safety classifier updated via management API");
    state.configure(config.clone());

    json_ok(&serde_json::json!(config))
}

pub(super) fn handle_safety_delete(state: &SafetyState) -> Response<Vec<u8>> {
    state.disable();
    info!("safety classifier disabled via management API");
    json_ok(&serde_json::Value::Null)
}

pub(super) fn handle_chat_list_llms() -> Response<Vec<u8>> {
    raw_json_response(serde_json::to_vec(&serde_json::json!(["Ollama"])).unwrap_or_default())
}

pub(super) async fn handle_chat_list_models(ollama_proxy: &str) -> Response<Vec<u8>> {
    let url = format!("{ollama_proxy}/v1/models");
    let client = reqwest::Client::new();

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "failed to fetch models from Ollama proxy");
            return json_err(502, &format!("failed to reach Ollama: {e}"));
        }
    };

    let body: serde_json::Value = match response.json().await {
        Ok(b) => b,
        Err(e) => {
            warn!(error = %e, "failed to parse Ollama models response");
            return json_err(502, &format!("invalid response from Ollama: {e}"));
        }
    };

    let models: Vec<String> = body
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(serde_json::Value::as_str))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    raw_json_response(serde_json::to_vec(&serde_json::json!(models)).unwrap_or_default())
}

pub(super) async fn handle_chat_completions(ollama_proxy: &str, body: &str) -> Response<Vec<u8>> {
    let request: serde_json::Value = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => return json_err(400, &format!("invalid request: {e}")),
    };

    let model = request.get("model").and_then(serde_json::Value::as_str).unwrap_or("");
    let system_prompt = request.get("systemPrompt").and_then(serde_json::Value::as_str).unwrap_or("");
    let user_prompt = request.get("userPrompt").and_then(serde_json::Value::as_str).unwrap_or("");

    let mut messages = Vec::new();

    if !system_prompt.is_empty() {
        messages.push(serde_json::json!({"role": "system", "content": system_prompt}));
    }

    if let Some(history) = request.get("chatHistory").and_then(|h| h.as_array()) {
        for msg in history {
            messages.push(msg.clone());
        }
    }

    messages.push(serde_json::json!({"role": "user", "content": user_prompt}));

    let openai_request = serde_json::json!({
        "model": model,
        "messages": messages,
    });

    let url = format!("{ollama_proxy}/v1/chat/completions");
    let client = reqwest::Client::new();

    let response = match client.post(&url)
        .json(&openai_request)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "chat completions request to Ollama proxy failed");
            return json_err(502, &format!("failed to reach Ollama: {e}"));
        }
    };

    let body: serde_json::Value = match response.json().await {
        Ok(b) => b,
        Err(e) => {
            warn!(error = %e, "failed to parse Ollama completions response");
            return json_err(502, &format!("invalid response from Ollama: {e}"));
        }
    };

    let content = body
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");

    let response_body = content.as_bytes().to_vec();
    Response::builder()
        .status(200)
        .header("Content-Type", "text/plain")
        .header("Content-Length", response_body.len())
        .body(response_body)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wanaku_praxis_apis::registry::InMemoryRegistry;
    use wanaku_praxis_apis::safety::SafetyState;

    fn status_of(resp: &Response<Vec<u8>>) -> u16 {
        resp.status().as_u16()
    }

    fn response_data(resp: &Response<Vec<u8>>) -> serde_json::Value {
        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap_or_default();
        body.get("data").cloned().unwrap_or(serde_json::Value::Null)
    }

    fn response_error(resp: &Response<Vec<u8>>) -> Option<String> {
        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap_or_default();
        body.get("error")
            .and_then(serde_json::Value::as_str)
            .map(String::from)
    }

    const TOOL_JSON: &str = r#"{
        "name": "test-tool",
        "description": "A test tool",
        "uri": "echo-tool://echo",
        "type": "echo-tool",
        "input_schema": {"type": "object", "properties": {"message": {"type": "string"}}}
    }"#;

    const RESOURCE_JSON: &str = r#"{
        "name": "test-resource",
        "description": "A test resource",
        "location": "/tmp/test.txt",
        "type": "file",
        "mime_type": "text/plain"
    }"#;

    const PROMPT_JSON: &str = r#"{
        "name": "test-prompt",
        "description": "A test prompt",
        "arguments": [],
        "messages": []
    }"#;

    const NAMESPACE_JSON: &str = r#"{
        "name": "test-ns",
        "path": "/test-ns/mcp"
    }"#;

    const SERVICE_JSON: &str = r#"{
        "name": "echo-tool",
        "address": "localhost:9191",
        "service_type": "tool-invoker"
    }"#;

    // ── Tool handler tests ──────────────────────────────────────────

    #[test]
    fn tool_create_and_list() {
        let registry = InMemoryRegistry::new();
        let resp = handle_tool_create(&registry, TOOL_JSON);
        assert_eq!(status_of(&resp), 200);

        let resp = handle_tool_list(&registry);
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        let arr = data.as_array();
        assert!(arr.is_some());
        assert_eq!(arr.map(Vec::len), Some(1));
        assert_eq!(data[0]["name"].as_str(), Some("test-tool"));
    }

    #[test]
    fn tool_create_and_get_by_name() {
        let registry = InMemoryRegistry::new();
        handle_tool_create(&registry, TOOL_JSON);

        let resp = handle_tool_get(&registry, "test-tool");
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        assert_eq!(data["name"].as_str(), Some("test-tool"));
        assert_eq!(data["uri"].as_str(), Some("echo-tool://echo"));
    }

    #[test]
    fn tool_create_defaults_namespace() {
        let registry = InMemoryRegistry::new();
        let resp = handle_tool_create(&registry, TOOL_JSON);
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        assert_eq!(data["namespace"].as_str(), Some("default"));
    }

    #[test]
    fn tool_get_nonexistent_returns_404() {
        let registry = InMemoryRegistry::new();
        let resp = handle_tool_get(&registry, "no-such-tool");
        assert_eq!(status_of(&resp), 404);
        assert!(response_error(&resp).is_some());
    }

    #[test]
    fn tool_delete_existing() {
        let registry = InMemoryRegistry::new();
        handle_tool_create(&registry, TOOL_JSON);

        let resp = handle_tool_delete(&registry, "test-tool");
        assert_eq!(status_of(&resp), 200);

        let resp = handle_tool_get(&registry, "test-tool");
        assert_eq!(status_of(&resp), 404);
    }

    #[test]
    fn tool_delete_nonexistent_returns_404() {
        let registry = InMemoryRegistry::new();
        let resp = handle_tool_delete(&registry, "no-such-tool");
        assert_eq!(status_of(&resp), 404);
    }

    #[test]
    fn tool_create_invalid_json_returns_400() {
        let registry = InMemoryRegistry::new();
        let resp = handle_tool_create(&registry, "not valid json{{{");
        assert_eq!(status_of(&resp), 400);
        assert!(response_error(&resp).is_some());
    }

    // ── Resource handler tests ──────────────────────────────────────

    #[test]
    fn resource_create_and_list() {
        let registry = InMemoryRegistry::new();
        let resp = handle_resource_create(&registry, RESOURCE_JSON);
        assert_eq!(status_of(&resp), 200);

        let resp = handle_resource_list(&registry);
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        let arr = data.as_array();
        assert!(arr.is_some());
        assert_eq!(arr.map(Vec::len), Some(1));
        assert_eq!(data[0]["name"].as_str(), Some("test-resource"));
    }

    #[test]
    fn resource_create_and_get_by_name() {
        let registry = InMemoryRegistry::new();
        handle_resource_create(&registry, RESOURCE_JSON);

        let resp = handle_resource_get(&registry, "test-resource");
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        assert_eq!(data["name"].as_str(), Some("test-resource"));
        assert_eq!(data["location"].as_str(), Some("/tmp/test.txt"));
    }

    #[test]
    fn resource_create_defaults_namespace() {
        let registry = InMemoryRegistry::new();
        let resp = handle_resource_create(&registry, RESOURCE_JSON);
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        assert_eq!(data["namespace"].as_str(), Some("default"));
    }

    #[test]
    fn resource_delete_existing() {
        let registry = InMemoryRegistry::new();
        handle_resource_create(&registry, RESOURCE_JSON);

        let resp = handle_resource_delete(&registry, "test-resource");
        assert_eq!(status_of(&resp), 200);

        let resp = handle_resource_get(&registry, "test-resource");
        assert_eq!(status_of(&resp), 404);
    }

    #[test]
    fn resource_delete_nonexistent_returns_404() {
        let registry = InMemoryRegistry::new();
        let resp = handle_resource_delete(&registry, "no-such-resource");
        assert_eq!(status_of(&resp), 404);
    }

    #[test]
    fn resource_create_invalid_json_returns_400() {
        let registry = InMemoryRegistry::new();
        let resp = handle_resource_create(&registry, "{{broken");
        assert_eq!(status_of(&resp), 400);
    }

    // ── Prompt handler tests ────────────────────────────────────────

    #[test]
    fn prompt_create_and_get() {
        let registry = InMemoryRegistry::new();
        let resp = handle_prompt_create(&registry, PROMPT_JSON);
        assert_eq!(status_of(&resp), 200);

        let resp = handle_prompt_get(&registry, "test-prompt");
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        assert_eq!(data["name"].as_str(), Some("test-prompt"));
        assert_eq!(data["description"].as_str(), Some("A test prompt"));
    }

    #[test]
    fn prompt_delete_nonexistent_returns_404() {
        let registry = InMemoryRegistry::new();
        let resp = handle_prompt_delete(&registry, "no-such-prompt");
        assert_eq!(status_of(&resp), 404);
    }

    #[test]
    fn prompt_create_invalid_json_returns_400() {
        let registry = InMemoryRegistry::new();
        let resp = handle_prompt_create(&registry, "[]");
        assert_eq!(status_of(&resp), 400);
    }

    // ── Namespace handler tests ─────────────────────────────────────

    #[test]
    fn namespace_create_and_get() {
        let registry = InMemoryRegistry::new();
        let resp = handle_namespace_create(&registry, NAMESPACE_JSON);
        assert_eq!(status_of(&resp), 200);

        let resp = handle_namespace_get(&registry, "test-ns");
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        assert_eq!(data["name"].as_str(), Some("test-ns"));
        assert_eq!(data["path"].as_str(), Some("/test-ns/mcp"));
    }

    #[test]
    fn namespace_update_overrides_name_from_path() {
        let registry = InMemoryRegistry::new();
        handle_namespace_create(&registry, NAMESPACE_JSON);

        let update_body = r#"{"name": "ignored", "path": "/updated/mcp"}"#;
        let resp = handle_namespace_update(&registry, "test-ns", update_body);
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        assert_eq!(data["name"].as_str(), Some("test-ns"));
        assert_eq!(data["path"].as_str(), Some("/updated/mcp"));
    }

    #[test]
    fn namespace_delete_nonexistent_returns_404() {
        let registry = InMemoryRegistry::new();
        let resp = handle_namespace_delete(&registry, "no-such-ns");
        assert_eq!(status_of(&resp), 404);
    }

    // ── Service handler tests ───────────────────────────────────────

    #[test]
    fn service_create_and_list() {
        let registry = InMemoryRegistry::new();
        let resp = handle_service_create(&registry, SERVICE_JSON);
        assert_eq!(status_of(&resp), 200);

        let resp = handle_service_list(&registry);
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        let arr = data.as_array();
        assert!(arr.is_some());
        assert_eq!(arr.map(Vec::len), Some(1));
    }

    #[test]
    fn service_get_nonexistent_returns_404() {
        let registry = InMemoryRegistry::new();
        let resp = handle_service_get(&registry, "no-such-service");
        assert_eq!(status_of(&resp), 404);
    }

    #[test]
    fn service_delete_existing() {
        let registry = InMemoryRegistry::new();
        handle_service_create(&registry, SERVICE_JSON);

        let resp = handle_service_delete(&registry, "echo-tool");
        assert_eq!(status_of(&resp), 200);

        let resp = handle_service_get(&registry, "echo-tool");
        assert_eq!(status_of(&resp), 404);
    }

    // ── Statistics handler tests ────────────────────────────────────

    #[test]
    fn statistics_empty_registry() {
        let registry = InMemoryRegistry::new();
        let resp = handle_statistics(&registry);
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        assert_eq!(data["toolsCount"].as_i64(), Some(0));
        assert_eq!(data["resourcesCount"].as_i64(), Some(0));
        assert_eq!(data["promptsCount"].as_i64(), Some(0));
        assert_eq!(data["forwardsCount"].as_i64(), Some(0));
    }

    #[test]
    fn statistics_reflects_registered_entries() {
        let registry = InMemoryRegistry::new();
        handle_tool_create(&registry, TOOL_JSON);
        handle_resource_create(&registry, RESOURCE_JSON);

        let resp = handle_statistics(&registry);
        let data = response_data(&resp);
        assert_eq!(data["toolsCount"].as_i64(), Some(1));
        assert_eq!(data["resourcesCount"].as_i64(), Some(1));
        assert_eq!(data["promptsCount"].as_i64(), Some(0));
    }

    // ── Safety handler tests ────────────────────────────────────────

    #[test]
    fn safety_get_returns_null_when_unconfigured() {
        let state = SafetyState::new();
        let resp = handle_safety_get(&state);
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        assert!(data.is_null());
    }

    #[test]
    fn safety_update_and_get() {
        let state = SafetyState::new();
        let body = r#"{
            "llm_url": "http://localhost:11434/v1",
            "llm_model": "llama3.2",
            "llm_api_key": "",
            "red_action": "block",
            "yellow_action": "warn"
        }"#;
        let resp = handle_safety_update(&state, body);
        assert_eq!(status_of(&resp), 200);

        let resp = handle_safety_get(&state);
        let data = response_data(&resp);
        assert_eq!(data["llm_model"].as_str(), Some("llama3.2"));
        assert_eq!(data["red_action"].as_str(), Some("block"));
    }

    #[test]
    fn safety_delete_clears_config() {
        let state = SafetyState::new();
        let body = r#"{
            "llm_url": "http://localhost:11434/v1",
            "llm_model": "test",
            "llm_api_key": "",
            "red_action": "log",
            "yellow_action": "log"
        }"#;
        handle_safety_update(&state, body);
        handle_safety_delete(&state);

        let resp = handle_safety_get(&state);
        let data = response_data(&resp);
        assert!(data.is_null());
    }

    #[test]
    fn safety_update_invalid_json_returns_400() {
        let state = SafetyState::new();
        let resp = handle_safety_update(&state, "not json");
        assert_eq!(status_of(&resp), 400);
    }

    // ── Capability handler tests ────────────────────────────────────

    #[test]
    fn capability_state_returns_empty_map() {
        let resp = handle_capability_state();
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        assert!(data.is_object());
        assert_eq!(data.as_object().map(|m| m.len()), Some(0));
    }

    #[test]
    fn capability_list_reflects_services() {
        let registry = InMemoryRegistry::new();
        handle_service_create(&registry, SERVICE_JSON);

        let resp = handle_capability_list(&registry);
        assert_eq!(status_of(&resp), 200);

        let data = response_data(&resp);
        let arr = data.as_array();
        assert_eq!(arr.map(Vec::len), Some(1));
        assert_eq!(data[0]["serviceName"].as_str(), Some("echo-tool"));
        assert_eq!(data[0]["host"].as_str(), Some("localhost"));
        assert_eq!(data[0]["port"].as_u64(), Some(9191));
    }
}
