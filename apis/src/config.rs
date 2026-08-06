//! Centralized environment variable configuration for Wanaku Praxis.
//!
//! All `WANAKU_*` environment variables are read once at startup via
//! [`LazyLock`] and exposed through the [`ENV`] static. No other module
//! should call `std::env::var` for these variables directly.

use std::path::PathBuf;
use std::sync::LazyLock;

/// Management API listen address (default `0.0.0.0:9090`).
const WANAKU_MGMT_LISTEN: &str = "WANAKU_MGMT_LISTEN";

/// Ollama backend address used in the default Praxis config (default `127.0.0.1:11434`).
const WANAKU_OLLAMA_UPSTREAM: &str = "WANAKU_OLLAMA_UPSTREAM";

/// Persistence backend selector. Set to `"file"` to enable file-based persistence.
/// Unset or any other value disables persistence.
const WANAKU_PERSIST_BACKEND: &str = "WANAKU_PERSIST_BACKEND";

/// Directory where `registry.json` is stored (default `/data/registry`).
/// Only used when [`WANAKU_PERSIST_BACKEND`] is `"file"`.
const WANAKU_PERSIST_PATH: &str = "WANAKU_PERSIST_PATH";

/// Base URL for the Classic proxy backend. Unset disables proxying.
const WANAKU_CLASSIC_URL: &str = "WANAKU_CLASSIC_URL";

/// Filesystem path to serve the admin UI from instead of the embedded assets.
/// Unset uses the compiled-in [`rust_embed`] bundle.
const WANAKU_UI_PATH: &str = "WANAKU_UI_PATH";

/// Base URL for the OpenAI-compatible safety classifier LLM.
/// Unset disables the safety classification feature entirely.
const WANAKU_SAFETY_LLM_URL: &str = "WANAKU_SAFETY_LLM_URL";

/// Model name sent in the `/v1/chat/completions` request (default `llama3.2`).
const WANAKU_SAFETY_LLM_MODEL: &str = "WANAKU_SAFETY_LLM_MODEL";

/// Bearer token for the safety classifier LLM. Empty string if not needed.
const WANAKU_SAFETY_LLM_API_KEY: &str = "WANAKU_SAFETY_LLM_API_KEY";

/// Action when classification is **red**: `log`, `warn`, or `block` (default `log`).
const WANAKU_SAFETY_RED_ACTION: &str = "WANAKU_SAFETY_RED_ACTION";

/// Action when classification is **yellow**: `log`, `warn`, or `block` (default `log`).
const WANAKU_SAFETY_YELLOW_ACTION: &str = "WANAKU_SAFETY_YELLOW_ACTION";

/// Authentication mode selector. Set to `"keycloak"` to enable OIDC auth.
/// Unset or `"none"` disables all authentication.
const WANAKU_HTTP_AUTH: &str = "WANAKU_HTTP_AUTH";

/// Keycloak base URL (e.g. `http://localhost:8543`).
const WANAKU_AUTH_SERVER: &str = "WANAKU_AUTH_SERVER";

/// Keycloak realm name (default `wanaku`).
const WANAKU_AUTH_REALM: &str = "WANAKU_AUTH_REALM";

/// OIDC client ID for the router (default `wanaku-mcp-router`).
const WANAKU_AUTH_CLIENT_ID: &str = "WANAKU_AUTH_CLIENT_ID";

/// Required audience claim for MCP bearer tokens (default `wanaku-mcp-client`).
const WANAKU_AUTH_MCP_AUDIENCE: &str = "WANAKU_AUTH_MCP_AUDIENCE";

/// Comma-separated namespace names that skip authentication (default `public`).
const WANAKU_AUTH_PUBLIC_NAMESPACES: &str = "WANAKU_AUTH_PUBLIC_NAMESPACES";

/// Maximum seconds to wait for the auth server at startup (default `60`).
const WANAKU_AUTH_STARTUP_TIMEOUT: &str = "WANAKU_AUTH_STARTUP_TIMEOUT";

/// File-persistence settings, present only when enabled.
#[derive(Debug, Clone)]
pub struct PersistEnv {
    /// Directory containing `registry.json`.
    pub dir: PathBuf,
}

/// OIDC authentication settings, present only when `WANAKU_HTTP_AUTH=keycloak`.
#[derive(Debug, Clone)]
pub struct AuthEnv {
    /// Keycloak base URL.
    pub server: String,
    /// Keycloak realm name.
    pub realm: String,
    /// OIDC client ID (public client used for code flow).
    pub client_id: String,
    /// Required `aud` claim for MCP bearer tokens.
    pub mcp_audience: String,
    /// Namespace names that skip authentication.
    pub public_namespaces: Vec<String>,
    /// Maximum seconds to wait for auth server at startup.
    pub startup_timeout_secs: u64,
}

