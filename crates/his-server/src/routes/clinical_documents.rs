//! REST routes for Phase 5d clinical document types.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use his_documentation::{
    ClinicalDocumentResponse, CreateAnesthesiaRecordRequest, CreateImmunizationRecordRequest,
    CreateInvoiceRecordRequest, CreateOperativeNoteRequest, CreatePrescriptionRequest,
    CreateProcedureNoteRequest, CreateProgressNoteRequest, CreateWellnessRecordRequest,
    DocumentBundleResponse, FinalizeClinicalDocumentRequest, UpdateAnesthesiaRecordRequest,
    UpdateImmunizationRecordRequest, UpdateInvoiceRecordRequest, UpdateOperativeNoteRequest,
    UpdatePrescriptionRequest, UpdateProcedureNoteRequest, UpdateProgressNoteRequest,
    UpdateWellnessRecordRequest,
};

use super::documentation::ApiError;
use crate::request_auth::RequestAuth;
use crate::state::AppState;

macro_rules! clinical_document_routes {
    ($base:literal, $create:ident, $get:ident, $update:ident, $finalize:ident, $export:ident,
     $create_req:ty, $update_req:ty) => {
        Router::new()
            .route($base, post($create))
            .route(concat!($base, "/{id}"), get($get).put($update))
            .route(concat!($base, "/{id}/finalize"), post($finalize))
            .route(concat!($base, "/{id}/export"), post($export))
    };
}

pub fn routes() -> Router<Arc<AppState>> {
    clinical_document_routes!(
        "/progress-notes",
        create_progress_note,
        get_progress_note,
        update_progress_note,
        finalize_progress_note,
        export_progress_note,
        CreateProgressNoteRequest,
        UpdateProgressNoteRequest
    )
    .merge(clinical_document_routes!(
        "/procedure-notes",
        create_procedure_note,
        get_procedure_note,
        update_procedure_note,
        finalize_procedure_note,
        export_procedure_note,
        CreateProcedureNoteRequest,
        UpdateProcedureNoteRequest
    ))
    .merge(clinical_document_routes!(
        "/operative-notes",
        create_operative_note,
        get_operative_note,
        update_operative_note,
        finalize_operative_note,
        export_operative_note,
        CreateOperativeNoteRequest,
        UpdateOperativeNoteRequest
    ))
    .merge(clinical_document_routes!(
        "/anesthesia-records",
        create_anesthesia_record,
        get_anesthesia_record,
        update_anesthesia_record,
        finalize_anesthesia_record,
        export_anesthesia_record,
        CreateAnesthesiaRecordRequest,
        UpdateAnesthesiaRecordRequest
    ))
    .merge(clinical_document_routes!(
        "/prescription-records",
        create_prescription_record,
        get_prescription_record,
        update_prescription_record,
        finalize_prescription_record,
        export_prescription_record,
        CreatePrescriptionRequest,
        UpdatePrescriptionRequest
    ))
    .merge(clinical_document_routes!(
        "/wellness-records",
        create_wellness_record,
        get_wellness_record,
        update_wellness_record,
        finalize_wellness_record,
        export_wellness_record,
        CreateWellnessRecordRequest,
        UpdateWellnessRecordRequest
    ))
    .merge(clinical_document_routes!(
        "/immunization-records",
        create_immunization_record,
        get_immunization_record,
        update_immunization_record,
        finalize_immunization_record,
        export_immunization_record,
        CreateImmunizationRecordRequest,
        UpdateImmunizationRecordRequest
    ))
    .merge(clinical_document_routes!(
        "/invoice-records",
        create_invoice_record,
        get_invoice_record,
        update_invoice_record,
        finalize_invoice_record,
        export_invoice_record,
        CreateInvoiceRecordRequest,
        UpdateInvoiceRecordRequest
    ))
}

