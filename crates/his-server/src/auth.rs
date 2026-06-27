use std::sync::Arc;

use axum::{
    Json,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use helios_auth::{
    AuthConfig, AuthProvider, InMemoryJtiCache, JtiCache, JwksBearerAuthProvider, JwksCache,
    Principal, build_jti_revocation,
};
use tracing::{info, warn};

use crate::request_auth::RequestAuth;

/// Shared JWT validation state for inbound HIS API requests.
pub struct AuthState {
    pub config: AuthConfig,
    pub provider: Arc<dyn AuthProvider>,
}

const EXEMPT_PATHS: &[&str] = &["/health", "/ready"];

fn is_exempt_path(path: &str) -> bool {
    let path = path.trim_end_matches('/');
    EXEMPT_PATHS.contains(&path)
}

/// Load auth configuration from `HIS_AUTH_*`, falling back to `HFS_AUTH_*` for local dev parity.
pub fn auth_config_from_env() -> AuthConfig {
    fn env_bool(primary: &str, fallback: &str) -> bool {
        std::env::var(primary)
            .or_else(|_| std::env::var(fallback))
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false)
    }

    fn env_opt(primary: &str, fallback: &str) -> Option<String> {
        std::env::var(primary)
            .ok()
            .or_else(|| std::env::var(fallback).ok())
            .filter(|v| !v.trim().is_empty())
    }

    AuthConfig {
        enabled: env_bool("HIS_AUTH_ENABLED", "HFS_AUTH_ENABLED"),
        jwks_url: env_opt("HIS_AUTH_JWKS_URL", "HFS_AUTH_JWKS_URL"),
        expected_issuer: env_opt("HIS_AUTH_ISSUER", "HFS_AUTH_ISSUER"),
        expected_audience: env_opt("HIS_AUTH_AUDIENCE", "HFS_AUTH_AUDIENCE"),
        tenant_claim: std::env::var("HIS_AUTH_TENANT_CLAIM")
            .or_else(|_| std::env::var("HFS_AUTH_TENANT_CLAIM"))
            .unwrap_or_else(|_| "organization_id".to_string()),
        allowed_algorithms: std::env::var("HIS_AUTH_ALGORITHMS")
            .or_else(|_| std::env::var("HFS_AUTH_ALGORITHMS"))
            .unwrap_or_else(|_| "RS256,RS384,ES256,ES384".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect(),
        jti_backend: std::env::var("HIS_AUTH_JTI_BACKEND")
            .or_else(|_| std::env::var("HFS_AUTH_JTI_BACKEND"))
            .unwrap_or_else(|_| "disabled".to_string()),
        jti_revocation_enabled: env_bool("HIS_AUTH_JTI_REVOCATION", "HFS_AUTH_JTI_REVOCATION"),
        redis_url: env_opt("HIS_AUTH_REDIS_URL", "HFS_AUTH_REDIS_URL"),
        jwks_min_refresh_interval: std::env::var("HIS_AUTH_JWKS_MIN_REFRESH_INTERVAL")
            .or_else(|_| std::env::var("HFS_AUTH_JWKS_MIN_REFRESH_INTERVAL"))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10),
        jwks_insecure_tls: env_bool("HIS_AUTH_INSECURE_TLS", "HFS_AUTH_INSECURE_TLS"),
        ..AuthConfig::default()
    }
}

pub async fn init_auth_state(config: &AuthConfig) -> anyhow::Result<AuthState> {
    let jwks_url = config
        .jwks_url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("HIS_AUTH_JWKS_URL is required when HIS_AUTH_ENABLED=true"))?;

    if config.expected_issuer.is_none() {
        anyhow::bail!("HIS_AUTH_ISSUER is required when HIS_AUTH_ENABLED=true");
    }

    let jti_cache: Arc<dyn JtiCache> = match config.jti_backend.as_str() {
        "memory" => {
            info!("HIS auth: in-memory JTI cache");
            Arc::new(InMemoryJtiCache::new())
        }
        "disabled" | "none" => {
            info!("HIS auth: JTI replay cache disabled");
            Arc::new(helios_auth::DisabledJtiCache)
        }
        other => {
            anyhow::bail!(
                "Invalid HIS_AUTH_JTI_BACKEND '{other}'. Supported: memory, disabled"
            );
        }
    };

    if config.jwks_insecure_tls {
        info!("HIS auth: JWKS fetch accepts invalid TLS certificates (dev only)");
    }
    let jwks_cache = Arc::new(JwksCache::with_insecure_tls(
        jwks_url,
        config.jwks_min_refresh_interval,
        config.jwks_insecure_tls,
    ));
    jwks_cache.initial_fetch().await?;

    let jti_revocation = build_jti_revocation(config)
        .map_err(|e| anyhow::anyhow!("JTI revocation init: {e}"))?;
    if config.jti_revocation_enabled {
        info!("HIS auth: JTI revocation blocklist enabled (shared Redis)");
    }

    let provider = Arc::new(JwksBearerAuthProvider::new(
        jwks_cache,
        jti_cache,
        jti_revocation,
        config,
    )) as Arc<dyn AuthProvider>;

    Ok(AuthState {
        config: config.clone(),
        provider,
    })
}

pub async fn auth_middleware(
    State(auth_state): State<Arc<AuthState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if is_exempt_path(&path) {
        return next.run(request).await;
    }

    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let Some(auth_header) = auth_header else {
        return unauthorized_response("Missing Authorization header");
    };

    let bearer_token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))
        .unwrap_or(&auth_header)
        .to_string();

    match auth_state.provider.authenticate(&auth_header).await {
        Ok(principal) => {
            let tenant_id = tenant_from_request(&request, &principal);
            request.extensions_mut().insert(RequestAuth {
                bearer_token,
                tenant_id,
                principal: Some(principal),
            });
            next.run(request).await
        }
        Err(err) => {
            warn!(error = %err, path = %path, "HIS authentication failed");
            unauthorized_response(&err.to_string())
        }
    }
}

fn tenant_from_request(request: &Request, principal: &Principal) -> String {
    if let Some(tenant) = principal.tenant_id() {
        return tenant.to_string();
    }
    request
        .headers()
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| "atrius-hospital".to_string())
}

fn unauthorized_response(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(
            header::WWW_AUTHENTICATE,
            axum::http::HeaderValue::from_static("Bearer"),
        )],
        Json(serde_json::json!({
            "error": "unauthorized",
            "message": message
        })),
    )
        .into_response()
}
