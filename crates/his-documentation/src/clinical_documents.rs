//! Generic clinical document CRUD for Phase 5d document types.

use his_domain::{
    AnesthesiaRecordSections, ImmunizationRecordSections, InvoiceRecordSections,
    OperativeNoteSections, PrescriptionSections, ProcedureNoteSections, ProgressNoteSections,
    WellnessSections, anesthesia_record_transaction, anesthesia_record_update_transaction,
    composition_encounter_id, composition_from_transaction_response, composition_patient_id,
    composition_profile, finalize_composition, immunization_record_transaction,
    immunization_record_update_transaction, invoice_record_transaction,
    invoice_record_update_transaction, operative_note_transaction,
    operative_note_update_transaction, prescription_transaction, prescription_update_transaction,
    procedure_note_transaction, procedure_note_update_transaction, progress_note_transaction,
    progress_note_update_transaction, wellness_record_transaction, wellness_record_update_transaction,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::error::{DocumentationError, practitioner_id_from_composition};
use crate::helpers::{
    ensure_composition_preliminary, ensure_encounter_inpatient, ensure_encounter_in_progress,
    encounter_patient_id, find_preliminary_for_encounter, new_composition_id,
};
use crate::kinds::ClinicalDocumentKind;
use crate::service::DocumentationService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalDocumentResponse {
    pub composition_id: String,
    pub encounter_id: String,
    pub status: String,
    pub composition: Value,
}

macro_rules! document_request {
    ($name:ident, $sections:ty, $default_title:expr) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct $name {
            pub encounter_id: String,
            #[serde(default = "default_practitioner_id")]
            pub practitioner_id: String,
            #[serde(default = $default_title)]
            pub title: String,
            pub sections: $sections,
        }
    };
}

document_request!(CreateProgressNoteRequest, ProgressNoteSections, "default_progress_title");
document_request!(CreateProcedureNoteRequest, ProcedureNoteSections, "default_procedure_title");
document_request!(CreateOperativeNoteRequest, OperativeNoteSections, "default_operative_title");
document_request!(CreateAnesthesiaRecordRequest, AnesthesiaRecordSections, "default_anesthesia_title");
document_request!(CreatePrescriptionRequest, PrescriptionSections, "default_prescription_title");
document_request!(CreateWellnessRecordRequest, WellnessSections, "default_wellness_title");
document_request!(CreateImmunizationRecordRequest, ImmunizationRecordSections, "default_immunization_title");
document_request!(CreateInvoiceRecordRequest, InvoiceRecordSections, "default_invoice_title");

macro_rules! update_request {
    ($name:ident, $sections:ty) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct $name {
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub title: Option<String>,
            pub sections: $sections,
        }
    };
}

update_request!(UpdateProgressNoteRequest, ProgressNoteSections);
update_request!(UpdateProcedureNoteRequest, ProcedureNoteSections);
update_request!(UpdateOperativeNoteRequest, OperativeNoteSections);
update_request!(UpdateAnesthesiaRecordRequest, AnesthesiaRecordSections);
update_request!(UpdatePrescriptionRequest, PrescriptionSections);
update_request!(UpdateWellnessRecordRequest, WellnessSections);
update_request!(UpdateImmunizationRecordRequest, ImmunizationRecordSections);
update_request!(UpdateInvoiceRecordRequest, InvoiceRecordSections);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizeClinicalDocumentRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub practitioner_id: Option<String>,
}

fn default_practitioner_id() -> String {
    "dr-patel".into()
}
fn default_progress_title() -> String {
    "Inpatient Progress Note".into()
}
fn default_procedure_title() -> String {
    "Procedure Note".into()
}
fn default_operative_title() -> String {
    "Operative Note".into()
}
fn default_anesthesia_title() -> String {
    "Anesthesia Record".into()
}
fn default_prescription_title() -> String {
    "Prescription".into()
}
fn default_wellness_title() -> String {
    "Wellness Record".into()
}
fn default_immunization_title() -> String {
    "Immunization Record".into()
}
fn default_invoice_title() -> String {
    "Invoice".into()
}

impl DocumentationService {
    pub async fn create_progress_note(
        &self,
        req: &CreateProgressNoteRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.create_document(
            ClinicalDocumentKind::ProgressNote,
            &req.encounter_id,
            &req.practitioner_id,
            req.sections.has_content(),
            |id, patient, enc, pr| {
                progress_note_transaction(id, patient, enc, pr, &req.title, &req.sections)
            },
        )
        .await
    }

    pub async fn read_progress_note(&self, id: &str) -> Result<Value, DocumentationError> {
        self.read_typed_document(id, ClinicalDocumentKind::ProgressNote)
            .await
    }

