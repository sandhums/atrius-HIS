//! Wellness record Composition (NDHM WellnessRecord shape, title-sliced sections).

use serde_json::Value;

use crate::clinical::specs::entry_sliced::{
    CompositionTypeCoding, EntrySlicedMeta, TitleSliceDef, title_sliced_transaction,
    title_sliced_update_transaction,
};
use crate::profiles::{
    ATRIUS_IN_OBSERVATION_BODY_MEASUREMENT, ATRIUS_IN_OBSERVATION_GENERAL_ASSESSMENT,
    ATRIUS_IN_OBSERVATION_LIFESTYLE, ATRIUS_IN_OBSERVATION_PHYSICAL_ACTIVITY,
    ATRIUS_IN_OBSERVATION_VITAL_SIGNS, ATRIUS_IN_OBSERVATION_WOMEN_HEALTH,
    ATRIUS_IN_OBSERVATION, ATRIUS_IN_WELLNESS_RECORD,
};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct WellnessSections {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vital_signs: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_measurement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_activity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general_assessment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub women_health: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifestyle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other_observations: Option<String>,
}

impl WellnessSections {
    pub fn has_content(&self) -> bool {
        [
            &self.vital_signs,
            &self.body_measurement,
            &self.physical_activity,
            &self.general_assessment,
            &self.women_health,
            &self.lifestyle,
            &self.other_observations,
        ]
        .iter()
        .any(|s| s.as_deref().is_some_and(|t| !t.trim().is_empty()))
    }
}

const SLICES: [TitleSliceDef<WellnessSections>; 7] = [
    TitleSliceDef {
        title: "Vital Signs",
        profile: ATRIUS_IN_OBSERVATION_VITAL_SIGNS,
        category_code: "vital-signs",
        category_display: "Vital Signs",
        field: |s| s.vital_signs.as_ref(),
        id_suffix: "vitals",
    },
    TitleSliceDef {
        title: "Body Measurement",
        profile: ATRIUS_IN_OBSERVATION_BODY_MEASUREMENT,
        category_code: "survey",
        category_display: "Survey",
        field: |s| s.body_measurement.as_ref(),
        id_suffix: "body",
    },
    TitleSliceDef {
        title: "Physical Activity",
        profile: ATRIUS_IN_OBSERVATION_PHYSICAL_ACTIVITY,
        category_code: "activity",
        category_display: "Activity",
        field: |s| s.physical_activity.as_ref(),
        id_suffix: "activity",
    },
    TitleSliceDef {
        title: "General Assessment",
        profile: ATRIUS_IN_OBSERVATION_GENERAL_ASSESSMENT,
        category_code: "survey",
        category_display: "Survey",
        field: |s| s.general_assessment.as_ref(),
        id_suffix: "general",
    },
    TitleSliceDef {
        title: "Women Health",
        profile: ATRIUS_IN_OBSERVATION_WOMEN_HEALTH,
        category_code: "survey",
        category_display: "Survey",
        field: |s| s.women_health.as_ref(),
        id_suffix: "women",
    },
    TitleSliceDef {
        title: "Lifestyle",
        profile: ATRIUS_IN_OBSERVATION_LIFESTYLE,
        category_code: "social-history",
        category_display: "Social History",
        field: |s| s.lifestyle.as_ref(),
        id_suffix: "lifestyle",
    },
    TitleSliceDef {
        title: "Other Observations",
        profile: ATRIUS_IN_OBSERVATION,
        category_code: "survey",
        category_display: "Survey",
        field: |s| s.other_observations.as_ref(),
        id_suffix: "other",
    },
];

const META: EntrySlicedMeta = EntrySlicedMeta {
    profile: ATRIUS_IN_WELLNESS_RECORD,
    composition_type: CompositionTypeCoding::Loinc {
        code: "11506-3",
        display: "Progress note",
    },
    title_narrative: "Wellness record",
};

#[must_use]
pub fn wellness_record_transaction(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &WellnessSections,
) -> Value {
    title_sliced_transaction(
        &META,
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        sections,
        &SLICES,
    )
}

#[must_use]
pub fn wellness_record_update_transaction(
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &WellnessSections,
) -> Value {
    title_sliced_update_transaction(
        &META,
        composition,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        sections,
        &SLICES,
    )
}
