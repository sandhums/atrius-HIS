//! Canonical Atrius India IG profile and identifier URLs.

pub const ATRIUS_IN_IG: &str = "https://atrius.in/fhir/r4/atrius-in";
#[allow(dead_code)]
pub const ATRIUS_IN_STRUCTURE_DEFINITION: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition";
pub const ATRIUS_IN_PATIENT: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-patient";
pub const ATRIUS_IN_ENCOUNTER: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-encounter";
pub const ATRIUS_IN_LOCATION: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-location";
pub const ATRIUS_IN_LOCATION_BED: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-location-bed";
pub const ATRIUS_IN_EPISODE_OF_CARE: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-episode-of-care";
pub const ATRIUS_IN_SCHEDULE: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-schedule";
pub const ATRIUS_IN_SLOT: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-slot";
pub const ATRIUS_IN_APPOINTMENT: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-appointment";
pub const ATRIUS_IN_CONDITION: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-condition";
pub const ATRIUS_IN_OBSERVATION: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-observation";
pub const ATRIUS_IN_OP_CONSULT_RECORD: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-op-consult-record";
pub const ATRIUS_IN_CONSULT_FOLLOW_UP_APPOINTMENT: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-consult-follow-up-appointment";
pub const ATRIUS_IN_DISCHARGE_SUMMARY_RECORD: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-discharge-summary-record";
pub const ATRIUS_IN_DIAGNOSTIC_REPORT_RECORD: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-diagnostic-report-record";
pub const ATRIUS_IN_HEALTH_DOCUMENT_RECORD: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-health-document-record";
pub const ATRIUS_IN_IMMUNIZATION_RECORD: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-immunization-record";
pub const ATRIUS_IN_PRESCRIPTION_RECORD: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-prescription-record";
pub const ATRIUS_IN_WELLNESS_RECORD: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-wellness-record";
pub const ATRIUS_IN_INPATIENT_PROGRESS_NOTE: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-inpatient-progress-note";
pub const ATRIUS_IN_INPATIENT_PROCEDURE_NOTE: &str =
    "https://atrius.in/fhir/r4/atrius-in/StructureDefinition/atrius-in-inpatient-procedure-note";
/// Deprecated: use [`ATRIUS_IN_OP_CONSULT_RECORD`] for OPD consultation notes.
pub const ATRIUS_IN_COMPOSITION: &str = ATRIUS_IN_OP_CONSULT_RECORD;
pub const ATRIUS_MRN_SYSTEM: &str = "https://atrius.in/fhir/r4/identifier/mrn";

/// HL7 core extension for registered place of birth (NDHM / Atrius-in-Patient slice).
pub const PATIENT_BIRTH_PLACE_EXTENSION: &str =
    "http://hl7.org/fhir/StructureDefinition/patient-birthPlace";
