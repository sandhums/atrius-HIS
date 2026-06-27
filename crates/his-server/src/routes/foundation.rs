use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, patch};
use axum::{Json, Router};
use his_foundation::{FoundationConfigResponse, OrganizationSummary, UpdateOrganizationRequest};
use serde_json::json;

use crate::request_auth::RequestAuth;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/foundation", get(get_foundation))
        .route("/foundation/organization", patch(update_organization))
}

async fn get_foundation(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
) -> Result<Json<FoundationConfigResponse>, ApiError> {
    Ok(Json(state.services(&auth).foundation.get_config().await?))
}

async fn update_organization(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Json(body): Json<UpdateOrganizationRequest>,
) -> Result<Json<OrganizationSummary>, ApiError> {
    let org = state.services(&auth).foundation.get_config().await?.organization;
    let updated = state
        .services(&auth)
        .foundation
        .update_organization_name(&org.id, &body)
        .await?;
    Ok(Json(updated))
}

#[derive(Debug)]
struct ApiError(his_foundation::FoundationError);

impl From<his_foundation::FoundationError> for ApiError {
    fn from(value: his_foundation::FoundationError) -> Self {
        Self(value)
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        let (status, code) = match &self.0 {
            his_foundation::FoundationError::InvalidRequest(_) => {
                (StatusCode::BAD_REQUEST, "invalid_request")
            }
            his_foundation::FoundationError::OrganizationNotFound => {
                (StatusCode::NOT_FOUND, "not_found")
            }
            his_foundation::FoundationError::Fhir(_) => (StatusCode::BAD_GATEWAY, "fhir_error"),
        };
        (
            status,
            Json(json!({ "error": code, "message": self.0.to_string() })),
        )
            .into_response()
    }
}
