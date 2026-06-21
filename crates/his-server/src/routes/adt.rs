use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use his_adt::{
    AdmitPatientRequest, AdmitPatientResponse, AdtError, BedBoardQuery, BedBoardResponse,
    DischargePatientRequest, StartVisitRequest, StartVisitResponse, TransferPatientRequest,
};
use serde_json::json;

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/encounters/admit", post(admit_patient))
        .route("/encounters/start-visit", post(start_visit))
        .route("/encounters/{id}", get(get_encounter))
        .route("/encounters/{id}/transfer", post(transfer_patient))
        .route("/encounters/{id}/discharge", post(discharge_patient))
        .route("/bed-board", get(bed_board))
}

async fn admit_patient(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AdmitPatientRequest>,
) -> Result<Json<AdmitPatientResponse>, ApiError> {
    Ok(Json(state.adt.admit(&req).await?))
}

async fn start_visit(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartVisitRequest>,
) -> Result<Json<StartVisitResponse>, ApiError> {
    Ok(Json(state.adt.start_visit(&req).await?))
}

async fn get_encounter(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(state.adt.read_encounter(&id).await?))
}

async fn transfer_patient(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<TransferPatientRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(state.adt.transfer(&id, &req).await?))
}

async fn discharge_patient(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<DischargePatientRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(state.adt.discharge(&id, &req).await?))
}

async fn bed_board(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BedBoardQuery>,
) -> Result<Json<BedBoardResponse>, ApiError> {
    Ok(Json(state.adt.bed_board(&query).await?))
}

#[derive(Debug)]
struct ApiError(AdtError);

impl From<AdtError> for ApiError {
    fn from(value: AdtError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match &self.0 {
            AdtError::InvalidRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid_request", "message": msg })),
            )
                .into_response(),
            AdtError::BedNotAvailable { bed_id } => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "bed_not_available",
                    "message": self.0.to_string(),
                    "bed_id": bed_id
                })),
            )
                .into_response(),
            AdtError::EncounterNotActive(status) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "encounter_not_active",
                    "message": self.0.to_string(),
                    "status": status
                })),
            )
                .into_response(),
            AdtError::AppointmentNotBookable {
                appointment_id,
                status,
            } => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "appointment_not_bookable",
                    "message": self.0.to_string(),
                    "appointment_id": appointment_id,
                    "status": status
                })),
            )
                .into_response(),
            AdtError::VisitAlreadyStarted {
                appointment_id,
                encounter_id,
            } => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "visit_already_started",
                    "message": self.0.to_string(),
                    "appointment_id": appointment_id,
                    "encounter_id": encounter_id
                })),
            )
                .into_response(),
            AdtError::EncounterNotFound(id)
            | AdtError::PatientNotFound(id)
            | AdtError::BedNotFound(id)
            | AdtError::AppointmentNotFound(id) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "not_found", "message": id })),
            )
                .into_response(),
            AdtError::Fhir(err) => (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "fhir_error", "message": err.to_string() })),
            )
                .into_response(),
        }
    }
}
