pub mod jwks;

use std::collections::HashSet;
use std::time::Duration;

use crate::config::AuthEnv;

pub use jwks::TokenError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    None,
    Keycloak,
}

#[derive(Clone)]
pub struct AuthState {
    mode: AuthMode,
    cache: Option<jwks::JwksCache>,
    mcp_audience: String,
    public_namespaces: HashSet<String>,
    client_id: String,
    auth_server: String,
    realm: String,
}

impl AuthState {
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            mode: AuthMode::None,
            cache: None,
            mcp_audience: String::new(),
            public_namespaces: HashSet::new(),
            client_id: String::new(),
            auth_server: String::new(),
            realm: String::new(),
        }
    }

    /// Build an enabled auth state from config.
    ///
    /// Does NOT bootstrap the JWKS cache — call [`AuthState::bootstrap`] after
    /// construction to validate connectivity and populate the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the JWKS cache cannot be constructed.
    pub fn from_config(env: &AuthEnv) -> Result<Self, jwks::JwksCacheError> {
        let cache = jwks::JwksCache::new(&env.server, &env.realm)?;

        Ok(Self {
            mode: AuthMode::Keycloak,
            cache: Some(cache),
            mcp_audience: env.mcp_audience.clone(),
            public_namespaces: env.public_namespaces.iter().cloned().collect(),
            client_id: env.client_id.clone(),
            auth_server: env.server.clone(),
            realm: env.realm.clone(),
        })
    }

    /// Wait for the auth server and populate the JWKS cache.
    ///
    /// Retries with exponential backoff up to `timeout`. Aborts with an error
    /// if the auth server is unreachable after the timeout expires.
    ///
    /// # Errors
    ///
    /// Returns an error if the JWKS cache cannot be bootstrapped within the timeout.
    pub async fn bootstrap(&self, timeout: Duration) -> Result<(), AuthBootstrapError> {
        let cache = match &self.cache {
            Some(c) => c,
            None => return Ok(()),
        };

        let start = tokio::time::Instant::now();
        let mut attempt: u32 = 0;
        let mut delay = Duration::from_secs(1);

        loop {
            attempt += 1;
            match cache.bootstrap().await {
                Ok(()) => {
                    tracing::info!("Auth server connected, JWKS cache populated");
                    return Ok(());
                }
                Err(e) => {
                    let elapsed = start.elapsed();
                    if elapsed >= timeout {
                        return Err(AuthBootstrapError {
                            url: cache.discovery_url().to_owned(),
                            timeout_secs: timeout.as_secs(),
                            last_error: e.to_string(),
                        });
                    }

                    let remaining = timeout.saturating_sub(elapsed);
                    let sleep_for = delay.min(remaining);

                    tracing::warn!(
                        url = %cache.discovery_url(),
                        attempt = attempt,
                        elapsed_secs = elapsed.as_secs(),
                        error = %e,
                        "Waiting for auth server"
                    );

                    tokio::time::sleep(sleep_for).await;
                    delay = (delay * 2).min(Duration::from_secs(16));
                }
            }
        }
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.mode == AuthMode::Keycloak
    }

    #[must_use]
    pub fn mode(&self) -> AuthMode {
        self.mode
    }

    #[must_use]
    pub fn is_public_namespace(&self, namespace: &str) -> bool {
        self.public_namespaces.contains(namespace)
    }

    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    #[must_use]
    pub fn auth_server(&self) -> &str {
        &self.auth_server
    }

    #[must_use]
    pub fn realm(&self) -> &str {
        &self.realm
    }

    #[must_use]
    pub fn mcp_audience(&self) -> &str {
        &self.mcp_audience
    }

    /// Validate a bearer token for MCP endpoints (requires MCP audience).
    ///
    /// When auth is disabled, returns `Ok("anonymous")`.
    ///
    /// # Errors
    ///
    /// Returns a [`TokenError`] with an actionable error message.
    pub async fn validate_bearer_token(&self, token: &str) -> Result<String, TokenError> {
        if !self.is_enabled() {
            return Ok("anonymous".into());
        }
        let cache = self.cache.as_ref().ok_or(TokenError::AuthUnavailable)?;
        cache.validate(token, Some(&self.mcp_audience)).await
    }

    /// Validate a bearer token for the management API (no audience requirement).
    ///
    /// When auth is disabled, returns `Ok("anonymous")`.
    ///
    /// # Errors
    ///
    /// Returns a [`TokenError`] with an actionable error message.
    pub async fn validate_management_token(&self, token: &str) -> Result<String, TokenError> {
        if !self.is_enabled() {
            return Ok("anonymous".into());
        }
        let cache = self.cache.as_ref().ok_or(TokenError::AuthUnavailable)?;
        cache.validate(token, None).await
    }

    /// Extract and validate a bearer token for MCP endpoints (requires MCP audience).
    ///
    /// When auth is disabled, returns `Ok("anonymous")` regardless of the header.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::MissingToken`] if the header is absent,
    /// [`TokenError::MalformedHeader`] if not `Bearer <token>`, or
    /// a validation error from the JWKS cache.
    pub async fn validate_authorization_header(
        &self,
        header_value: Option<&str>,
    ) -> Result<String, TokenError> {
        if !self.is_enabled() {
            return Ok("anonymous".into());
        }

        let token = extract_token_from_header(header_value)?;
        self.validate_bearer_token(token).await
    }

    /// Extract and validate a bearer token for the management API (no audience requirement).
    ///
    /// When auth is disabled, returns `Ok("anonymous")` regardless of the header.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::MissingToken`] if the header is absent,
    /// [`TokenError::MalformedHeader`] if not `Bearer <token>`, or
    /// a validation error from the JWKS cache.
    pub async fn validate_management_authorization_header(
        &self,
        header_value: Option<&str>,
    ) -> Result<String, TokenError> {
        if !self.is_enabled() {
            return Ok("anonymous".into());
        }

        let token = extract_token_from_header(header_value)?;
        self.validate_management_token(token).await
    }

    pub async fn health_status(&self) -> &'static str {
        if self.mode == AuthMode::None {
            return "disabled";
        }
        match &self.cache {
            Some(c) if c.is_healthy().await => "ok",
            Some(_) => "degraded",
            None => "disabled",
        }
    }
}

