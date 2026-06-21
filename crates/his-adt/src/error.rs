use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum AdtError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("bed not available: {bed_id}")]
    BedNotAvailable { bed_id: String },
    #[error("encounter not found: {0}")]
    EncounterNotFound(String),
    #[error("encounter is not active (status={0})")]
    EncounterNotActive(String),
    #[error("patient not found: {0}")]
    PatientNotFound(String),
    #[error("bed not found: {0}")]
    BedNotFound(String),
    #[error("appointment not found: {0}")]
    AppointmentNotFound(String),
    #[error("appointment is not bookable (status={status})")]
    AppointmentNotBookable { appointment_id: String, status: String },
    #[error("visit already started for appointment {appointment_id} (encounter={encounter_id})")]
    VisitAlreadyStarted {
        appointment_id: String,
        encounter_id: String,
    },
    #[error("FHIR error: {0}")]
    Fhir(#[from] anyhow::Error),
}

impl AdtError {
    pub fn from_fhir(err: anyhow::Error) -> Self {
        Self::Fhir(err)
    }
}

pub fn encounter_from_transaction_response(response: &Value) -> Option<Value> {
    resource_from_transaction_response(response, "Encounter")
}

pub fn episode_from_transaction_response(response: &Value) -> Option<Value> {
    resource_from_transaction_response(response, "EpisodeOfCare")
}

fn resource_from_transaction_response(response: &Value, resource_type: &str) -> Option<Value> {
    response
        .get("entry")
        .and_then(|e| e.as_array())
        .and_then(|entries| {
            entries.iter().find_map(|entry| {
                let resource = entry.get("resource")?;
                if resource.get("resourceType")?.as_str()? == resource_type {
                    Some(resource.clone())
                } else {
                    None
                }
            })
        })
}
