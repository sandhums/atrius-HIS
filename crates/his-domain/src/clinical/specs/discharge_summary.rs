//! Discharge summary Composition (NDHM DischargeSummaryRecord shape).

use serde_json::{Value, json};

use crate::adt::now_datetime;
use crate::clinical::entry_builders::{
    SNOMED, narrative_care_plan, narrative_condition, narrative_diagnostic_report_lab,
    narrative_lab_result_observation, narrative_medication_request, narrative_observation,
    narrative_procedure, section_text_div,
};
use crate::clinical::slice::{EntryKind, SnomedSliceDef};
use crate::clinical::transaction::{create_transaction, update_transaction};
use crate::profiles::ATRIUS_IN_DISCHARGE_SUMMARY_RECORD;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DischargeSummarySections {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chief_complaint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exam: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hospital_course: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub investigations: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discharge_medications: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedures: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub care_plan: Option<String>,
}

impl DischargeSummarySections {
    pub fn has_content(&self) -> bool {
        [
            &self.chief_complaint,
            &self.exam,
            &self.hospital_course,
            &self.investigations,
            &self.discharge_medications,
            &self.procedures,
            &self.care_plan,
        ]
        .iter()
        .any(|s| s.as_deref().is_some_and(|t| !t.trim().is_empty()))
    }
}

const DISCHARGE_SUMMARY_SLICES: [SnomedSliceDef<DischargeSummarySections>; 7] = [
    SnomedSliceDef {
        slice: "ChiefComplaints",
        title: "Chief complaints",
        code: "422843007",
        display: "Chief complaint section",
        field: |s| s.chief_complaint.as_ref(),
        entry: EntryKind::Condition,
        id_suffix: "cc",
    },
    SnomedSliceDef {
        slice: "PhysicalExamination",
        title: "Physical examination",
        code: "425044008",
        display: "Physical exam section",
        field: |s| s.exam.as_ref(),
        entry: EntryKind::Observation { exam: true },
        id_suffix: "exam",
    },
    SnomedSliceDef {
        slice: "MedicalHistory",
        title: "Hospital course",
        code: "1003642006",
        display: "Past medical history section",
        field: |s| s.hospital_course.as_ref(),
        entry: EntryKind::Condition,
        id_suffix: "course",
    },
    SnomedSliceDef {
        slice: "Investigations",
        title: "Investigations",
        code: "721981007",
        display: "Diagnostic studies report",
        field: |s| s.investigations.as_ref(),
        entry: EntryKind::DiagnosticReportLab,
        id_suffix: "investigations",
    },
    SnomedSliceDef {
        slice: "Medications",
        title: "Discharge medications",
        code: "1003606003",
        display: "Medication history section",
        field: |s| s.discharge_medications.as_ref(),
        entry: EntryKind::MedicationRequest,
        id_suffix: "medications",
    },
    SnomedSliceDef {
        slice: "Procedures",
        title: "Procedures",
        code: "1003640003",
        display: "History of past procedure section",
        field: |s| s.procedures.as_ref(),
        entry: EntryKind::Procedure,
        id_suffix: "procedures",
    },
    SnomedSliceDef {
        slice: "CarePlan",
        title: "Care plan",
        code: "734163000",
        display: "Care plan",
        field: |s| s.care_plan.as_ref(),
        entry: EntryKind::CarePlan,
        id_suffix: "careplan",
    },
];

pub fn profile_slice_codes() -> &'static [&'static str] {
    &[
        "422843007", "425044008", "1003642006", "721981007", "1003606003", "1003640003",
        "734163000",
    ]
}

#[must_use]
pub fn discharge_summary_transaction(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &DischargeSummarySections,
) -> Value {
    let (composition, entry_resources) = build_with_entries(
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        sections,
        "preliminary",
        None,
    );
    create_transaction(composition_id, composition, entry_resources)
}

#[must_use]
pub fn discharge_summary_update_transaction(
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &DischargeSummarySections,
) -> Value {
    let composition_id = composition
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let (updated, entry_resources) = build_with_entries(
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        sections,
        "preliminary",
        None,
    );
    update_transaction(composition_id, updated, entry_resources)
}

