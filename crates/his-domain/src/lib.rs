//! Shared types and FHIR HTTP client for Atrius HIS domain services.

mod adt;
mod clinical;
mod config;
mod documentation;
mod fhir_client;
mod narrative;
mod patient;
mod platform;
mod profiles;
mod schedule_recurrence;
mod scheduling;
mod search;

pub use adt::{
    active_bed_id, admit_transaction, bed_with_occupancy, build_ambulatory_encounter,
    build_inpatient_encounter, build_inpatient_episode_of_care, discharge_transaction,
    encounter_active_location_id, encounter_appointment_id, encounter_class_code,
    encounter_patient_id, encounter_practitioner_id, encounter_reason_text,
    finish_episode_of_care, finish_visit_transaction, is_bed_available, now_datetime,
    operational_status_code, primary_episode_of_care_id, start_visit_transaction,
    transfer_transaction,
};
pub use config::HisConfig;
pub use fhir_client::FhirClient;
pub use narrative::generate_patient_narrative;
pub use patient::{Address, BirthPlace, Telecom, build_patient, mrn_identifier, patient_display_name};
pub use platform::{PlatformHealth, PlatformProbe};
pub use documentation::{
    AnesthesiaRecordSections, ConsultNoteSections, DischargeSummarySections,
    ImmunizationRecordSections, InvoiceRecordSections, OperativeNoteSections,
    PrescriptionSections, ProcedureNoteSections, ProgressNoteSections, WellnessSections,
    anesthesia_record_transaction, anesthesia_record_update_transaction,
    build_consultation_composition, composition_encounter_id, composition_from_transaction_response,
    composition_patient_id, composition_profile, discharge_summary_transaction,
    discharge_summary_update_transaction, export_document_bundle, finalize_composition,
    finalize_consultation_composition, immunization_record_transaction,
    immunization_record_update_transaction, invoice_record_transaction,
    invoice_record_update_transaction, merge_consultation_sections, op_consult_transaction,
    op_consult_update_transaction, operative_note_transaction, operative_note_update_transaction,
    prescription_transaction, prescription_update_transaction, procedure_note_transaction,
    procedure_note_update_transaction, progress_note_transaction, progress_note_update_transaction,
    section_entry_references, wellness_record_transaction, wellness_record_update_transaction,
    LabCatalogEntry, LAB_CATALOG, build_lab_diagnostic_report, build_lab_fulfillment_task,
    build_lab_result_observation, build_lab_service_request, is_lab_diagnostic_report,
    is_lab_fulfillment_task, is_lab_service_request, lab_fulfillment_task_id,
    lab_order_place_transaction, lab_result_observation_id, lab_result_report_id,
    lab_result_transaction, lab_service_request_id, resolve_lab_display,
};
pub use profiles::{
    ATRIUS_IN_ANESTHESIA_RECORD, ATRIUS_IN_APPOINTMENT, ATRIUS_IN_COMPOSITION, ATRIUS_IN_CONDITION,
    ATRIUS_IN_DIAGNOSTIC_REPORT_LAB, ATRIUS_IN_DIAGNOSTIC_REPORT_RECORD, ATRIUS_IN_DISCHARGE_SUMMARY_RECORD,
    ATRIUS_IN_ENCOUNTER, ATRIUS_IN_EPISODE_OF_CARE, ATRIUS_IN_HEALTH_DOCUMENT_RECORD,
    ATRIUS_IN_IMMUNIZATION, ATRIUS_IN_IMMUNIZATION_RECORD, ATRIUS_IN_INPATIENT_PROCEDURE_NOTE,
    ATRIUS_IN_INPATIENT_PROGRESS_NOTE, ATRIUS_IN_INVOICE, ATRIUS_IN_INVOICE_RECORD, ATRIUS_IN_LOCATION,
    ATRIUS_IN_LOCATION_BED, ATRIUS_IN_OBSERVATION, ATRIUS_IN_OBSERVATION_BODY_MEASUREMENT,
    ATRIUS_IN_OBSERVATION_GENERAL_ASSESSMENT, ATRIUS_IN_OBSERVATION_LIFESTYLE,
    ATRIUS_IN_OBSERVATION_PHYSICAL_ACTIVITY, ATRIUS_IN_OBSERVATION_VITAL_SIGNS,
    ATRIUS_IN_OBSERVATION_WOMEN_HEALTH, ATRIUS_IN_OP_CONSULT_RECORD, ATRIUS_IN_OPERATIVE_NOTE,
    ATRIUS_IN_PATIENT, ATRIUS_IN_PRESCRIPTION_RECORD, ATRIUS_IN_SCHEDULE, ATRIUS_IN_SCHEDULE_RECURRENCE,
    ATRIUS_IN_SLOT, ATRIUS_IN_APPOINTMENT_VISIT_MODE, ATRIUS_IN_VISIT_MODE_CS,
    ATRIUS_IN_WELLNESS_RECORD, ATRIUS_MRN_SYSTEM, PATIENT_BIRTH_PLACE_EXTENSION,
};
pub use schedule_recurrence::{
    ExpandError, ScheduleRecurrence, build_schedule, build_slot, expand_schedule_slots,
    expand_slots_transaction, parse_date_or_datetime, parse_fhir_datetime,
    parse_schedule_recurrence, schedule_recurrence_extension, slot_id_for_instant,
};
pub use scheduling::{
    SlotTiming, appointment_location_id, appointment_patient_id, appointment_practitioner_id,
    appointment_slot_ids, book_appointment_transaction, build_appointment,
    cancel_appointment_transaction, reschedule_appointment_transaction, slot_timing_from_resource,
    slot_with_status,
};
pub use search::resources_from_search_bundle;
