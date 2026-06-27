//! Immunization record Composition (NDHM ImmunizationRecord shape).

use serde_json::Value;

use crate::clinical::specs::entry_sliced::{
    CompositionTypeCoding, EntrySlicedMeta, build_immunization_entries, entry_sliced_transaction,
    entry_sliced_update_transaction,
};
use crate::profiles::ATRIUS_IN_IMMUNIZATION_RECORD;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ImmunizationRecordSections {
    #[serde(default)]
    pub immunizations: Vec<String>,
}

impl ImmunizationRecordSections {
    pub fn has_content(&self) -> bool {
        self.immunizations.iter().any(|v| !v.trim().is_empty())
    }
}

const META: EntrySlicedMeta = EntrySlicedMeta {
    profile: ATRIUS_IN_IMMUNIZATION_RECORD,
    composition_type: CompositionTypeCoding::Snomed {
        code: "41000179103",
        display: "Immunization record",
    },
    title_narrative: "Immunization record",
};

fn build_bundle_parts(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    sections: &ImmunizationRecordSections,
) -> (String, Vec<Value>, Vec<Value>) {
    let vaccines: Vec<String> = sections
        .immunizations
        .iter()
        .filter(|v| !v.trim().is_empty())
        .cloned()
        .collect();
    let section_text = vaccines.join("; ");
    let (resources, refs) = build_immunization_entries(
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        &vaccines,
    );
    (section_text, resources, refs)
}

#[must_use]
pub fn immunization_record_transaction(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ImmunizationRecordSections,
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
pub fn immunization_record_update_transaction(
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ImmunizationRecordSections,
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
