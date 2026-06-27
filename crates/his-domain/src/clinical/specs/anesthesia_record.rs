//! Anesthesia record Composition.

use serde_json::Value;

use crate::clinical::slice::{EntryKind, SnomedSliceDef};
use crate::clinical::specs::snomed_document::{
    SnomedDocumentMeta, snomed_document_transaction, snomed_document_update_transaction,
};
use crate::profiles::ATRIUS_IN_ANESTHESIA_RECORD;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AnesthesiaRecordSections {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_anesthesia_eval: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub airway_assessment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anesthetic_agents: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intraoperative_monitoring: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pacu_handoff: Option<String>,
}

impl AnesthesiaRecordSections {
    pub fn has_content(&self) -> bool {
        [
            &self.pre_anesthesia_eval,
            &self.airway_assessment,
            &self.anesthetic_agents,
            &self.intraoperative_monitoring,
            &self.pacu_handoff,
        ]
        .iter()
        .any(|s| s.as_deref().is_some_and(|t| !t.trim().is_empty()))
    }
}

const SLICES: [SnomedSliceDef<AnesthesiaRecordSections>; 5] = [
    SnomedSliceDef {
        slice: "PreAnesthesiaEval",
        title: "Pre-anesthesia evaluation",
        code: "371529009",
        display: "History and physical report",
        field: |s| s.pre_anesthesia_eval.as_ref(),
        entry: EntryKind::Condition,
        id_suffix: "preanes",
    },
    SnomedSliceDef {
        slice: "AirwayAssessment",
        title: "Airway assessment",
        code: "425044008",
        display: "Physical exam section",
        field: |s| s.airway_assessment.as_ref(),
        entry: EntryKind::Observation { exam: true },
        id_suffix: "airway",
    },
    SnomedSliceDef {
        slice: "AnestheticAgents",
        title: "Anesthetic agents",
        code: "721912009",
        display: "Medication summary document",
        field: |s| s.anesthetic_agents.as_ref(),
        entry: EntryKind::MedicationStatement,
        id_suffix: "agents",
    },
    SnomedSliceDef {
        slice: "IntraoperativeMonitoring",
        title: "Intraoperative monitoring",
        code: "404684003",
        display: "Clinical finding",
        field: |s| s.intraoperative_monitoring.as_ref(),
        entry: EntryKind::Observation { exam: false },
        id_suffix: "monitoring",
    },
    SnomedSliceDef {
        slice: "PacuHandoff",
        title: "PACU handoff",
        code: "390906007",
        display: "Follow-up encounter",
        field: |s| s.pacu_handoff.as_ref(),
        entry: EntryKind::Appointment,
        id_suffix: "pacu",
    },
];

const META: SnomedDocumentMeta = SnomedDocumentMeta {
    profile: ATRIUS_IN_ANESTHESIA_RECORD,
    type_code: "371525003",
    type_display: "Clinical procedure report",
};

#[must_use]
pub fn anesthesia_record_transaction(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &AnesthesiaRecordSections,
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
pub fn anesthesia_record_update_transaction(
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &AnesthesiaRecordSections,
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
