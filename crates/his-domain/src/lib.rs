//! Shared types and FHIR HTTP client for Atrius HIS domain services.

mod adt;
mod config;
mod documentation;
mod fhir_client;
mod narrative;
mod patient;
mod platform;
mod profiles;
mod scheduling;
mod search;

pub use adt::{
    active_bed_id, admit_transaction, bed_with_occupancy, build_ambulatory_encounter,
    build_inpatient_encounter, build_inpatient_episode_of_care, discharge_transaction,
    finish_episode_of_care, is_bed_available, now_datetime, operational_status_code,
    primary_episode_of_care_id, start_visit_transaction, transfer_transaction,
};
pub use config::HisConfig;
pub use fhir_client::FhirClient;
pub use narrative::generate_patient_narrative;
pub use patient::{Address, BirthPlace, Telecom, build_patient, mrn_identifier};
pub use platform::{PlatformHealth, PlatformProbe};
pub use documentation::{
    ConsultNoteSections, build_consultation_composition, composition_encounter_id,
    composition_from_transaction_response, composition_patient_id, finalize_consultation_composition,
    merge_consultation_sections, op_consult_transaction, op_consult_update_transaction,
};
pub use profiles::{
    ATRIUS_IN_APPOINTMENT, ATRIUS_IN_COMPOSITION, ATRIUS_IN_CONDITION, ATRIUS_IN_DIAGNOSTIC_REPORT_RECORD,
    ATRIUS_IN_DISCHARGE_SUMMARY_RECORD, ATRIUS_IN_ENCOUNTER, ATRIUS_IN_EPISODE_OF_CARE,
    ATRIUS_IN_HEALTH_DOCUMENT_RECORD, ATRIUS_IN_IMMUNIZATION_RECORD, ATRIUS_IN_INPATIENT_PROCEDURE_NOTE,
    ATRIUS_IN_INPATIENT_PROGRESS_NOTE, ATRIUS_IN_LOCATION, ATRIUS_IN_LOCATION_BED, ATRIUS_IN_OBSERVATION,
    ATRIUS_IN_OP_CONSULT_RECORD, ATRIUS_IN_PATIENT, ATRIUS_IN_PRESCRIPTION_RECORD, ATRIUS_IN_SCHEDULE,
    ATRIUS_IN_SLOT, ATRIUS_IN_WELLNESS_RECORD, ATRIUS_MRN_SYSTEM, PATIENT_BIRTH_PLACE_EXTENSION,
};
pub use scheduling::{
    SlotTiming, appointment_location_id, appointment_patient_id, appointment_practitioner_id,
    appointment_slot_ids, book_appointment_transaction, build_appointment,
    cancel_appointment_transaction, reschedule_appointment_transaction, slot_timing_from_resource,
    slot_with_status,
};
pub use search::resources_from_search_bundle;