fn build_with_entries(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &DischargeSummarySections,
    status: &str,
    attester_practitioner_id: Option<&str>,
) -> (Value, Vec<Value>) {
    let mut section_entries = Vec::new();
    let mut entry_resources = Vec::new();

    for slice in DISCHARGE_SUMMARY_SLICES {
        let Some(text) = (slice.field)(sections).filter(|t| !t.trim().is_empty()) else {
            continue;
        };
        let resource_id = format!("{composition_id}-{}", slice.id_suffix);
        match slice.entry {
            EntryKind::DiagnosticReportLab => {
                let result_id = format!("{resource_id}-result");
                let observation = narrative_lab_result_observation(
                    &result_id,
                    patient_id,
                    encounter_id,
                    text,
                    slice.title,
                );
                let report = narrative_diagnostic_report_lab(
                    &resource_id,
                    patient_id,
                    encounter_id,
                    practitioner_id,
                    text,
                    slice.title,
                    &result_id,
                );
                entry_resources.push(observation);
                entry_resources.push(report);
            }
            _ => {
                let resource = build_entry_resource(
                    &slice,
                    &resource_id,
                    patient_id,
                    encounter_id,
                    practitioner_id,
                    text,
                );
                entry_resources.push(resource);
            }
        }
        section_entries.push(json!({
            "title": slice.title,
            "code": {
                "coding": [{
                    "system": SNOMED,
                    "code": slice.code,
                    "display": slice.display
                }]
            },
            "text": section_text_div(text),
            "entry": [{
                "reference": format!("{}/{resource_id}", slice.resource_type())
            }]
        }));
    }

    let mut composition = build_composition_shell(
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        status,
        attester_practitioner_id,
    );
    composition["section"] = json!(section_entries);
    (composition, entry_resources)
}

fn build_entry_resource(
    slice: &SnomedSliceDef<DischargeSummarySections>,
    resource_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    text: &str,
) -> Value {
    match slice.entry {
        EntryKind::Condition => narrative_condition(
            resource_id,
            patient_id,
            encounter_id,
            text,
            slice.title,
        ),
        EntryKind::Observation { exam } => narrative_observation(
            resource_id,
            patient_id,
            encounter_id,
            text,
            slice.title,
            exam,
        ),
        EntryKind::MedicationRequest => narrative_medication_request(
            resource_id,
            patient_id,
            encounter_id,
            practitioner_id,
            text,
        ),
        EntryKind::Procedure => {
            narrative_procedure(resource_id, patient_id, encounter_id, text, slice.title)
        }
        EntryKind::CarePlan => {
            narrative_care_plan(resource_id, patient_id, encounter_id, text, slice.title)
        }
        EntryKind::AllergyIntolerance
        | EntryKind::Appointment
        | EntryKind::DiagnosticReportLab
        | EntryKind::MedicationStatement
        | EntryKind::ServiceRequest { .. } => {
            unreachable!("discharge summary slices do not use these entry kinds")
        }
    }
}

fn build_composition_shell(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    status: &str,
    attester_practitioner_id: Option<&str>,
) -> Value {
    let date = now_datetime();
    let mut composition = json!({
        "resourceType": "Composition",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_DISCHARGE_SUMMARY_RECORD] },
        "status": status,
        "type": {
            "coding": [{
                "system": SNOMED,
                "code": "373942005",
                "display": "Discharge summary"
            }]
        },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "date": date,
        "author": [{ "reference": format!("Practitioner/{practitioner_id}") }],
        "title": title,
        "text": section_text_div(&format!("Discharge summary: {title}"))
    });

    if let Some(practitioner_id) = attester_practitioner_id {
        composition["attester"] = json!([{
            "mode": "professional",
            "time": date,
            "party": { "reference": format!("Practitioner/{practitioner_id}") }
        }]);
    }

    composition
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discharge_sections_follow_profile_order() {
        let sections = DischargeSummarySections {
            chief_complaint: Some("Chest pain".into()),
            exam: Some("Stable vitals".into()),
            hospital_course: Some("Rule out ACS".into()),
            investigations: Some("Troponin negative".into()),
            discharge_medications: Some("Aspirin 75mg".into()),
            procedures: Some("Coronary angiography".into()),
            care_plan: Some("Cardiology follow up".into()),
        };
        let (comp, entries) = build_with_entries(
            "comp-ds-1",
            "pat-1",
            "enc-1",
            "dr-patel",
            "Discharge Summary",
            &sections,
            "preliminary",
            None,
        );
        assert_eq!(comp["meta"]["profile"][0], ATRIUS_IN_DISCHARGE_SUMMARY_RECORD);
        assert_eq!(entries.len(), 8);
        let inv_entry = comp["section"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["code"]["coding"][0]["code"] == "721981007")
            .expect("investigations section");
        assert!(
            inv_entry["entry"][0]["reference"]
                .as_str()
                .unwrap()
                .starts_with("DiagnosticReport/")
        );
        let med_entry = comp["section"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["code"]["coding"][0]["code"] == "1003606003")
            .expect("medications section");
        assert!(
            med_entry["entry"][0]["reference"]
                .as_str()
                .unwrap()
                .starts_with("MedicationRequest/")
        );
        let codes: Vec<&str> = comp["section"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["code"]["coding"][0]["code"].as_str().unwrap())
            .collect();
        assert_eq!(codes, profile_slice_codes());
    }
}