    pub async fn update_progress_note(
        &self,
        id: &str,
        req: &UpdateProgressNoteRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.update_document(
            id,
            ClinicalDocumentKind::ProgressNote,
            req.sections.has_content(),
            &req.title,
            "Inpatient Progress Note",
            |comp, patient, enc, pr, title| {
                progress_note_update_transaction(comp, patient, enc, pr, title, &req.sections)
            },
        )
        .await
    }

    pub async fn finalize_progress_note(
        &self,
        id: &str,
        req: &FinalizeClinicalDocumentRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.finalize_document(id, ClinicalDocumentKind::ProgressNote, req).await
    }

    pub async fn create_procedure_note(
        &self,
        req: &CreateProcedureNoteRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.create_document(
            ClinicalDocumentKind::ProcedureNote,
            &req.encounter_id,
            &req.practitioner_id,
            req.sections.has_content(),
            |id, patient, enc, pr| {
                procedure_note_transaction(id, patient, enc, pr, &req.title, &req.sections)
            },
        )
        .await
    }

    pub async fn read_procedure_note(&self, id: &str) -> Result<Value, DocumentationError> {
        self.read_typed_document(id, ClinicalDocumentKind::ProcedureNote)
            .await
    }

    pub async fn update_procedure_note(
        &self,
        id: &str,
        req: &UpdateProcedureNoteRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.update_document(
            id,
            ClinicalDocumentKind::ProcedureNote,
            req.sections.has_content(),
            &req.title,
            "Procedure Note",
            |comp, patient, enc, pr, title| {
                procedure_note_update_transaction(comp, patient, enc, pr, title, &req.sections)
            },
        )
        .await
    }

    pub async fn finalize_procedure_note(
        &self,
        id: &str,
        req: &FinalizeClinicalDocumentRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.finalize_document(id, ClinicalDocumentKind::ProcedureNote, req)
            .await
    }

    pub async fn create_operative_note(
        &self,
        req: &CreateOperativeNoteRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.create_document(
            ClinicalDocumentKind::OperativeNote,
            &req.encounter_id,
            &req.practitioner_id,
            req.sections.has_content(),
            |id, patient, enc, pr| {
                operative_note_transaction(id, patient, enc, pr, &req.title, &req.sections)
            },
        )
        .await
    }

    pub async fn read_operative_note(&self, id: &str) -> Result<Value, DocumentationError> {
        self.read_typed_document(id, ClinicalDocumentKind::OperativeNote)
            .await
    }

    pub async fn update_operative_note(
        &self,
        id: &str,
        req: &UpdateOperativeNoteRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.update_document(
            id,
            ClinicalDocumentKind::OperativeNote,
            req.sections.has_content(),
            &req.title,
            "Operative Note",
            |comp, patient, enc, pr, title| {
                operative_note_update_transaction(comp, patient, enc, pr, title, &req.sections)
            },
        )
        .await
    }

    pub async fn finalize_operative_note(
        &self,
        id: &str,
        req: &FinalizeClinicalDocumentRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.finalize_document(id, ClinicalDocumentKind::OperativeNote, req)
            .await
    }

    pub async fn create_anesthesia_record(
        &self,
        req: &CreateAnesthesiaRecordRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.create_document(
            ClinicalDocumentKind::AnesthesiaRecord,
            &req.encounter_id,
            &req.practitioner_id,
            req.sections.has_content(),
            |id, patient, enc, pr| {
                anesthesia_record_transaction(id, patient, enc, pr, &req.title, &req.sections)
            },
        )
        .await
    }

    pub async fn read_anesthesia_record(&self, id: &str) -> Result<Value, DocumentationError> {
        self.read_typed_document(id, ClinicalDocumentKind::AnesthesiaRecord)
            .await
    }

    pub async fn update_anesthesia_record(
        &self,
        id: &str,
        req: &UpdateAnesthesiaRecordRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.update_document(
            id,
            ClinicalDocumentKind::AnesthesiaRecord,
            req.sections.has_content(),
            &req.title,
            "Anesthesia Record",
            |comp, patient, enc, pr, title| {
                anesthesia_record_update_transaction(comp, patient, enc, pr, title, &req.sections)
            },
        )
        .await
    }

    pub async fn finalize_anesthesia_record(
        &self,
        id: &str,
        req: &FinalizeClinicalDocumentRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.finalize_document(id, ClinicalDocumentKind::AnesthesiaRecord, req)
            .await
    }

    pub async fn create_prescription_record(
        &self,
        req: &CreatePrescriptionRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.create_document(
            ClinicalDocumentKind::Prescription,
            &req.encounter_id,
            &req.practitioner_id,
            req.sections.has_content(),
            |id, patient, enc, pr| {
                prescription_transaction(id, patient, enc, pr, &req.title, &req.sections)
            },
        )
        .await
    }

