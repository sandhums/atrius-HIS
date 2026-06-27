mod clinical_documents;
mod error;
mod helpers;
mod kinds;
mod service;

pub use clinical_documents::*;
pub use error::DocumentationError;
pub use kinds::ClinicalDocumentKind;
pub use service::{
    ConsultationNoteListResponse, ConsultationNoteResponse, CreateConsultationNoteRequest,
    CreateDischargeSummaryRequest, DischargeSummaryResponse, DocumentationService,
    DocumentBundleResponse, FinalizeConsultationNoteRequest, FinalizeDischargeSummaryRequest,
    UpdateConsultationNoteRequest, UpdateDischargeSummaryRequest,
};
