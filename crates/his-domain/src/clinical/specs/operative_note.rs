//! Operative note Composition (surgical case documentation).

use serde_json::Value;

use crate::clinical::slice::{EntryKind, SnomedSliceDef};
use crate::clinical::specs::snomed_document::{
    SnomedDocumentMeta, snomed_document_transaction, snomed_document_update_transaction,
};
use crate::profiles::ATRIUS_IN_OPERATIVE_NOTE;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OperativeNoteSections {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_op_diagnosis: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedure_performed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub findings: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specimens: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_op_plan: Option<String>,
}

impl OperativeNoteSections {
    pub fn has_content(&self) -> bool {
        [
            &self.pre_op_diagnosis,
            &self.procedure_performed,
            &self.findings,
            &self.specimens,
            &self.post_op_plan,
        ]
        .iter()
        .any(|s| s.as_deref().is_some_and(|t| !t.trim().is_empty()))
    }
}

const SLICES: [SnomedSliceDef<OperativeNoteSections>; 5] = [
    SnomedSliceDef {
        slice: "PreOpDiagnosis",
        title: "Pre-operative diagnosis",
        code: "371529009",
        display: "History and physical report",
        field: |s| s.pre_op_diagnosis.as_ref(),
        entry: EntryKind::Condition,
        id_suffix: "preop",
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
        slice: "IntraoperativeFindings",
        title: "Intraoperative findings",
        code: "404684003",
        display: "Clinical finding",
        field: |s| s.findings.as_ref(),
        entry: EntryKind::Observation { exam: false },
        id_suffix: "findings",
    },
    SnomedSliceDef {
        slice: "Specimens",
        title: "Specimens",
        code: "721981007",
        display: "Diagnostic studies report",
        field: |s| s.specimens.as_ref(),
        entry: EntryKind::Observation { exam: false },
        id_suffix: "specimens",
    },
    SnomedSliceDef {
        slice: "PostOpPlan",
        title: "Post-operative plan",
        code: "734163000",
        display: "Care plan",
        field: |s| s.post_op_plan.as_ref(),
        entry: EntryKind::CarePlan,
        id_suffix: "postop",
    },
];

const META: SnomedDocumentMeta = SnomedDocumentMeta {
    profile: ATRIUS_IN_OPERATIVE_NOTE,
    type_code: "371525003",
    type_display: "Clinical procedure report",
};

#[must_use]
pub fn operative_note_transaction(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &OperativeNoteSections,
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
pub fn operative_note_update_transaction(
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &OperativeNoteSections,
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