    pub async fn read_prescription_record(&self, id: &str) -> Result<Value, DocumentationError> {
        self.read_typed_document(id, ClinicalDocumentKind::Prescription)
            .await
    }

    pub async fn update_prescription_record(
        &self,
        id: &str,
        req: &UpdatePrescriptionRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.update_document(
            id,
            ClinicalDocumentKind::Prescription,
            req.sections.has_content(),
            &req.title,
            "Prescription",
            |comp, patient, enc, pr, title| {
                prescription_update_transaction(comp, patient, enc, pr, title, &req.sections)
            },
        )
        .await
    }

    pub async fn finalize_prescription_record(
        &self,
        id: &str,
        req: &FinalizeClinicalDocumentRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.finalize_document(id, ClinicalDocumentKind::Prescription, req)
            .await
    }

    pub async fn create_wellness_record(
        &self,
        req: &CreateWellnessRecordRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.create_document(
            ClinicalDocumentKind::Wellness,
            &req.encounter_id,
            &req.practitioner_id,
            req.sections.has_content(),
            |id, patient, enc, pr| {
                wellness_record_transaction(id, patient, enc, pr, &req.title, &req.sections)
            },
        )
        .await
    }

    pub async fn read_wellness_record(&self, id: &str) -> Result<Value, DocumentationError> {
        self.read_typed_document(id, ClinicalDocumentKind::Wellness)
            .await
    }

    pub async fn update_wellness_record(
        &self,
        id: &str,
        req: &UpdateWellnessRecordRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.update_document(
            id,
            ClinicalDocumentKind::Wellness,
            req.sections.has_content(),
            &req.title,
            "Wellness Record",
            |comp, patient, enc, pr, title| {
                wellness_record_update_transaction(comp, patient, enc, pr, title, &req.sections)
            },
        )
        .await
    }

    pub async fn finalize_wellness_record(
        &self,
        id: &str,
        req: &FinalizeClinicalDocumentRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.finalize_document(id, ClinicalDocumentKind::Wellness, req)
            .await
    }

    pub async fn create_immunization_record(
        &self,
        req: &CreateImmunizationRecordRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.create_document(
            ClinicalDocumentKind::ImmunizationRecord,
            &req.encounter_id,
            &req.practitioner_id,
            req.sections.has_content(),
            |id, patient, enc, pr| {
                immunization_record_transaction(id, patient, enc, pr, &req.title, &req.sections)
            },
        )
        .await
    }

    pub async fn read_immunization_record(&self, id: &str) -> Result<Value, DocumentationError> {
        self.read_typed_document(id, ClinicalDocumentKind::ImmunizationRecord)
            .await
    }

    pub async fn update_immunization_record(
        &self,
        id: &str,
        req: &UpdateImmunizationRecordRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.update_document(
            id,
            ClinicalDocumentKind::ImmunizationRecord,
            req.sections.has_content(),
            &req.title,
            "Immunization Record",
            |comp, patient, enc, pr, title| {
                immunization_record_update_transaction(
                    comp,
                    patient,
                    enc,
                    pr,
                    title,
                    &req.sections,
                )
            },
        )
        .await
    }

    pub async fn finalize_immunization_record(
        &self,
        id: &str,
        req: &FinalizeClinicalDocumentRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.finalize_document(id, ClinicalDocumentKind::ImmunizationRecord, req)
            .await
    }

    pub async fn create_invoice_record(
        &self,
        req: &CreateInvoiceRecordRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.create_document(
            ClinicalDocumentKind::InvoiceRecord,
            &req.encounter_id,
            &req.practitioner_id,
            req.sections.has_content(),
            |id, patient, enc, pr| {
                invoice_record_transaction(id, patient, enc, pr, &req.title, &req.sections)
            },
        )
        .await
    }

    pub async fn read_invoice_record(&self, id: &str) -> Result<Value, DocumentationError> {
        self.read_typed_document(id, ClinicalDocumentKind::InvoiceRecord)
            .await
    }

    pub async fn update_invoice_record(
        &self,
        id: &str,
        req: &UpdateInvoiceRecordRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.update_document(
            id,
            ClinicalDocumentKind::InvoiceRecord,
            req.sections.has_content(),
            &req.title,
            "Invoice",
            |comp, patient, enc, pr, title| {
                invoice_record_update_transaction(comp, patient, enc, pr, title, &req.sections)
            },
        )
        .await
    }

    pub async fn finalize_invoice_record(
        &self,
        id: &str,
        req: &FinalizeClinicalDocumentRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        self.finalize_document(id, ClinicalDocumentKind::InvoiceRecord, req)
            .await
    }

