//! Shared Composition lifecycle helpers for clinical documents.

use crate::adt::now_datetime;
use serde_json::{Value, json};

/// Mark a Composition final with professional attestation.
#[must_use]
pub fn finalize_composition(composition: &Value, practitioner_id: &str) -> Value {
    let mut updated = composition.clone();
    let attested = now_datetime();
    updated["status"] = json!("final");
    updated["attester"] = json!([{
        "mode": "professional",
        "time": attested,
        "party": { "reference": format!("Practitioner/{practitioner_id}") }
    }]);
    updated
}

/// Extract the Composition from a transaction response bundle.
#[must_use]
pub fn composition_from_transaction_response(response: &Value) -> Option<Value> {
    response
        .get("entry")
        .and_then(|e| e.as_array())
        .and_then(|entries| {
            entries.iter().find_map(|entry| {
                let resource = entry.get("resource")?;
                if resource.get("resourceType")?.as_str()? == "Composition" {
                    Some(resource.clone())
                } else {
                    None
                }
            })
        })
}

/// Patient id from Composition.subject reference.
pub fn composition_patient_id(composition: &Value) -> Option<String> {
    composition
        .get("subject")
        .and_then(|s| s.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("Patient/"))
        .map(str::to_string)
}

/// Encounter id from Composition.encounter reference.
pub fn composition_encounter_id(composition: &Value) -> Option<String> {
    composition
        .get("encounter")
        .and_then(|e| e.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("Encounter/"))
        .map(str::to_string)
}

/// Profile URL from Composition.meta.profile (first entry).
pub fn composition_profile(composition: &Value) -> Option<&str> {
    composition
        .get("meta")
        .and_then(|m| m.get("profile"))
        .and_then(|p| p.as_array())
        .and_then(|profiles| profiles.first())
        .and_then(|v| v.as_str())
}