macro_rules! handlers {
    ($create:ident, $get:ident, $update:ident, $finalize:ident, $export:ident,
     $create_method:ident, $read_method:ident, $update_method:ident, $finalize_method:ident,
     $create_req:ty, $update_req:ty) => {
        async fn $create(
            State(state): State<Arc<AppState>>,
            auth: RequestAuth,
            Json(req): Json<$create_req>,
        ) -> Result<Json<ClinicalDocumentResponse>, ApiError> {
            Ok(Json(
                state.services(&auth).documentation.$create_method(&req).await?,
            ))
        }

        async fn $get(
            State(state): State<Arc<AppState>>,
            auth: RequestAuth,
            Path(id): Path<String>,
        ) -> Result<Json<serde_json::Value>, ApiError> {
            Ok(Json(
                state.services(&auth).documentation.$read_method(&id).await?,
            ))
        }

        async fn $update(
            State(state): State<Arc<AppState>>,
            auth: RequestAuth,
            Path(id): Path<String>,
            Json(req): Json<$update_req>,
        ) -> Result<Json<ClinicalDocumentResponse>, ApiError> {
            Ok(Json(
                state
                    .services(&auth)
                    .documentation
                    .$update_method(&id, &req)
                    .await?,
            ))
        }

        async fn $finalize(
            State(state): State<Arc<AppState>>,
            auth: RequestAuth,
            Path(id): Path<String>,
            Json(req): Json<FinalizeClinicalDocumentRequest>,
        ) -> Result<Json<ClinicalDocumentResponse>, ApiError> {
            Ok(Json(
                state
                    .services(&auth)
                    .documentation
                    .$finalize_method(&id, &req)
                    .await?,
            ))
        }

        async fn $export(
            State(state): State<Arc<AppState>>,
            auth: RequestAuth,
            Path(id): Path<String>,
        ) -> Result<Json<DocumentBundleResponse>, ApiError> {
            Ok(Json(
                state
                    .services(&auth)
                    .documentation
                    .export_document_bundle(&id)
                    .await?,
            ))
        }
    };
}

handlers!(
    create_progress_note,
    get_progress_note,
    update_progress_note,
    finalize_progress_note,
    export_progress_note,
    create_progress_note,
    read_progress_note,
    update_progress_note,
    finalize_progress_note,
    CreateProgressNoteRequest,
    UpdateProgressNoteRequest
);
handlers!(
    create_procedure_note,
    get_procedure_note,
    update_procedure_note,
    finalize_procedure_note,
    export_procedure_note,
    create_procedure_note,
    read_procedure_note,
    update_procedure_note,
    finalize_procedure_note,
    CreateProcedureNoteRequest,
    UpdateProcedureNoteRequest
);
handlers!(
    create_operative_note,
    get_operative_note,
    update_operative_note,
    finalize_operative_note,
    export_operative_note,
    create_operative_note,
    read_operative_note,
    update_operative_note,
    finalize_operative_note,
    CreateOperativeNoteRequest,
    UpdateOperativeNoteRequest
);
handlers!(
    create_anesthesia_record,
    get_anesthesia_record,
    update_anesthesia_record,
    finalize_anesthesia_record,
    export_anesthesia_record,
    create_anesthesia_record,
    read_anesthesia_record,
    update_anesthesia_record,
    finalize_anesthesia_record,
    CreateAnesthesiaRecordRequest,
    UpdateAnesthesiaRecordRequest
);
handlers!(
    create_prescription_record,
    get_prescription_record,
    update_prescription_record,
    finalize_prescription_record,
    export_prescription_record,
    create_prescription_record,
    read_prescription_record,
    update_prescription_record,
    finalize_prescription_record,
    CreatePrescriptionRequest,
    UpdatePrescriptionRequest
);
handlers!(
    create_wellness_record,
    get_wellness_record,
    update_wellness_record,
    finalize_wellness_record,
    export_wellness_record,
    create_wellness_record,
    read_wellness_record,
    update_wellness_record,
    finalize_wellness_record,
    CreateWellnessRecordRequest,
    UpdateWellnessRecordRequest
);
handlers!(
    create_immunization_record,
    get_immunization_record,
    update_immunization_record,
    finalize_immunization_record,
    export_immunization_record,
    create_immunization_record,
    read_immunization_record,
    update_immunization_record,
    finalize_immunization_record,
    CreateImmunizationRecordRequest,
    UpdateImmunizationRecordRequest
);
handlers!(
    create_invoice_record,
    get_invoice_record,
    update_invoice_record,
    finalize_invoice_record,
    export_invoice_record,
    create_invoice_record,
    read_invoice_record,
    update_invoice_record,
    finalize_invoice_record,
    CreateInvoiceRecordRequest,
    UpdateInvoiceRecordRequest
);
