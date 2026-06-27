use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use his_documentation::{
    ConsultationNoteListResponse, ConsultationNoteResponse, CreateConsultationNoteRequest,
    CreateDischargeSummaryRequest, DischargeSummaryResponse, DocumentationError,
    DocumentBundleResponse, FinalizeConsultationNoteRequest, FinalizeDischargeSummaryRequest,
    UpdateConsultationNoteRequest, UpdateDischargeSummaryRequest,
};
use serde_json::json;

use crate::request_auth::RequestAuth;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/consultation-notes", post(create_consultation_note))
        .route("/consultation-notes/{id}", get(get_consultation_note).put(update_consultation_note))
        .route(
            "/consultation-notes/{id}/finalize",
            post(finalize_consultation_note),
        )
        .route(
            "/encounters/{encounter_id}/consultation-notes",
            get(list_consultation_notes),
        )
        .route("/discharge-summaries", post(create_discharge_summary))
        .route(
            "/discharge-summaries/{id}",
            get(get_discharge_summary).put(update_discharge_summary),
        )
        .route(
            "/discharge-summaries/{id}/finalize",
            post(finalize_discharge_summary),
        )
        .route(
            "/consultation-notes/{id}/export",
            post(export_consultation_note),
        )
        .route(
            "/discharge-summaries/{id}/export",
            post(export_discharge_summary),
        )
}

async fn create_consultation_note(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Json(req): Json<CreateConsultationNoteRequest>,
) -> Result<Json<ConsultationNoteResponse>, ApiError> {
    Ok(Json(state.services(&auth).documentation.create_consultation_note(&req).await?))
}

async fn get_consultation_note(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(
        state.services(&auth).documentation.read_consultation_note(&id).await?,
    ))
}

async fn update_consultation_note(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
    Json(req): Json<UpdateConsultationNoteRequest>,
) -> Result<Json<ConsultationNoteResponse>, ApiError> {
    Ok(Json(
        state.services(&auth).documentation.update_consultation_note(&id, &req).await?,
    ))
}

async fn finalize_consultation_note(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
    Json(req): Json<FinalizeConsultationNoteRequest>,
) -> Result<Json<ConsultationNoteResponse>, ApiError> {
    Ok(Json(
        state
            .services(&auth)
            .documentation
            .finalize_consultation_note(&id, &req)
            .await?,
    ))
}

async fn list_consultation_notes(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(encounter_id): Path<String>,
) -> Result<Json<ConsultationNoteListResponse>, ApiError> {
    Ok(Json(
        state.services(&auth).documentation.list_by_encounter(&encounter_id).await?,
    ))
}

async fn create_discharge_summary(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Json(req): Json<CreateDischargeSummaryRequest>,
) -> Result<Json<DischargeSummaryResponse>, ApiError> {
    Ok(Json(state.services(&auth).documentation.create_discharge_summary(&req).await?))
}

async fn get_discharge_summary(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(state.services(&auth).documentation.read_discharge_summary(&id).await?))
}

async fn update_discharge_summary(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
    Json(req): Json<UpdateDischargeSummaryRequest>,
) -> Result<Json<DischargeSummaryResponse>, ApiError> {
    Ok(Json(
        state.services(&auth).documentation.update_discharge_summary(&id, &req).await?,
    ))
}

async fn finalize_discharge_summary(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
    Json(req): Json<FinalizeDischargeSummaryRequest>,
) -> Result<Json<DischargeSummaryResponse>, ApiError> {
    Ok(Json(
        state
            .services(&auth)
            .documentation
            .finalize_discharge_summary(&id, &req)
            .await?,
    ))
}

async fn export_consultation_note(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
) -> Result<Json<DocumentBundleResponse>, ApiError> {
    Ok(Json(state.services(&auth).documentation.export_document_bundle(&id).await?))
}

async fn export_discharge_summary(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
) -> Result<Json<DocumentBundleResponse>, ApiError> {
    Ok(Json(state.services(&auth).documentation.export_document_bundle(&id).await?))
}

#[derive(Debug)]
pub(crate) struct ApiError(DocumentationError);

impl From<DocumentationError> for ApiError {
    fn from(value: DocumentationError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match &self.0 {
            DocumentationError::InvalidRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid_request", "message": msg })),
            )
                .into_response(),
            DocumentationError::DraftNoteExists {
                encounter_id,
                composition_id,
            } => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "draft_note_exists",
                    "message": self.0.to_string(),
                    "encounter_id": encounter_id,
                    "composition_id": composition_id
                })),
            )
                .into_response(),
            DocumentationError::EncounterNotActive(status) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "encounter_not_active",
                    "message": self.0.to_string(),
                    "status": status
                })),
            )
                .into_response(),
            DocumentationError::EncounterNotInpatient(class_code) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "encounter_not_inpatient",
                    "message": self.0.to_string(),
                    "class": class_code
                })),
            )
                .into_response(),
            DocumentationError::CompositionNotEditable(status) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "composition_not_editable",
                    "message": self.0.to_string(),
                    "status": status
                })),
            )
                .into_response(),
            DocumentationError::CompositionNotPreliminary(status) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "composition_not_preliminary",
                    "message": self.0.to_string(),
                    "status": status
                })),
            )
                .into_response(),
            DocumentationError::CompositionNotFinal(status) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "composition_not_final",
                    "message": self.0.to_string(),
                    "status": status
                })),
            )
                .into_response(),
            DocumentationError::EncounterNotFound(id)
            | DocumentationError::CompositionNotFound(id)
            | DocumentationError::PractitionerNotFound(id) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "not_found", "message": id })),
            )
                .into_response(),
            DocumentationError::Fhir(err) => (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "fhir_error", "message": err.to_string() })),
            )
                .into_response(),
        }
    }
}
