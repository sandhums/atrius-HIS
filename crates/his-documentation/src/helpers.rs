//! Shared validation helpers for clinical document services.

use his_domain::FhirClient;
use serde_json::Value;

use crate::error::DocumentationError;
use crate::kinds::ClinicalDocumentKind;

pub fn ensure_encounter_in_progress(encounter: &Value) -> Result<(), DocumentationError> {
    let status = encounter
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    if status != "in-progress" {
        return Err(DocumentationError::EncounterNotActive(status.to_string()));
    }
    Ok(())
}

pub fn ensure_encounter_inpatient(encounter: &Value) -> Result<(), DocumentationError> {
    let class_code = encounter
        .get("class")
        .and_then(|c| c.get("code"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    if class_code != "IMP" {
        return Err(DocumentationError::EncounterNotInpatient(class_code.to_string()));
    }
    Ok(())
}

pub fn ensure_composition_preliminary(composition: &Value) -> Result<(), DocumentationError> {
    let status = composition
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    if status == "preliminary" {
        Ok(())
    } else if status == "final" || status == "amended" {
        Err(DocumentationError::CompositionNotEditable(status.to_string()))
    } else {
        Err(DocumentationError::CompositionNotPreliminary(
            status.to_string(),
        ))
    }
}

pub fn ensure_composition_final(composition: &Value) -> Result<(), DocumentationError> {
    let status = composition
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    if status == "final" {
        Ok(())
    } else {
        Err(DocumentationError::CompositionNotFinal(status.to_string()))
    }
}

pub async fn find_preliminary_for_encounter(
    fhir: &FhirClient,
    encounter_id: &str,
    kind: ClinicalDocumentKind,
) -> Result<Option<String>, DocumentationError> {
    let bundle = fhir
        .search_resources(
            "Composition",
            &[
                ("encounter", &format!("Encounter/{encounter_id}")),
                ("status", "preliminary"),
            ],
        )
        .await
        .map_err(DocumentationError::from_fhir)?;

    let notes = his_domain::resources_from_search_bundle(&bundle)
        .map_err(DocumentationError::from_fhir)?;
    Ok(notes
        .into_iter()
        .find(|note| {
            his_domain::composition_profile(note) == Some(kind.profile_url())
        })
        .and_then(|note| {
            note.get("id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        }))
}

pub fn encounter_patient_id(encounter: &Value) -> Result<String, DocumentationError> {
    encounter
        .get("subject")
        .and_then(|s| s.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("Patient/"))
        .map(str::to_string)
        .ok_or_else(|| {
            DocumentationError::InvalidRequest("encounter has no Patient subject reference".into())
        })
}

pub fn new_composition_id() -> String {
    format!("comp-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}
