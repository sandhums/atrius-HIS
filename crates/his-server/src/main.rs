use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use his_domain::{HisConfig, PlatformProbe};
use serde::Serialize;
use tracing::info;

mod routes;
mod state;

use crate::routes::{adt, documentation, registration, scheduling};
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

    let state = Arc::new(AppState::from_config(&config)?);

    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get({
            let config = config.clone();
            move || ready(config.clone())
        }))
        .nest(
            "/api/v1",
            registration::routes()
                .merge(scheduling::routes())
                .merge(adt::routes())
                .merge(documentation::routes()),
        )
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
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

async fn ready(config: HisConfig) -> impl IntoResponse {
    let probe = match PlatformProbe::new(config) {
        Ok(probe) => probe,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "ready": false, "error": err.to_string() })),
            )
                .into_response();
        }
    };

    let platform = probe.check().await;
    let status = if platform.ready() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
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
