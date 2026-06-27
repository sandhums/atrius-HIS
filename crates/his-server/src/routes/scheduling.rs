use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use his_scheduling::{
    BookAppointmentRequest, BookAppointmentResponse, CancelAppointmentRequest, ExpandSlotsQuery,
    ExpandSlotsResponse, FindSlotsQuery, FindSlotsResponse, ListBookingDoctorsResponse,
    PractitionerAppointmentsQuery, PractitionerAppointmentsResponse, RescheduleAppointmentRequest,
    SchedulingError,
};
use serde_json::json;

use crate::request_auth::RequestAuth;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/booking-doctors", get(list_booking_doctors))
        .route("/slots", get(find_slots))
        .route("/schedules/{id}/expand-slots", post(expand_schedule_slots))
        .route("/appointments", post(book_appointment))
        .route("/appointments/{id}", get(get_appointment))
        .route("/appointments/{id}/cancel", post(cancel_appointment))
        .route("/appointments/{id}/reschedule", post(reschedule_appointment))
        .route("/practitioners/{id}/appointments", get(list_practitioner_appointments))
}

async fn list_booking_doctors(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
) -> Result<Json<ListBookingDoctorsResponse>, ApiError> {
    let response = state.services(&auth).scheduling.list_booking_doctors().await?;
    Ok(Json(response))
}

async fn find_slots(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Query(query): Query<FindSlotsQuery>,
) -> Result<Json<FindSlotsResponse>, ApiError> {
    let response = state.services(&auth).scheduling.find_available_slots(&query).await?;
    Ok(Json(response))
}

async fn expand_schedule_slots(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
    Query(query): Query<ExpandSlotsQuery>,
) -> Result<Json<ExpandSlotsResponse>, ApiError> {
    let response = state
        .services(&auth)
        .scheduling
        .expand_schedule_slots(&id, &query)
        .await?;
    Ok(Json(response))
}

async fn book_appointment(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Json(req): Json<BookAppointmentRequest>,
) -> Result<Json<BookAppointmentResponse>, ApiError> {
    let response = state.services(&auth).scheduling.book_appointment(&req).await?;
    Ok(Json(response))
}

async fn get_appointment(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let appointment = state.services(&auth).scheduling.read_appointment(&id).await?;
    Ok(Json(appointment))
}

async fn cancel_appointment(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
    Json(req): Json<CancelAppointmentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let appointment = state.services(&auth).scheduling.cancel_appointment(&id, &req).await?;
    Ok(Json(appointment))
}

async fn reschedule_appointment(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
    Json(req): Json<RescheduleAppointmentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let appointment = state.services(&auth).scheduling.reschedule_appointment(&id, &req).await?;
    Ok(Json(appointment))
}

async fn list_practitioner_appointments(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
    Query(query): Query<PractitionerAppointmentsQuery>,
) -> Result<Json<PractitionerAppointmentsResponse>, ApiError> {
    let response = state
        .services(&auth)
        .scheduling
        .list_practitioner_appointments(&id, &query)
        .await?;
    Ok(Json(response))
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
            SchedulingError::ScheduleNotFound(id) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "schedule_not_found", "message": id })),
            )
                .into_response(),
            SchedulingError::Expand(msg) => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "expand_failed", "message": msg })),
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
