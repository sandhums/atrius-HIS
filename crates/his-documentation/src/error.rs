use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum DocumentationError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("encounter not found: {0}")]
    EncounterNotFound(String),
    #[error("encounter is not active (status={0})")]
    EncounterNotActive(String),
    #[error("encounter is not inpatient (class={0})")]
    EncounterNotInpatient(String),
    #[error("composition not found: {0}")]
    CompositionNotFound(String),
    #[error("draft note already exists for encounter {encounter_id} (composition={composition_id})")]
    DraftNoteExists {
        encounter_id: String,
        composition_id: String,
    },
    #[error("composition is not editable (status={0})")]
    CompositionNotEditable(String),
    #[error("composition is not preliminary (status={0})")]
    CompositionNotPreliminary(String),
    #[error("composition is not final (status={0})")]
    CompositionNotFinal(String),
    #[error("practitioner not found: {0}")]
    PractitionerNotFound(String),
    #[error("FHIR error: {0}")]
    Fhir(#[from] anyhow::Error),
}

impl DocumentationError {
    pub fn from_fhir(err: anyhow::Error) -> Self {
        Self::Fhir(err)
    }
}

pub fn practitioner_id_from_composition(composition: &Value) -> Option<String> {
    composition
        .get("author")
        .and_then(|a| a.as_array())
        .and_then(|authors| authors.first())
        .and_then(|author| author.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("Practitioner/"))
        .map(str::to_string)
}
