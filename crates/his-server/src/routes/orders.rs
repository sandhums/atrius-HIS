use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use his_orders::{
    LabCatalogResponse, LabOrderResponse, ListLabOrdersResponse, ListLabResultsResponse,
    ListLabTasksResponse, OrderError, PlaceLabOrderRequest, PostLabResultRequest,
    PostLabResultResponse, RevokeLabOrderResponse,
};
use serde_json::json;

use crate::request_auth::RequestAuth;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/lab-catalog", get(lab_catalog))
        .route("/lab-orders", post(place_lab_order))
        .route("/lab-orders/{id}", get(read_lab_order))
        .route("/lab-orders/{id}/revoke", post(revoke_lab_order))
        .route("/lab-orders/{id}/result", post(post_lab_result))
        .route("/encounters/{id}/lab-orders", get(list_lab_orders))
        .route("/encounters/{id}/lab-tasks", get(list_lab_tasks))
        .route("/encounters/{id}/lab-results", get(list_lab_results))
}

async fn lab_catalog(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
) -> Json<LabCatalogResponse> {
    Json(state.services(&auth).orders.lab_catalog())
}

async fn place_lab_order(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Json(req): Json<PlaceLabOrderRequest>,
) -> Result<Json<LabOrderResponse>, ApiError> {
    Ok(Json(state.services(&auth).orders.place_lab_order(&req).await?))
}

async fn list_lab_orders(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(encounter_id): Path<String>,
) -> Result<Json<ListLabOrdersResponse>, ApiError> {
    Ok(Json(state.services(&auth).orders.list_lab_orders(&encounter_id).await?))
}

async fn list_lab_tasks(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(encounter_id): Path<String>,
) -> Result<Json<ListLabTasksResponse>, ApiError> {
    Ok(Json(state.services(&auth).orders.list_lab_tasks(&encounter_id).await?))
}

async fn list_lab_results(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(encounter_id): Path<String>,
) -> Result<Json<ListLabResultsResponse>, ApiError> {
    Ok(Json(state.services(&auth).orders.list_lab_results(&encounter_id).await?))
}

async fn read_lab_order(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(state.services(&auth).orders.read_lab_order(&id).await?))
}

async fn revoke_lab_order(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
) -> Result<Json<RevokeLabOrderResponse>, ApiError> {
    Ok(Json(state.services(&auth).orders.revoke_lab_order(&id).await?))
}

async fn post_lab_result(
    State(state): State<Arc<AppState>>,
    auth: RequestAuth,
    Path(id): Path<String>,
    Json(req): Json<PostLabResultRequest>,
) -> Result<Json<PostLabResultResponse>, ApiError> {
    Ok(Json(state.services(&auth).orders.post_lab_result(&id, &req).await?))
}

#[derive(Debug)]
struct ApiError(OrderError);

impl From<OrderError> for ApiError {
    fn from(value: OrderError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match &self.0 {
            OrderError::InvalidRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid_request", "message": msg })),
            )
                .into_response(),
            OrderError::EncounterNotFound(id) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "encounter_not_found", "message": id })),
            )
                .into_response(),
            OrderError::EncounterNotActive(status) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "encounter_not_active",
                    "message": self.0.to_string(),
                    "status": status
                })),
            )
                .into_response(),
            OrderError::LabOrderNotFound(id) | OrderError::LabTaskNotFound(id) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "not_found", "message": id })),
            )
                .into_response(),
            OrderError::LabOrderNotActive(status) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "lab_order_not_active",
                    "message": self.0.to_string(),
                    "status": status
                })),
            )
                .into_response(),
            OrderError::UnknownLoincCode(code) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "unknown_loinc_code",
                    "message": self.0.to_string(),
                    "loinc_code": code
                })),
            )
                .into_response(),
            OrderError::Fhir(err) => (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "fhir_error", "message": err.to_string() })),
            )
                .into_response(),
        }
    }
}
