//! Prescription record Composition (NDHM PrescriptionRecord shape).

use serde_json::Value;

use crate::clinical::specs::entry_sliced::{
    CompositionTypeCoding, EntrySlicedMeta, build_medication_entries, entry_sliced_transaction,
    entry_sliced_update_transaction,
};
use crate::profiles::ATRIUS_IN_PRESCRIPTION_RECORD;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PrescriptionSections {
    #[serde(default)]
    pub medications: Vec<String>,
}

impl PrescriptionSections {
    pub fn has_content(&self) -> bool {
        self.medications.iter().any(|m| !m.trim().is_empty())
    }
}

const META: EntrySlicedMeta = EntrySlicedMeta {
    profile: ATRIUS_IN_PRESCRIPTION_RECORD,
    composition_type: CompositionTypeCoding::Snomed {
        code: "440545006",
        display: "Prescription record",
    },
    title_narrative: "Prescription record",
};

fn build_bundle_parts(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    sections: &PrescriptionSections,
) -> (String, Vec<Value>, Vec<Value>) {
    let meds: Vec<String> = sections
        .medications
        .iter()
        .filter(|m| !m.trim().is_empty())
        .cloned()
        .collect();
    let section_text = meds.join("; ");
    let (resources, refs) = build_medication_entries(
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        &meds,
    );
    (section_text, resources, refs)
}

#[must_use]
pub fn prescription_transaction(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &PrescriptionSections,
) -> Value {
    let (section_text, resources, refs) = build_bundle_parts(
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        sections,
    );
    entry_sliced_transaction(
        &META,
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        &section_text,
        resources,
        refs,
    )
}

#[must_use]
pub fn prescription_update_transaction(
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &PrescriptionSections,
) -> Value {
    let composition_id = composition
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let (section_text, resources, refs) = build_bundle_parts(
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        sections,
    );
    entry_sliced_update_transaction(
        &META,
        composition,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        &section_text,
        resources,
        refs,
    )
}
