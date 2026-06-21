use his_domain::{
    ConsultNoteSections, FhirClient, composition_encounter_id, composition_from_transaction_response,
    composition_patient_id, finalize_consultation_composition, op_consult_transaction,
    op_consult_update_transaction, resources_from_search_bundle,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::error::{DocumentationError, practitioner_id_from_composition};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConsultationNoteRequest {
    pub encounter_id: String,
    #[serde(default = "default_practitioner_id")]
    pub practitioner_id: String,
    #[serde(default = "default_note_title")]
    pub title: String,
    pub sections: ConsultNoteSections,
}

fn default_practitioner_id() -> String {
    "dr-patel".into()
}

fn default_note_title() -> String {
    "OPD Consultation Note".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConsultationNoteRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub sections: ConsultNoteSections,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizeConsultationNoteRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub practitioner_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultationNoteResponse {
    pub composition_id: String,
    pub encounter_id: String,
    pub status: String,
    pub composition: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultationNoteListResponse {
    pub count: usize,
    pub notes: Vec<Value>,
}

#[derive(Clone)]
pub struct DocumentationService {
    fhir: FhirClient,
}

impl DocumentationService {
    pub fn new(fhir: FhirClient) -> Self {
        Self { fhir }
    }

    pub async fn create_consultation_note(
        &self,
        req: &CreateConsultationNoteRequest,
    ) -> Result<ConsultationNoteResponse, DocumentationError> {
        if req.encounter_id.trim().is_empty() {
            return Err(DocumentationError::InvalidRequest(
                "encounter_id is required".into(),
            ));
        }
        if req.practitioner_id.trim().is_empty() {
            return Err(DocumentationError::InvalidRequest(
                "practitioner_id is required".into(),
            ));
        }
        if !req.sections.has_content() {
            return Err(DocumentationError::InvalidRequest(
                "sections must include at least one non-empty field".into(),
            ));
        }

        let encounter = self.read_encounter(&req.encounter_id).await?;
        ensure_encounter_in_progress(&encounter)?;

        if let Some(existing_id) = self
            .find_preliminary_note_for_encounter(&req.encounter_id)
            .await?
        {
            return Err(DocumentationError::DraftNoteExists {
                encounter_id: req.encounter_id.clone(),
                composition_id: existing_id,
            });
        }

        self.fhir
            .read_resource("Practitioner", &req.practitioner_id)
            .await
            .map_err(|_| DocumentationError::PractitionerNotFound(req.practitioner_id.clone()))?;

        let patient_id = encounter
            .get("subject")
            .and_then(|s| s.get("reference"))
            .and_then(|r| r.as_str())
            .and_then(|r| r.strip_prefix("Patient/"))
            .ok_or_else(|| {
                DocumentationError::InvalidRequest(
                    "encounter has no Patient subject reference".into(),
                )
            })?;

        let composition_id = new_composition_id();
        let bundle = op_consult_transaction(
            &composition_id,
            patient_id,
            &req.encounter_id,
            &req.practitioner_id,
            &req.title,
            &req.sections,
        );

        debug!(%composition_id, encounter_id = %req.encounter_id, "create consultation note");
        let response = self
            .fhir
            .post_transaction(&bundle)
            .await
            .map_err(DocumentationError::from_fhir)?;

        let created = composition_from_transaction_response(&response).ok_or_else(|| {
            DocumentationError::InvalidRequest(
                "transaction response did not include Composition".into(),
            )
        })?;

        let composition_id = created
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&composition_id)
            .to_string();

        Ok(ConsultationNoteResponse {
            composition_id: composition_id.clone(),
            encounter_id: req.encounter_id.clone(),
            status: created
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("preliminary")
                .to_string(),
            composition: created,
        })
    }

    pub async fn read_consultation_note(
        &self,
        composition_id: &str,
    ) -> Result<Value, DocumentationError> {
        self.fhir
            .read_resource("Composition", composition_id)
            .await
            .map_err(|_| DocumentationError::CompositionNotFound(composition_id.to_string()))
    }

    pub async fn update_consultation_note(
        &self,
        composition_id: &str,
        req: &UpdateConsultationNoteRequest,
    ) -> Result<ConsultationNoteResponse, DocumentationError> {
        if !req.sections.has_content() {
            return Err(DocumentationError::InvalidRequest(
                "sections must include at least one non-empty field".into(),
            ));
        }

        let composition = self.read_consultation_note(composition_id).await?;
        ensure_composition_preliminary(&composition)?;

        let patient_id = composition_patient_id(&composition).ok_or_else(|| {
            DocumentationError::InvalidRequest("composition has no Patient subject".into())
        })?;
        let encounter_id = composition_encounter_id(&composition).ok_or_else(|| {
            DocumentationError::InvalidRequest("composition has no Encounter reference".into())
        })?;
        let practitioner_id = practitioner_id_from_composition(&composition)
            .unwrap_or_else(|| "dr-patel".into());

        let title = req
            .title
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| composition.get("title").and_then(|v| v.as_str()))
            .unwrap_or("Consultation Note")
            .to_string();

        let bundle = op_consult_update_transaction(
            &composition,
            &patient_id,
            &encounter_id,
            &practitioner_id,
            &title,
            &req.sections,
        );

        debug!(%composition_id, "update consultation note");
        let response = self
            .fhir
            .post_transaction(&bundle)
            .await
            .map_err(DocumentationError::from_fhir)?;

        let saved = composition_from_transaction_response(&response).unwrap_or(composition);

        let encounter_id = composition_encounter_id(&saved).unwrap_or_default();
        Ok(ConsultationNoteResponse {
            composition_id: composition_id.to_string(),
            encounter_id,
            status: saved
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("preliminary")
                .to_string(),
            composition: saved,
        })
    }

    pub async fn finalize_consultation_note(
        &self,
        composition_id: &str,
        req: &FinalizeConsultationNoteRequest,
    ) -> Result<ConsultationNoteResponse, DocumentationError> {
        let composition = self.read_consultation_note(composition_id).await?;
        ensure_composition_preliminary(&composition)?;

        let practitioner_id = req
            .practitioner_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| practitioner_id_from_composition(&composition))
            .ok_or_else(|| {
                DocumentationError::InvalidRequest(
                    "practitioner_id is required when composition has no author".into(),
                )
            })?;

        self.fhir
            .read_resource("Practitioner", &practitioner_id)
            .await
            .map_err(|_| DocumentationError::PractitionerNotFound(practitioner_id.clone()))?;

        let finalized = finalize_consultation_composition(&composition, &practitioner_id);
        debug!(%composition_id, %practitioner_id, "finalize consultation note");
        let saved = self
            .fhir
            .update_resource("Composition", composition_id, &finalized)
            .await
            .map_err(DocumentationError::from_fhir)?;

        let encounter_id = composition_encounter_id(&saved).unwrap_or_default();
        Ok(ConsultationNoteResponse {
            composition_id: composition_id.to_string(),
            encounter_id,
            status: saved
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("final")
                .to_string(),
            composition: saved,
        })
    }

    pub async fn list_by_encounter(
        &self,
        encounter_id: &str,
    ) -> Result<ConsultationNoteListResponse, DocumentationError> {
        if encounter_id.trim().is_empty() {
            return Err(DocumentationError::InvalidRequest(
                "encounter_id is required".into(),
            ));
        }

        let bundle = self
            .fhir
            .search_resources(
                "Composition",
                &[("encounter", &format!("Encounter/{encounter_id}"))],
            )
            .await
            .map_err(DocumentationError::from_fhir)?;

        let notes = resources_from_search_bundle(&bundle).map_err(DocumentationError::from_fhir)?;
        let count = notes.len();
        Ok(ConsultationNoteListResponse { count, notes })
    }

    async fn read_encounter(&self, encounter_id: &str) -> Result<Value, DocumentationError> {
        self.fhir
            .read_resource("Encounter", encounter_id)
            .await
            .map_err(|_| DocumentationError::EncounterNotFound(encounter_id.to_string()))
    }

    async fn find_preliminary_note_for_encounter(
        &self,
        encounter_id: &str,
    ) -> Result<Option<String>, DocumentationError> {
        let bundle = self
            .fhir
            .search_resources(
                "Composition",
                &[
                    ("encounter", &format!("Encounter/{encounter_id}")),
                    ("status", "preliminary"),
                ],
            )
            .await
            .map_err(DocumentationError::from_fhir)?;

        let notes = resources_from_search_bundle(&bundle).map_err(DocumentationError::from_fhir)?;
        Ok(notes
            .first()
            .and_then(|note| note.get("id"))
            .and_then(|v| v.as_str())
            .map(str::to_string))
    }
}

fn ensure_encounter_in_progress(encounter: &Value) -> Result<(), DocumentationError> {
    let status = encounter
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    if status != "in-progress" {
        return Err(DocumentationError::EncounterNotActive(status.to_string()));
    }
    Ok(())
}

fn ensure_composition_preliminary(composition: &Value) -> Result<(), DocumentationError> {
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

fn new_composition_id() -> String {
    format!("comp-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}