fn extract_token_from_header(header_value: Option<&str>) -> Result<&str, TokenError> {
    let value = header_value.ok_or(TokenError::MissingToken)?;
    let token = extract_bearer_token(value).ok_or(TokenError::MalformedHeader)?;
    if token.is_empty() {
        return Err(TokenError::MalformedHeader);
    }
    Ok(token)
}

/// Extract the token from a `Bearer <token>` header value (case-insensitive per RFC 7235).
fn extract_bearer_token(header: &str) -> Option<&str> {
    let prefix_len = "Bearer ".len();
    if header.len() < prefix_len {
        return None;
    }
    if header[..prefix_len - 1].eq_ignore_ascii_case("Bearer") && header.as_bytes()[prefix_len - 1] == b' ' {
        Some(&header[prefix_len..])
    } else {
        None
    }
}

#[derive(Debug, thiserror::Error)]
#[error(
    "Auth server unreachable at {url} after {timeout_secs}s. \
     Either start Keycloak or set WANAKU_HTTP_AUTH=none to run without authentication. \
     Last error: {last_error}"
)]
pub struct AuthBootstrapError {
    pub url: String,
    pub timeout_secs: u64,
    pub last_error: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_state() {
        let state = AuthState::disabled();
        assert!(!state.is_enabled());
        assert_eq!(state.mode(), AuthMode::None);
    }

    #[test]
    fn public_namespace_check() {
        let env = AuthEnv {
            server: "http://localhost:8543".into(),
            realm: "wanaku".into(),
            client_id: "test".into(),
            mcp_audience: "test".into(),
            public_namespaces: vec!["public".into(), "demo".into()],
            startup_timeout_secs: 5,
        };
        let state = AuthState::from_config(&env);
        assert!(state.is_ok());
        let state = state.ok();
        assert!(state.as_ref().is_some_and(|s| s.is_enabled()));
        assert!(state.as_ref().is_some_and(|s| s.is_public_namespace("public")));
        assert!(state.as_ref().is_some_and(|s| s.is_public_namespace("demo")));
        assert!(state.as_ref().is_some_and(|s| !s.is_public_namespace("finance")));
    }

