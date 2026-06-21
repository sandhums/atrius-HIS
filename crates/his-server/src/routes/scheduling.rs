use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use his_scheduling::{
    BookAppointmentRequest, BookAppointmentResponse, CancelAppointmentRequest, FindSlotsQuery,
    FindSlotsResponse, RescheduleAppointmentRequest, SchedulingError,
};
use serde_json::json;

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/slots", get(find_slots))
        .route("/appointments", post(book_appointment))
        .route("/appointments/{id}", get(get_appointment))
        .route("/appointments/{id}/cancel", post(cancel_appointment))
        .route("/appointments/{id}/reschedule", post(reschedule_appointment))
}

async fn find_slots(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FindSlotsQuery>,
) -> Result<Json<FindSlotsResponse>, ApiError> {
    let response = state.scheduling.find_available_slots(&query).await?;
    Ok(Json(response))
}

async fn book_appointment(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BookAppointmentRequest>,
) -> Result<Json<BookAppointmentResponse>, ApiError> {
    let response = state.scheduling.book_appointment(&req).await?;
    Ok(Json(response))
}

async fn get_appointment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let appointment = state.scheduling.read_appointment(&id).await?;
    Ok(Json(appointment))
}

async fn cancel_appointment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CancelAppointmentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let appointment = state.scheduling.cancel_appointment(&id, &req).await?;
    Ok(Json(appointment))
}

async fn reschedule_appointment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<RescheduleAppointmentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let appointment = state.scheduling.reschedule_appointment(&id, &req).await?;
    Ok(Json(appointment))
}

#[derive(Debug)]
struct ApiError(SchedulingError);

impl From<SchedulingError> for ApiError {
    fn from(value: SchedulingError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match &self.0 {
            SchedulingError::InvalidRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid_request", "message": msg })),
            )
                .into_response(),
            SchedulingError::SlotNotAvailable { slot_id, status } => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "slot_not_available",
                    "message": self.0.to_string(),
                    "slot_id": slot_id,
                    "status": status
                })),
            )
                .into_response(),
            SchedulingError::AppointmentNotFound(id) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "appointment_not_found", "message": id })),
            )
                .into_response(),
            SchedulingError::AppointmentNotActive(status) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "appointment_not_active",
                    "message": self.0.to_string(),
                    "status": status
                })),
            )
                .into_response(),
            SchedulingError::PatientNotFound(id) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "patient_not_found", "message": id })),
            )
                .into_response(),
            SchedulingError::Fhir(err) => (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "fhir_error", "message": err.to_string() })),
            )
                .into_response(),
        }
    }
}
