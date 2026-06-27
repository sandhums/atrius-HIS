//! Inpatient procedure note Composition.

use serde_json::Value;

use crate::clinical::slice::{EntryKind, SnomedSliceDef};
use crate::clinical::specs::snomed_document::{
    SnomedDocumentMeta, snomed_document_transaction, snomed_document_update_transaction,
};
use crate::profiles::ATRIUS_IN_INPATIENT_PROCEDURE_NOTE;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ProcedureNoteSections {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indication: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedure_performed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub findings: Option<String>,
}

impl ProcedureNoteSections {
    pub fn has_content(&self) -> bool {
        [&self.indication, &self.procedure_performed, &self.findings]
            .iter()
            .any(|s| s.as_deref().is_some_and(|t| !t.trim().is_empty()))
    }
}

const SLICES: [SnomedSliceDef<ProcedureNoteSections>; 3] = [
    SnomedSliceDef {
        slice: "Indication",
        title: "Indication",
        code: "404684003",
        display: "Clinical finding",
        field: |s| s.indication.as_ref(),
        entry: EntryKind::Condition,
        id_suffix: "indication",
    },
    SnomedSliceDef {
        slice: "ProcedurePerformed",
        title: "Procedure performed",
        code: "371525003",
        display: "Clinical procedure report",
        field: |s| s.procedure_performed.as_ref(),
        entry: EntryKind::Procedure,
        id_suffix: "procedure",
    },
    SnomedSliceDef {
        slice: "Findings",
        title: "Findings",
        code: "721981007",
        display: "Diagnostic studies report",
        field: |s| s.findings.as_ref(),
        entry: EntryKind::Observation { exam: false },
        id_suffix: "findings",
    },
];

const META: SnomedDocumentMeta = SnomedDocumentMeta {
    profile: ATRIUS_IN_INPATIENT_PROCEDURE_NOTE,
    type_code: "371525003",
    type_display: "Clinical procedure report",
};

#[must_use]
pub fn procedure_note_transaction(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ProcedureNoteSections,
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
pub fn procedure_note_update_transaction(
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ProcedureNoteSections,
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
