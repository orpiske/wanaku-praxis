use bytes::Bytes;
use praxis_filter::{FilterAction, FilterError, HttpFilterContext, Rejection};
use wanaku_praxis_apis::auth::{AuthState, TokenError};

crate::body_filter_boilerplate!(AuthFilter, "wanaku_auth");

const AUTH_SUB_METADATA_KEY: &str = "wanaku.auth.sub";
const JSONRPC_AUTH_ERROR: i32 = -32001;

impl AuthFilter {
    async fn handle_body(
        &self,
        ctx: &mut HttpFilterContext<'_>,
        body: &mut Option<Bytes>,
    ) -> Result<FilterAction, FilterError> {
        // Skip auth for non-MCP requests (CORS preflight OPTIONS, etc.).
        // The MCP filter runs before this and sets mcp.method only for valid
        // JSON-RPC requests. OPTIONS preflight has no body, so mcp.method
        // is never set — letting it pass through to the CORS filter's
        // on_request handler.
        if ctx.get_metadata(crate::MCP_METHOD_KEY).is_none() {
            return Ok(FilterAction::Continue);
        }

        let auth_state = match ctx.extensions.get::<AuthState>() {
            Some(s) => s,
            None => return Ok(FilterAction::Continue),
        };

        if !auth_state.is_enabled() {
            return Ok(FilterAction::Continue);
        }

        let namespace = ctx
            .get_metadata(crate::namespace::NAMESPACE_METADATA_KEY)
            .unwrap_or("default");

        if auth_state.is_public_namespace(namespace) {
            tracing::debug!(namespace = %namespace, "skipping auth for public namespace");
            return Ok(FilterAction::Continue);
        }

        let auth_header = ctx
            .request
            .headers
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        let json_rpc_id = crate::response::extract_json_rpc_id(body);

        match auth_state.validate_authorization_header(auth_header).await {
            Ok(subject) => {
                tracing::debug!(subject = %subject, namespace = %namespace, "auth validated");
                ctx.set_metadata(AUTH_SUB_METADATA_KEY, &subject);
                Ok(FilterAction::Continue)
            }
            Err(e) => {
                tracing::debug!(error = %e, namespace = %namespace, "auth rejected");
                Ok(FilterAction::Reject(auth_rejection(&e, &json_rpc_id)))
            }
        }
    }
}

fn auth_rejection(error: &TokenError, json_rpc_id: &serde_json::Value) -> Rejection {
    let status = error.status_code();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": json_rpc_id,
        "error": {
            "code": JSONRPC_AUTH_ERROR,
            "message": error.to_string(),
        }
    });

    let mut rejection = Rejection::status(status)
        .with_header("content-type", "application/json")
        .with_header("access-control-allow-origin", "*")
        .with_header("access-control-expose-headers", "WWW-Authenticate");

    if let Some(www_auth) = error.www_authenticate() {
        rejection = rejection.with_header("www-authenticate", &www_auth);
    }

    rejection.with_body(Bytes::from(body.to_string()))
}
