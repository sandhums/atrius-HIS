use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("duplicate patient(s) found")]
    Duplicate { matches: Vec<crate::DuplicateMatch> },
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error(transparent)]
    Fhir(#[from] anyhow::Error),
}
