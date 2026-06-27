//! Inpatient progress note (SOAP) Composition.

use serde_json::Value;

use crate::clinical::slice::{EntryKind, SnomedSliceDef};
use crate::clinical::specs::snomed_document::{
    SnomedDocumentMeta, snomed_document_transaction, snomed_document_update_transaction,
};
use crate::profiles::ATRIUS_IN_INPATIENT_PROGRESS_NOTE;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ProgressNoteSections {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subjective: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assessment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
}

impl ProgressNoteSections {
    pub fn has_content(&self) -> bool {
        [&self.subjective, &self.objective, &self.assessment, &self.plan]
            .iter()
            .any(|s| s.as_deref().is_some_and(|t| !t.trim().is_empty()))
    }
}

const SLICES: [SnomedSliceDef<ProgressNoteSections>; 4] = [
    SnomedSliceDef {
        slice: "Subjective",
        title: "Subjective",
        code: "422843007",
        display: "Chief complaint section",
        field: |s| s.subjective.as_ref(),
        entry: EntryKind::Condition,
        id_suffix: "subjective",
    },
    SnomedSliceDef {
        slice: "Objective",
        title: "Objective",
        code: "425044008",
        display: "Physical exam section",
        field: |s| s.objective.as_ref(),
        entry: EntryKind::Observation { exam: true },
        id_suffix: "objective",
    },
    SnomedSliceDef {
        slice: "Assessment",
        title: "Assessment",
        code: "404684003",
        display: "Clinical finding",
        field: |s| s.assessment.as_ref(),
        entry: EntryKind::Observation { exam: false },
        id_suffix: "assessment",
    },
    SnomedSliceDef {
        slice: "Plan",
        title: "Plan",
        code: "390906007",
        display: "Follow-up encounter",
        field: |s| s.plan.as_ref(),
        entry: EntryKind::Appointment,
        id_suffix: "plan",
    },
];

const META: SnomedDocumentMeta = SnomedDocumentMeta {
    profile: ATRIUS_IN_INPATIENT_PROGRESS_NOTE,
    type_code: "371530004",
    type_display: "Clinical consultation report",
};

#[must_use]
pub fn progress_note_transaction(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ProgressNoteSections,
) -> Value {
    snomed_document_transaction(
        &META,
        &SLICES,
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        sections,
    )
}

#[must_use]
pub fn progress_note_update_transaction(
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ProgressNoteSections,
) -> Value {
    snomed_document_update_transaction(
        &META,
        &SLICES,
        composition,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        sections,
    )
}
