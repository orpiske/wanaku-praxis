use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::{DecodingKey, Validation};
use tokio::sync::{Mutex, RwLock};

const JWKS_TTL: Duration = Duration::from_secs(300);
const JWKS_FETCH_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct CachedKey {
    key: DecodingKey,
    algorithm: jsonwebtoken::Algorithm,
}

struct CacheInner {
    keys: HashMap<String, CachedKey>,
    fetched_at: Instant,
}

#[derive(Clone)]
pub struct JwksCache {
    inner: Arc<RwLock<Option<CacheInner>>>,
    refresh_lock: Arc<Mutex<()>>,
    client: reqwest::Client,
    discovery_url: String,
    issuer: String,
}

impl JwksCache {
    pub fn new(auth_server: &str, realm: &str) -> Result<Self, JwksCacheError> {
        let discovery_url = format!(
            "{}/realms/{realm}/.well-known/openid-configuration",
            auth_server.trim_end_matches('/')
        );
        let issuer = format!(
            "{}/realms/{realm}",
            auth_server.trim_end_matches('/')
        );
        let client = reqwest::Client::builder()
            .timeout(JWKS_FETCH_TIMEOUT)
            .build()
            .map_err(|e| JwksCacheError::HttpClient(e.to_string()))?;

        Ok(Self {
            inner: Arc::new(RwLock::new(None)),
            refresh_lock: Arc::new(Mutex::new(())),
            client,
            discovery_url,
            issuer,
        })
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn discovery_url(&self) -> &str {
        &self.discovery_url
    }

    pub async fn bootstrap(&self) -> Result<(), JwksCacheError> {
        self.refresh().await
    }

    pub async fn validate(
        &self,
        token: &str,
        required_audience: Option<&str>,
    ) -> Result<String, TokenError> {
        let header = jsonwebtoken::decode_header(token)
            .map_err(|_| TokenError::InvalidToken)?;

        let kid = header.kid.ok_or(TokenError::InvalidToken)?;

        let subject = match self.try_validate(token, &kid, required_audience).await {
            Ok(sub) => sub,
            Err(TokenError::InvalidToken) => {
                self.guarded_refresh().await.map_err(|_| TokenError::AuthUnavailable)?;
                self.try_validate(token, &kid, required_audience).await?
            }
            Err(e) => return Err(e),
        };

        Ok(subject)
    }

    async fn try_validate(
        &self,
        token: &str,
        kid: &str,
        required_audience: Option<&str>,
    ) -> Result<String, TokenError> {
        let guard = self.inner.read().await;
        let cache = guard.as_ref().ok_or(TokenError::AuthUnavailable)?;

        if cache.fetched_at.elapsed() > JWKS_TTL {
            drop(guard);
            self.guarded_refresh().await.map_err(|_| TokenError::AuthUnavailable)?;
            let guard = self.inner.read().await;
            let cache = guard.as_ref().ok_or(TokenError::AuthUnavailable)?;
            return self.validate_against_cache(token, kid, required_audience, cache);
        }

        let cached = cache.keys.get(kid).ok_or(TokenError::InvalidToken)?;
        self.decode_and_extract(token, cached, required_audience)
    }

    fn validate_against_cache(
        &self,
        token: &str,
        kid: &str,
        required_audience: Option<&str>,
        cache: &CacheInner,
    ) -> Result<String, TokenError> {
        let cached = cache.keys.get(kid).ok_or(TokenError::InvalidToken)?;
        self.decode_and_extract(token, cached, required_audience)
    }

    fn decode_and_extract(
        &self,
        token: &str,
        cached: &CachedKey,
        required_audience: Option<&str>,
    ) -> Result<String, TokenError> {
        let mut validation = Validation::new(cached.algorithm);
        validation.set_issuer(&[&self.issuer]);
        if let Some(aud) = required_audience {
            validation.set_audience(&[aud]);
        } else {
            validation.validate_aud = false;
        }
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.set_required_spec_claims(&["exp", "sub", "iss"]);

        let data = jsonwebtoken::decode::<serde_json::Value>(token, &cached.key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => TokenError::Expired,
                jsonwebtoken::errors::ErrorKind::InvalidAudience => {
                    TokenError::WrongAudience {
                        expected: required_audience.unwrap_or("").to_owned(),
                    }
                }
                jsonwebtoken::errors::ErrorKind::InvalidIssuer => {
                    TokenError::WrongIssuer {
                        expected: self.issuer.clone(),
                    }
                }
                _ => TokenError::InvalidToken,
            })?;

        let sub = data
            .claims
            .get("sub")
            .and_then(serde_json::Value::as_str)
            .ok_or(TokenError::InvalidToken)?
            .to_owned();

        Ok(sub)
    }

    /// Refresh with a mutex guard so only one concurrent refresh happens.
    async fn guarded_refresh(&self) -> Result<(), JwksCacheError> {
        let _guard = self.refresh_lock.lock().await;

        {
            let cache = self.inner.read().await;
            if let Some(inner) = cache.as_ref() {
                if inner.fetched_at.elapsed() < Duration::from_secs(5) {
                    return Ok(());
                }
            }
        }

        self.refresh().await
    }

    async fn refresh(&self) -> Result<(), JwksCacheError> {
        let jwks_uri = self.discover_jwks_uri().await?;
        let keys = self.fetch_jwks(&jwks_uri).await?;

        let cache = CacheInner {
            keys,
            fetched_at: Instant::now(),
        };

        let mut guard = self.inner.write().await;
        *guard = Some(cache);

        tracing::info!("JWKS cache refreshed");
        Ok(())
    }