    #[test]
    fn config_accessors() {
        let env = AuthEnv {
            server: "http://kc:8543".into(),
            realm: "myrealm".into(),
            client_id: "my-client".into(),
            mcp_audience: "my-audience".into(),
            public_namespaces: vec![],
            startup_timeout_secs: 30,
        };
        let state = AuthState::from_config(&env);
        assert!(state.is_ok());
        let state = state.ok();
        assert_eq!(state.as_ref().map(AuthState::auth_server), Some("http://kc:8543"));
        assert_eq!(state.as_ref().map(AuthState::realm), Some("myrealm"));
        assert_eq!(state.as_ref().map(AuthState::client_id), Some("my-client"));
        assert_eq!(state.as_ref().map(AuthState::mcp_audience), Some("my-audience"));
    }

    #[tokio::test]
    async fn validate_disabled_passes_through() {
        let state = AuthState::disabled();
        let result = state.validate_authorization_header(None).await;
        assert_eq!(result.ok().as_deref(), Some("anonymous"));

        let result = state.validate_authorization_header(Some("Bearer token")).await;
        assert_eq!(result.ok().as_deref(), Some("anonymous"));

        let result = state.validate_bearer_token("some.jwt.token").await;
        assert_eq!(result.ok().as_deref(), Some("anonymous"));
    }

    #[tokio::test]
    async fn validate_missing_header_on_enabled() {
        let env = AuthEnv {
            server: "http://localhost:1".into(),
            realm: "test".into(),
            client_id: "test".into(),
            mcp_audience: "test".into(),
            public_namespaces: vec![],
            startup_timeout_secs: 1,
        };
        let state = AuthState::from_config(&env);
        assert!(state.is_ok());
        let state = state.ok();
        if let Some(s) = &state {
            let result = s.validate_authorization_header(None).await;
            assert!(matches!(result, Err(TokenError::MissingToken)));
        }
    }

    #[tokio::test]
    async fn validate_malformed_header() {
        let env = AuthEnv {
            server: "http://localhost:1".into(),
            realm: "test".into(),
            client_id: "test".into(),
            mcp_audience: "test".into(),
            public_namespaces: vec![],
            startup_timeout_secs: 1,
        };
        let state = AuthState::from_config(&env);
        assert!(state.is_ok());
        let state = state.ok();
        if let Some(s) = &state {
            let result = s.validate_authorization_header(Some("Basic abc123")).await;
            assert!(matches!(result, Err(TokenError::MalformedHeader)));
        }
    }

    #[test]
    fn bearer_case_insensitive() {
        assert_eq!(extract_bearer_token("Bearer abc"), Some("abc"));
        assert_eq!(extract_bearer_token("bearer abc"), Some("abc"));
        assert_eq!(extract_bearer_token("BEARER abc"), Some("abc"));
        assert_eq!(extract_bearer_token("BeArEr abc"), Some("abc"));
        assert_eq!(extract_bearer_token("Basic abc"), None);
        assert_eq!(extract_bearer_token("Bear"), None);
        assert_eq!(extract_bearer_token(""), None);
    }

    #[test]
    fn token_error_status_codes() {
        assert_eq!(TokenError::MissingToken.status_code(), 401);
        assert_eq!(TokenError::MalformedHeader.status_code(), 401);
        assert_eq!(TokenError::Expired.status_code(), 401);
        assert_eq!(TokenError::InvalidToken.status_code(), 401);
        assert_eq!(
            TokenError::WrongAudience {
                expected: "x".into()
            }
            .status_code(),
            403
        );
        assert_eq!(TokenError::AuthUnavailable.status_code(), 503);
    }

    #[test]
    fn token_error_www_authenticate() {
        assert!(TokenError::MissingToken.www_authenticate().is_some());
        assert!(TokenError::MalformedHeader.www_authenticate().is_some());
        assert!(TokenError::AuthUnavailable.www_authenticate().is_none());
    }
}
