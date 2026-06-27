use std::env;
use std::sync::Arc;

use axum::middleware;
use axum::routing::get;
use axum::{Json, Router};
use his_domain::{HisConfig, PlatformProbe};
use serde::Serialize;
use tracing::info;

mod auth;
mod request_auth;
mod routes;
mod state;

use crate::routes::{adt, clinical_documents, documentation, foundation, orders, registration, scheduling};
use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ReadyResponse {
    ready: bool,
    platform: his_domain::PlatformHealth,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,his_server=debug,his_domain=debug".into()),
        )
        .init();

    let host = env::var("HIS_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = env::var("HIS_SERVER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8096);

    let config = HisConfig::from_env()?;
    config.validate()?;

    let auth_config = auth::auth_config_from_env();
    let auth_state = if auth_config.enabled {
        info!("HIS inbound authentication is ENABLED");
        Some(Arc::new(auth::init_auth_state(&auth_config).await?))
    } else {
        info!("HIS inbound authentication is DISABLED");
        None
    };

    let mut app_state = AppState::from_config(&config)?;
    if let Some(auth) = auth_state.clone() {
        app_state = app_state.with_auth(auth);
    }
    let state = Arc::new(app_state);

    let mut api = registration::routes()
        .merge(scheduling::routes())
        .merge(adt::routes())
        .merge(documentation::routes())
        .merge(clinical_documents::routes())
        .merge(orders::routes())
        .merge(foundation::routes());

    if let Some(auth) = auth_state {
        api = api.layer(middleware::from_fn_with_state(auth, auth::auth_middleware));
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get({
            let config = config.clone();
            move || ready(config.clone())
        }))
        .nest("/api/v1", api)
        .with_state(state);

    let addr: std::net::SocketAddr = format!("{host}:{port}").parse()?;
    info!(%addr, "his-server listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn ready(config: HisConfig) -> impl axum::response::IntoResponse {
    let probe = match PlatformProbe::new(config) {
        Ok(probe) => probe,
        Err(err) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "ready": false, "error": err.to_string() })),
            )
                .into_response();
        }
    };

    let platform = probe.check().await;
    let status = if platform.ready() {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(ReadyResponse {
            ready: platform.ready(),
            platform,
        }),
    )
        .into_response()
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C handler");
    info!("shutdown signal received");
}

use axum::response::IntoResponse;
