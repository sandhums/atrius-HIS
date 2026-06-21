//! Patient registration domain service.

mod error;
mod mrn;
mod service;

pub use error::RegistrationError;
pub use mrn::generate_mrn;
pub use service::{
    DuplicateMatch, DuplicateSummary, RegisterPatientRequest, RegisterPatientResponse,
    RegistrationService,
};
