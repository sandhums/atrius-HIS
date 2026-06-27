//! Clinical document builders: Composition records, entry resources, and NDHM export.

pub mod document_bundle;
pub mod entry_builders;
pub mod orders;
pub mod lifecycle;
pub mod slice;
pub mod specs;
pub mod transaction;

pub use document_bundle::{export_document_bundle, section_entry_references};
pub use lifecycle::{
    composition_encounter_id, composition_from_transaction_response, composition_patient_id,
    composition_profile, finalize_composition,
};
pub use specs::discharge_summary::{
    DischargeSummarySections, discharge_summary_transaction, discharge_summary_update_transaction,
};
pub use specs::op_consult::{
    ConsultNoteSections, build_consultation_composition, merge_consultation_sections,
    op_consult_transaction, op_consult_update_transaction,
};
pub use specs::progress_note::{
    ProgressNoteSections, progress_note_transaction, progress_note_update_transaction,
};
pub use specs::procedure_note::{
    ProcedureNoteSections, procedure_note_transaction, procedure_note_update_transaction,
};
pub use specs::operative_note::{
    OperativeNoteSections, operative_note_transaction, operative_note_update_transaction,
};
pub use specs::anesthesia_record::{
    AnesthesiaRecordSections, anesthesia_record_transaction, anesthesia_record_update_transaction,
};
pub use specs::prescription::{
    PrescriptionSections, prescription_transaction, prescription_update_transaction,
};
pub use specs::wellness::{
    WellnessSections, wellness_record_transaction, wellness_record_update_transaction,
};
pub use specs::immunization_record::{
    ImmunizationRecordSections, immunization_record_transaction,
    immunization_record_update_transaction,
};
pub use specs::invoice_record::{
    InvoiceRecordSections, invoice_record_transaction, invoice_record_update_transaction,
};
pub use orders::{
    LabCatalogEntry, LAB_CATALOG, build_lab_diagnostic_report, build_lab_fulfillment_task,
    build_lab_result_observation, build_lab_service_request, is_lab_diagnostic_report,
    is_lab_fulfillment_task, is_lab_service_request, lab_fulfillment_task_id,
    lab_order_place_transaction, lab_result_observation_id, lab_result_report_id,
    lab_result_transaction, lab_service_request_id, resolve_lab_display,
};

/// Backward-compatible alias.
#[must_use]
pub fn finalize_consultation_composition(composition: &Value, practitioner_id: &str) -> Value {
    finalize_composition(composition, practitioner_id)
}

use serde_json::Value;
