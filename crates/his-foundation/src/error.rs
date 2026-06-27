use thiserror::Error;

#[derive(Debug, Error)]
pub enum FoundationError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("organization not found")]
    OrganizationNotFound,
    #[error("FHIR error: {0}")]
    Fhir(#[from] anyhow::Error),
}
