//! Clinical document kind discriminator for workflow and profile dispatch.

use his_domain::{
    ATRIUS_IN_ANESTHESIA_RECORD, ATRIUS_IN_DISCHARGE_SUMMARY_RECORD,
    ATRIUS_IN_IMMUNIZATION_RECORD, ATRIUS_IN_INPATIENT_PROCEDURE_NOTE,
    ATRIUS_IN_INPATIENT_PROGRESS_NOTE, ATRIUS_IN_INVOICE_RECORD,
    ATRIUS_IN_OP_CONSULT_RECORD, ATRIUS_IN_OPERATIVE_NOTE, ATRIUS_IN_PRESCRIPTION_RECORD,
    ATRIUS_IN_WELLNESS_RECORD,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClinicalDocumentKind {
    OpConsult,
    DischargeSummary,
    ProgressNote,
    ProcedureNote,
    OperativeNote,
    AnesthesiaRecord,
    Prescription,
    Wellness,
    ImmunizationRecord,
    InvoiceRecord,
}

impl ClinicalDocumentKind {
    pub fn profile_url(self) -> &'static str {
        match self {
            Self::OpConsult => ATRIUS_IN_OP_CONSULT_RECORD,
            Self::DischargeSummary => ATRIUS_IN_DISCHARGE_SUMMARY_RECORD,
            Self::ProgressNote => ATRIUS_IN_INPATIENT_PROGRESS_NOTE,
            Self::ProcedureNote => ATRIUS_IN_INPATIENT_PROCEDURE_NOTE,
            Self::OperativeNote => ATRIUS_IN_OPERATIVE_NOTE,
            Self::AnesthesiaRecord => ATRIUS_IN_ANESTHESIA_RECORD,
            Self::Prescription => ATRIUS_IN_PRESCRIPTION_RECORD,
            Self::Wellness => ATRIUS_IN_WELLNESS_RECORD,
            Self::ImmunizationRecord => ATRIUS_IN_IMMUNIZATION_RECORD,
            Self::InvoiceRecord => ATRIUS_IN_INVOICE_RECORD,
        }
    }

    pub fn draft_conflict_label(self) -> &'static str {
        match self {
            Self::OpConsult => "consultation note",
            Self::DischargeSummary => "discharge summary",
            Self::ProgressNote => "progress note",
            Self::ProcedureNote => "procedure note",
            Self::OperativeNote => "operative note",
            Self::AnesthesiaRecord => "anesthesia record",
            Self::Prescription => "prescription record",
            Self::Wellness => "wellness record",
            Self::ImmunizationRecord => "immunization record",
            Self::InvoiceRecord => "invoice record",
        }
    }

    pub fn api_path(self) -> &'static str {
        match self {
            Self::OpConsult => "consultation-notes",
            Self::DischargeSummary => "discharge-summaries",
            Self::ProgressNote => "progress-notes",
            Self::ProcedureNote => "procedure-notes",
            Self::OperativeNote => "operative-notes",
            Self::AnesthesiaRecord => "anesthesia-records",
            Self::Prescription => "prescription-records",
            Self::Wellness => "wellness-records",
            Self::ImmunizationRecord => "immunization-records",
            Self::InvoiceRecord => "invoice-records",
        }
    }

    pub fn requires_inpatient_encounter(self) -> bool {
        matches!(
            self,
            Self::DischargeSummary
                | Self::ProgressNote
                | Self::ProcedureNote
                | Self::OperativeNote
                | Self::AnesthesiaRecord
        )
    }
}
