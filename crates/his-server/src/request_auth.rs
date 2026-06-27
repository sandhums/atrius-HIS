use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use helios_auth::Principal;

use crate::state::AppState;

/// Caller identity and bearer token for upstream HFS calls.
#[derive(Clone, Debug)]
pub struct RequestAuth {
    pub bearer_token: String,
    pub tenant_id: String,
    pub principal: Option<Principal>,
}

impl RequestAuth {
    pub fn from_headers(parts: &Parts, default_tenant: &str, fallback_bearer: Option<&str>) -> Self {
        let bearer_token = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer ").or_else(|| h.strip_prefix("bearer ")))
            .map(str::to_string)
            .or_else(|| fallback_bearer.map(str::to_string))
            .unwrap_or_default();

        let tenant_id = parts
            .headers
            .get("X-Tenant-ID")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
            .unwrap_or_else(|| default_tenant.to_string());

        Self {
            bearer_token,
            tenant_id,
            principal: None,
        }
    }
}

impl FromRequestParts<Arc<AppState>> for RequestAuth {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        if let Some(auth) = parts.extensions.get::<RequestAuth>() {
            return Ok(auth.clone());
        }

        if state.auth_enabled() {
            return Err((
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({
                    "error": "unauthorized",
                    "message": "missing validated Authorization context"
                })),
            )
                .into_response());
        }

        Ok(Self::from_headers(
            parts,
            &state.config.default_tenant,
            state.config.fhir_bearer_token.as_deref(),
        ))
    }
}