    async fn discover_jwks_uri(&self) -> Result<String, JwksCacheError> {
        let resp: serde_json::Value = self
            .client
            .get(&self.discovery_url)
            .send()
            .await
            .map_err(|e| JwksCacheError::Discovery(e.to_string()))?
            .error_for_status()
            .map_err(|e| JwksCacheError::Discovery(e.to_string()))?
            .json()
            .await
            .map_err(|e| JwksCacheError::Discovery(e.to_string()))?;

        resp.get("jwks_uri")
            .and_then(serde_json::Value::as_str)
            .map(String::from)
            .ok_or_else(|| JwksCacheError::Discovery("missing jwks_uri in discovery doc".into()))
    }

    async fn fetch_jwks(
        &self,
        jwks_uri: &str,
    ) -> Result<HashMap<String, CachedKey>, JwksCacheError> {
        let resp: serde_json::Value = self
            .client
            .get(jwks_uri)
            .send()
            .await
            .map_err(|e| JwksCacheError::Fetch(e.to_string()))?
            .error_for_status()
            .map_err(|e| JwksCacheError::Fetch(e.to_string()))?
            .json()
            .await
            .map_err(|e| JwksCacheError::Fetch(e.to_string()))?;

        let keys_array = resp
            .get("keys")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| JwksCacheError::Fetch("missing 'keys' array in JWKS".into()))?;

        let mut keys = HashMap::new();
        for jwk in keys_array {
            let kid = match jwk.get("kid").and_then(serde_json::Value::as_str) {
                Some(k) => k.to_owned(),
                None => continue,
            };

            let kty = jwk.get("kty").and_then(serde_json::Value::as_str).unwrap_or("");
            let alg = jwk.get("alg").and_then(serde_json::Value::as_str).unwrap_or("");

            let parsed = match (kty, alg) {
                ("RSA", "RS256") | ("RSA", "") => parse_rsa_key(jwk, jsonwebtoken::Algorithm::RS256),
                ("RSA", "RS384") => parse_rsa_key(jwk, jsonwebtoken::Algorithm::RS384),
                ("RSA", "RS512") => parse_rsa_key(jwk, jsonwebtoken::Algorithm::RS512),
                _ => {
                    tracing::warn!(kid = %kid, kty = %kty, alg = %alg, "skipping unsupported key type");
                    continue;
                }
            };

            match parsed {
                Ok(cached) => {
                    keys.insert(kid, cached);
                }
                Err(e) => {
                    tracing::warn!(kid = %kid, error = %e, "failed to parse JWKS key");
                }
            }
        }

        if keys.is_empty() {
            return Err(JwksCacheError::Fetch("no usable keys in JWKS".into()));
        }

        tracing::debug!(key_count = keys.len(), "parsed JWKS keys");
        Ok(keys)
    }

    pub async fn is_healthy(&self) -> bool {
        let guard = self.inner.read().await;
        guard
            .as_ref()
            .is_some_and(|c| c.fetched_at.elapsed() < JWKS_TTL * 2)
    }
}

fn parse_rsa_key(
    jwk: &serde_json::Value,
    algorithm: jsonwebtoken::Algorithm,
) -> Result<CachedKey, JwksCacheError> {
    let n = jwk
        .get("n")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| JwksCacheError::Fetch("missing 'n' in RSA key".into()))?;
    let e = jwk
        .get("e")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| JwksCacheError::Fetch("missing 'e' in RSA key".into()))?;

    let key = DecodingKey::from_rsa_components(n, e)
        .map_err(|err| JwksCacheError::Fetch(format!("invalid RSA components: {err}")))?;

    Ok(CachedKey { key, algorithm })
}

#[derive(Debug, thiserror::Error)]
pub enum JwksCacheError {
    #[error("failed to build HTTP client: {0}")]
    HttpClient(String),
    #[error("OIDC discovery failed: {0}")]
    Discovery(String),
    #[error("JWKS fetch failed: {0}")]
    Fetch(String),
}

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("missing_token: No Authorization header. Set 'Authorization: Bearer <token>' or use a public namespace.")]
    MissingToken,
    #[error("invalid_request: Authorization header must be 'Bearer <token>'.")]
    MalformedHeader,
    #[error("token_expired: Access token has expired. Obtain a new token from the auth server.")]
    Expired,
    #[error("invalid_token: Token signature verification failed.")]
    InvalidToken,
    #[error("wrong_audience: Token audience does not include '{expected}'. Ensure you requested the correct scope.")]
    WrongAudience { expected: String },
    #[error("invalid_issuer: Token issuer does not match expected '{expected}'.")]
    WrongIssuer { expected: String },
    #[error("auth_unavailable: Cannot validate tokens — auth server is temporarily unreachable. Try again shortly.")]
    AuthUnavailable,
}

impl TokenError {
    #[must_use]
    pub fn status_code(&self) -> u16 {
        match self {
            Self::MissingToken
            | Self::MalformedHeader
            | Self::Expired
            | Self::InvalidToken
            | Self::WrongIssuer { .. } => 401,
            Self::WrongAudience { .. } => 403,
            Self::AuthUnavailable => 503,
        }
    }

    #[must_use]
    pub fn www_authenticate(&self) -> Option<String> {
        match self {
            Self::MissingToken => Some("Bearer realm=\"wanaku\"".into()),
            Self::MalformedHeader => {
                Some("Bearer realm=\"wanaku\", error=\"invalid_request\"".into())
            }
            Self::Expired | Self::InvalidToken | Self::WrongIssuer { .. } => {
                Some("Bearer realm=\"wanaku\", error=\"invalid_token\"".into())
            }
            Self::WrongAudience { .. } => {
                Some("Bearer realm=\"wanaku\", error=\"insufficient_scope\"".into())
            }
            Self::AuthUnavailable => None,
        }
    }
}