    async fn create_document(
        &self,
        kind: ClinicalDocumentKind,
        encounter_id: &str,
        practitioner_id: &str,
        has_content: bool,
        build: impl FnOnce(&str, &str, &str, &str) -> Value,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        if encounter_id.trim().is_empty() {
            return Err(DocumentationError::InvalidRequest(
                "encounter_id is required".into(),
            ));
        }
        if practitioner_id.trim().is_empty() {
            return Err(DocumentationError::InvalidRequest(
                "practitioner_id is required".into(),
            ));
        }
        if !has_content {
            return Err(DocumentationError::InvalidRequest(
                "sections must include at least one non-empty field".into(),
            ));
        }

        let encounter = self.read_encounter(encounter_id).await?;
        ensure_encounter_in_progress(&encounter)?;
        if kind.requires_inpatient_encounter() {
            ensure_encounter_inpatient(&encounter)?;
        }

        if let Some(existing_id) =
            find_preliminary_for_encounter(&self.fhir, encounter_id, kind).await?
        {
            return Err(DocumentationError::DraftNoteExists {
                encounter_id: encounter_id.to_string(),
                composition_id: existing_id,
            });
        }

        self.fhir
            .read_resource("Practitioner", practitioner_id)
            .await
            .map_err(|_| DocumentationError::PractitionerNotFound(practitioner_id.to_string()))?;

        let patient_id = encounter_patient_id(&encounter)?;
        let composition_id = new_composition_id();
        let bundle = build(&composition_id, &patient_id, encounter_id, practitioner_id);

        debug!(%composition_id, %encounter_id, kind = ?kind, "create clinical document");
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

        Ok(clinical_document_response(composition_id, encounter_id, created))
    }

    async fn read_typed_document(
        &self,
        composition_id: &str,
        kind: ClinicalDocumentKind,
    ) -> Result<Value, DocumentationError> {
        let composition = self.read_composition(composition_id).await?;
        if composition_profile(&composition) != Some(kind.profile_url()) {
            return Err(DocumentationError::CompositionNotFound(
                composition_id.to_string(),
            ));
        }
        Ok(composition)
    }

    async fn update_document(
        &self,
        composition_id: &str,
        kind: ClinicalDocumentKind,
        has_content: bool,
        title: &Option<String>,
        default_title: &str,
        build: impl FnOnce(&Value, &str, &str, &str, &str) -> Value,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        if !has_content {
            return Err(DocumentationError::InvalidRequest(
                "sections must include at least one non-empty field".into(),
            ));
        }

        let composition = self.read_typed_document(composition_id, kind).await?;
        ensure_composition_preliminary(&composition)?;

        let patient_id = composition_patient_id(&composition).ok_or_else(|| {
            DocumentationError::InvalidRequest("composition has no Patient subject".into())
        })?;
        let encounter_id = composition_encounter_id(&composition).ok_or_else(|| {
            DocumentationError::InvalidRequest("composition has no Encounter reference".into())
        })?;
        let practitioner_id = practitioner_id_from_composition(&composition)
            .unwrap_or_else(|| "dr-patel".into());
        let title = title
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| composition.get("title").and_then(|v| v.as_str()))
            .unwrap_or(default_title)
            .to_string();

        let bundle = build(
            &composition,
            &patient_id,
            &encounter_id,
            &practitioner_id,
            &title,
        );

        debug!(%composition_id, kind = ?kind, "update clinical document");
        let response = self
            .fhir
            .post_transaction(&bundle)
            .await
            .map_err(DocumentationError::from_fhir)?;

        let saved = composition_from_transaction_response(&response).unwrap_or(composition);
        Ok(clinical_document_response(
            composition_id.to_string(),
            &encounter_id,
            saved,
        ))
    }

    async fn finalize_document(
        &self,
        composition_id: &str,
        kind: ClinicalDocumentKind,
        req: &FinalizeClinicalDocumentRequest,
    ) -> Result<ClinicalDocumentResponse, DocumentationError> {
        let composition = self.read_typed_document(composition_id, kind).await?;
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
        debug!(%composition_id, %practitioner_id, kind = ?kind, "finalize clinical document");
        let saved = self
            .fhir
            .update_resource("Composition", composition_id, &finalized)
            .await
            .map_err(DocumentationError::from_fhir)?;

        let encounter_id = composition_encounter_id(&saved).unwrap_or_default();
        Ok(clinical_document_response(
            composition_id.to_string(),
            &encounter_id,
            saved,
        ))
    }

    async fn read_composition(&self, composition_id: &str) -> Result<Value, DocumentationError> {
        self.fhir
            .read_resource("Composition", composition_id)
            .await
            .map_err(|_| DocumentationError::CompositionNotFound(composition_id.to_string()))
    }
}

fn clinical_document_response(
    composition_id: String,
    encounter_id: &str,
    composition: Value,
) -> ClinicalDocumentResponse {
    ClinicalDocumentResponse {
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
