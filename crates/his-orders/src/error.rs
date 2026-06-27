use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum OrderError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("encounter not found: {0}")]
    EncounterNotFound(String),
    #[error("encounter is not active (status={0})")]
    EncounterNotActive(String),
    #[error("lab order not found: {0}")]
    LabOrderNotFound(String),
    #[error("lab order is not active (status={0})")]
    LabOrderNotActive(String),
    #[error("unknown LOINC code: {0}")]
    UnknownLoincCode(String),
    #[error("lab task not found: {0}")]
    LabTaskNotFound(String),
    #[error("FHIR error: {0}")]
    Fhir(#[from] anyhow::Error),
}

impl OrderError {
    pub fn from_fhir(err: anyhow::Error) -> Self {
        Self::Fhir(err)
    }
}

pub fn service_request_id_from_create_response(response: &Value, fallback_id: &str) -> String {
    response
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| fallback_id.to_string())
}

pub fn encounter_patient_id(encounter: &Value) -> Result<String, OrderError> {
    encounter
        .get("subject")
        .and_then(|s| s.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("Patient/"))
        .map(str::to_string)
        .ok_or_else(|| {
            OrderError::InvalidRequest("encounter has no Patient subject reference".into())
        })
}

pub fn ensure_encounter_in_progress(encounter: &Value) -> Result<(), OrderError> {
    let status = encounter
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    if status != "in-progress" {
        return Err(OrderError::EncounterNotActive(status.to_string()));
    }
    Ok(())
}

pub fn ensure_lab_order_active(order: &Value) -> Result<(), OrderError> {
    let status = order
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    if status == "active" {
        Ok(())
    } else {
        Err(OrderError::LabOrderNotActive(status.to_string()))
    }
}
