use his_domain::resources_from_search_bundle;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum SchedulingError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("slot not available")]
    SlotNotAvailable { slot_id: String, status: String },
    #[error("appointment not found")]
    AppointmentNotFound(String),
    #[error("appointment is not active (status={0})")]
    AppointmentNotActive(String),
    #[error("patient not found: {0}")]
    PatientNotFound(String),
    #[error("FHIR error: {0}")]
    Fhir(#[from] anyhow::Error),
}

impl SchedulingError {
    pub fn from_fhir(err: anyhow::Error) -> Self {
        Self::Fhir(err)
    }
}

pub fn resource_from_transaction_response(response: &Value) -> Option<Value> {
    response
        .get("entry")
        .and_then(|e| e.as_array())
        .and_then(|entries| {
            entries.iter().find_map(|entry| {
                entry
                    .get("response")
                    .and_then(|r| r.get("location"))
                    .and_then(|loc| loc.as_str())
                    .filter(|loc| loc.contains("Appointment/"))
                    .and_then(|_| entry.get("resource").cloned())
            })
        })
        .or_else(|| {
            response
                .get("entry")
                .and_then(|e| e.as_array())
                .and_then(|entries| {
                    entries.iter().find_map(|entry| {
                        let resource = entry.get("resource")?;
                        if resource.get("resourceType")?.as_str()? == "Appointment" {
                            Some(resource.clone())
                        } else {
                            None
                        }
                    })
                })
        })
}
