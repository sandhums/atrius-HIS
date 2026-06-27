use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use his_registration::{
    DuplicateSummary, RegisterPatientRequest, RegisterPatientResponse, RegistrationError,
};
use serde_json::json;

use crate::request_auth::RequestAuth;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/patients", post(register_patient))
        .route("/patients/check-duplicates", post(check_duplicates))
        .route("/patients/{id}", get(get_patient))
}

async fn register_patient(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Json(req): Json<RegisterPatientRequest>,
) -> Result<Json<RegisterPatientResponse>, ApiError> {
    let response = state.services(&auth).registration.register(req).await?;
    Ok(Json(response))
}

async fn check_duplicates(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Json(req): Json<RegisterPatientRequest>,
) -> Result<Json<DuplicateSummary>, ApiError> {
    let summary = state
        .services(&auth)
        .registration
        .check_duplicates(&req)
        .await?;
    Ok(Json(summary))
}

async fn get_patient(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let patient = state.services(&auth).registration.read_patient(&id).await?;
    Ok(Json(patient))
}

#[derive(Debug)]
struct ApiError(RegistrationError);

impl From<RegistrationError> for ApiError {
    fn from(value: RegistrationError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match &self.0 {
            RegistrationError::Duplicate { matches } => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "duplicate_patient",
                    "message": self.0.to_string(),
                    "duplicates": matches
                })),
            )
                .into_response(),
            RegistrationError::InvalidRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid_request", "message": msg })),
            )
                .into_response(),
            RegistrationError::Fhir(err) => (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "fhir_error", "message": err.to_string() })),
            )
                .into_response(),
        }
    }
}