/// Safety classifier settings, present only when enabled.
#[derive(Debug, Clone)]
pub struct SafetyEnv {
    /// Full base URL for the OpenAI-compatible endpoint (e.g. `http://localhost:11434/v1`).
    pub llm_url: String,
    /// Model name passed in `/v1/chat/completions`.
    pub llm_model: String,
    /// Bearer token for authentication (empty if not needed).
    pub llm_api_key: String,
    /// Configured action for red (dangerous) classifications.
    pub red_action: String,
    /// Configured action for yellow (ambiguous) classifications.
    pub yellow_action: String,
}

/// Typed snapshot of all `WANAKU_*` environment variables.
#[derive(Debug, Clone)]
pub struct WanakuEnv {
    /// Management API listen address.
    pub mgmt_listen: String,
    /// Ollama upstream address for the default Praxis pipeline config.
    pub ollama_upstream: String,
    /// File-persistence config. `None` when persistence is disabled.
    pub persist: Option<PersistEnv>,
    /// Classic proxy base URL. `None` when proxying is disabled.
    pub classic_url: Option<String>,
    /// Override path for serving the admin UI from the filesystem.
    pub ui_path: Option<PathBuf>,
    /// Safety classifier config. `None` when the feature is disabled.
    pub safety: Option<SafetyEnv>,
    /// OIDC auth config. `None` when auth is disabled.
    pub auth: Option<AuthEnv>,
}

/// Global configuration, initialized lazily on first access.
pub static ENV: LazyLock<WanakuEnv> = LazyLock::new(WanakuEnv::from_env);

impl WanakuEnv {
    #[must_use]
    pub fn ollama_proxy_port(&self) -> u16 {
        8082
    }

    fn from_env() -> Self {
        let persist = std::env::var(WANAKU_PERSIST_BACKEND)
            .ok()
            .filter(|b| b == "file")
            .map(|_| {
                let dir = std::env::var(WANAKU_PERSIST_PATH)
                    .unwrap_or_else(|_| "/data/registry".to_owned());
                PersistEnv {
                    dir: PathBuf::from(dir),
                }
            });

        Self {
            mgmt_listen: std::env::var(WANAKU_MGMT_LISTEN)
                .unwrap_or_else(|_| "0.0.0.0:9090".to_owned()),
            ollama_upstream: std::env::var(WANAKU_OLLAMA_UPSTREAM)
                .unwrap_or_else(|_| "127.0.0.1:11434".to_owned()),
            persist,
            classic_url: std::env::var(WANAKU_CLASSIC_URL)
                .ok()
                .map(|u| u.trim_end_matches('/').to_owned()),
            ui_path: std::env::var(WANAKU_UI_PATH).ok().map(PathBuf::from),
            safety: std::env::var(WANAKU_SAFETY_LLM_URL)
                .ok()
                .filter(|u| !u.is_empty())
                .map(|url| SafetyEnv {
                    llm_url: url.trim_end_matches('/').to_owned(),
                    llm_model: std::env::var(WANAKU_SAFETY_LLM_MODEL)
                        .unwrap_or_else(|_| "llama3.2".to_owned()),
                    llm_api_key: std::env::var(WANAKU_SAFETY_LLM_API_KEY)
                        .unwrap_or_default(),
                    red_action: std::env::var(WANAKU_SAFETY_RED_ACTION)
                        .unwrap_or_else(|_| "log".to_owned()),
                    yellow_action: std::env::var(WANAKU_SAFETY_YELLOW_ACTION)
                        .unwrap_or_else(|_| "log".to_owned()),
                }),
            auth: std::env::var(WANAKU_HTTP_AUTH)
                .ok()
                .filter(|v| v.eq_ignore_ascii_case("keycloak"))
                .map(|_| AuthEnv {
                    server: std::env::var(WANAKU_AUTH_SERVER)
                        .unwrap_or_else(|_| "http://localhost:8543".to_owned()),
                    realm: std::env::var(WANAKU_AUTH_REALM)
                        .unwrap_or_else(|_| "wanaku".to_owned()),
                    client_id: std::env::var(WANAKU_AUTH_CLIENT_ID)
                        .unwrap_or_else(|_| "wanaku-mcp-router".to_owned()),
                    mcp_audience: std::env::var(WANAKU_AUTH_MCP_AUDIENCE)
                        .unwrap_or_else(|_| "wanaku-mcp-client".to_owned()),
                    public_namespaces: std::env::var(WANAKU_AUTH_PUBLIC_NAMESPACES)
                        .unwrap_or_else(|_| "public".to_owned())
                        .split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    startup_timeout_secs: std::env::var(WANAKU_AUTH_STARTUP_TIMEOUT)
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(60),
                }),
        }
    }
}
