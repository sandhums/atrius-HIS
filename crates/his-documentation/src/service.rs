use his_domain::{
    ConsultNoteSections, DischargeSummarySections, FhirClient, composition_encounter_id,
    composition_from_transaction_response, composition_patient_id, composition_profile,
    discharge_summary_transaction, discharge_summary_update_transaction, export_document_bundle,
    finalize_composition, op_consult_transaction, op_consult_update_transaction,
    resources_from_search_bundle, section_entry_references,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::error::{DocumentationError, practitioner_id_from_composition};
use crate::helpers::{
    ensure_composition_final, ensure_composition_preliminary, ensure_encounter_inpatient,
    ensure_encounter_in_progress, find_preliminary_for_encounter, new_composition_id,
};
use crate::kinds::ClinicalDocumentKind;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDischargeSummaryRequest {
    pub encounter_id: String,
    #[serde(default = "default_practitioner_id")]
    pub practitioner_id: String,
    #[serde(default = "default_discharge_title")]
    pub title: String,
    pub sections: DischargeSummarySections,
}

fn default_discharge_title() -> String {
    "Discharge Summary".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDischargeSummaryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub sections: DischargeSummarySections,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizeDischargeSummaryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub practitioner_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DischargeSummaryResponse {
    pub composition_id: String,
    pub encounter_id: String,
    pub status: String,
    pub composition: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentBundleResponse {
    pub composition_id: String,
    pub bundle: Value,
}

#[derive(Clone)]
pub struct DocumentationService {
    pub(crate) fhir: FhirClient,
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

        if let Some(existing_id) = find_preliminary_for_encounter(
            &self.fhir,
            &req.encounter_id,
            ClinicalDocumentKind::OpConsult,
        )
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

        let finalized = finalize_composition(&composition, &practitioner_id);
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

    pub async fn create_discharge_summary(
        &self,
        req: &CreateDischargeSummaryRequest,
    ) -> Result<DischargeSummaryResponse, DocumentationError> {
        if req.encounter_id.trim().is_empty() {
            return Err(DocumentationError::InvalidRequest(
                "encounter_id is required".into(),
            ));
        }
        if !req.sections.has_content() {
            return Err(DocumentationError::InvalidRequest(
                "sections must include at least one non-empty field".into(),
            ));
        }

        let encounter = self.read_encounter(&req.encounter_id).await?;
        ensure_encounter_in_progress(&encounter)?;
        ensure_encounter_inpatient(&encounter)?;

        if let Some(existing_id) = find_preliminary_for_encounter(
            &self.fhir,
            &req.encounter_id,
            ClinicalDocumentKind::DischargeSummary,
        )
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

        let patient_id = crate::helpers::encounter_patient_id(&encounter)?;
        let composition_id = new_composition_id();
        let bundle = discharge_summary_transaction(
            &composition_id,
            &patient_id,
            &req.encounter_id,
            &req.practitioner_id,
            &req.title,
            &req.sections,
        );

        debug!(%composition_id, encounter_id = %req.encounter_id, "create discharge summary");
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

        Ok(discharge_response(composition_id, &req.encounter_id, created))
    }

    pub async fn read_discharge_summary(
        &self,
        composition_id: &str,
    ) -> Result<Value, DocumentationError> {
        let composition = self.read_consultation_note(composition_id).await?;
        if composition_profile(&composition) != Some(ClinicalDocumentKind::DischargeSummary.profile_url())
        {
            return Err(DocumentationError::CompositionNotFound(
                composition_id.to_string(),
            ));
        }
        Ok(composition)
    }

    pub async fn update_discharge_summary(
        &self,
        composition_id: &str,
        req: &UpdateDischargeSummaryRequest,
    ) -> Result<DischargeSummaryResponse, DocumentationError> {
        if !req.sections.has_content() {
            return Err(DocumentationError::InvalidRequest(
                "sections must include at least one non-empty field".into(),
            ));
        }

        let composition = self.read_discharge_summary(composition_id).await?;
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
            .unwrap_or("Discharge Summary")
            .to_string();

        let bundle = discharge_summary_update_transaction(
            &composition,
            &patient_id,
            &encounter_id,
            &practitioner_id,
            &title,
            &req.sections,
        );

        debug!(%composition_id, "update discharge summary");
        let response = self
            .fhir
            .post_transaction(&bundle)
            .await
            .map_err(DocumentationError::from_fhir)?;

        let saved = composition_from_transaction_response(&response).unwrap_or(composition);
        Ok(discharge_response(
            composition_id.to_string(),
            &encounter_id,
            saved,
        ))
    }

    pub async fn finalize_discharge_summary(
        &self,
        composition_id: &str,
        req: &FinalizeDischargeSummaryRequest,
    ) -> Result<DischargeSummaryResponse, DocumentationError> {
        let composition = self.read_discharge_summary(composition_id).await?;
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

        let finalized = finalize_composition(&composition, &practitioner_id);
        debug!(%composition_id, %practitioner_id, "finalize discharge summary");
        let saved = self
            .fhir
            .update_resource("Composition", composition_id, &finalized)
            .await
            .map_err(DocumentationError::from_fhir)?;

        let encounter_id = composition_encounter_id(&saved).unwrap_or_default();
        Ok(discharge_response(
            composition_id.to_string(),
            &encounter_id,
            saved,
        ))
    }

    pub async fn export_document_bundle(
        &self,
        composition_id: &str,
    ) -> Result<DocumentBundleResponse, DocumentationError> {
        let composition = self.read_consultation_note(composition_id).await?;
        ensure_composition_final(&composition)?;

        let mut referenced = Vec::new();
        if let Some(patient_id) = composition_patient_id(&composition) {
            referenced.push(
                self.fhir
                    .read_resource("Patient", &patient_id)
                    .await
                    .map_err(DocumentationError::from_fhir)?,
            );
        }
        if let Some(encounter_id) = composition_encounter_id(&composition) {
            referenced.push(
                self.fhir
                    .read_resource("Encounter", &encounter_id)
                    .await
                    .map_err(DocumentationError::from_fhir)?,
            );
        }

        for reference in section_entry_references(&composition) {
            let Some((resource_type, id)) = reference.split_once('/') else {
                continue;
            };
            referenced.push(
                self.fhir
                    .read_resource(resource_type, id)
                    .await
                    .map_err(DocumentationError::from_fhir)?,
            );
        }

        let bundle = export_document_bundle(&composition, &referenced);
        Ok(DocumentBundleResponse {
            composition_id: composition_id.to_string(),
            bundle,
        })
    }

    pub(crate) async fn read_encounter(&self, encounter_id: &str) -> Result<Value, DocumentationError> {
        self.fhir
            .read_resource("Encounter", encounter_id)
            .await
            .map_err(|_| DocumentationError::EncounterNotFound(encounter_id.to_string()))
    }
}

fn discharge_response(
    composition_id: String,
    encounter_id: &str,
    composition: Value,
) -> DischargeSummaryResponse {
    DischargeSummaryResponse {
        composition_id: composition
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&composition_id)
            .to_string(),
        encounter_id: encounter_id.to_string(),
        status: composition
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("preliminary")
            .to_string(),
        composition,
    }
}
