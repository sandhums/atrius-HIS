mod error;
mod service;

pub use error::DocumentationError;
pub use service::{
    ConsultationNoteListResponse, ConsultationNoteResponse, CreateConsultationNoteRequest,
    DocumentationService, FinalizeConsultationNoteRequest, UpdateConsultationNoteRequest,
};
