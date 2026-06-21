//! Consultation note (NDHM OPConsultRecord) builders for Phase 5a clinical documentation.

use serde_json::{Value, json};

use crate::adt::now_datetime;
use crate::profiles::{
    ATRIUS_IN_CONDITION, ATRIUS_IN_CONSULT_FOLLOW_UP_APPOINTMENT, ATRIUS_IN_OBSERVATION,
    ATRIUS_IN_OP_CONSULT_RECORD,
};

const SNOMED: &str = "http://snomed.info/sct";
const CONDITION_CLINICAL: &str = "http://terminology.hl7.org/CodeSystem/condition-clinical";
const CONDITION_VER: &str = "http://terminology.hl7.org/CodeSystem/condition-ver-status";
const OBS_CATEGORY: &str = "http://terminology.hl7.org/CodeSystem/observation-category";

/// SOAP-style consultation note sections (plain text; rendered to section narrative and entry resources).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ConsultNoteSections {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chief_complaint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hpi: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exam: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assessment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
}

impl ConsultNoteSections {
    pub fn has_content(&self) -> bool {
        [
            &self.chief_complaint,
            &self.hpi,
            &self.exam,
            &self.assessment,
            &self.plan,
        ]
        .iter()
        .any(|s| s.as_deref().is_some_and(|t| !t.trim().is_empty()))
    }
}

struct OpConsultSlice {
    slice: &'static str,
    title: &'static str,
    code: &'static str,
    display: &'static str,
    field: fn(&ConsultNoteSections) -> Option<&String>,
    resource_type: &'static str,
    id_suffix: &'static str,
}

// Slice order matches atrius-in-op-consult-record (openAtEnd section slicing).
const OP_CONSULT_SLICES: [OpConsultSlice; 5] = [
    OpConsultSlice {
        slice: "ChiefComplaints",
        title: "Chief complaints",
        code: "422843007",
        display: "Chief complaint section",
        field: |s| s.chief_complaint.as_ref(),
        resource_type: "Condition",
        id_suffix: "cc",
    },
    OpConsultSlice {
        slice: "PhysicalExamination",
        title: "Physical examination",
        code: "425044008",
        display: "Physical exam section",
        field: |s| s.exam.as_ref(),
        resource_type: "Observation",
        id_suffix: "exam",
    },
    OpConsultSlice {
        slice: "MedicalHistory",
        title: "Medical history",
        code: "371529009",
        display: "History and physical report",
        field: |s| s.hpi.as_ref(),
        resource_type: "Condition",
        id_suffix: "hpi",
    },
    OpConsultSlice {
        slice: "FollowUp",
        title: "Follow up",
        code: "390906007",
        display: "Follow-up encounter",
        field: |s| s.plan.as_ref(),
        resource_type: "Appointment",
        id_suffix: "plan",
    },
    OpConsultSlice {
        slice: "OtherObservations",
        title: "Assessment",
        code: "404684003",
        display: "Clinical finding",
        field: |s| s.assessment.as_ref(),
        resource_type: "Observation",
        id_suffix: "assessment",
    },
];

/// Build a preliminary OP consult Composition (NDHM OPConsultRecord shape).
#[must_use]
pub fn build_consultation_composition(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ConsultNoteSections,
) -> Value {
    build_op_consult_with_entries(
        id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        sections,
        "preliminary",
        None,
    )
    .0
}

/// FHIR transaction bundle: section entry resources + OP consult Composition.
#[must_use]
pub fn op_consult_transaction(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ConsultNoteSections,
) -> Value {
    let (composition, entry_resources) = build_op_consult_with_entries(
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        sections,
        "preliminary",
        None,
    );

    let mut entries: Vec<Value> = entry_resources
        .into_iter()
        .map(|resource| {
            let resource_type = resource
                .get("resourceType")
                .and_then(|v| v.as_str())
                .unwrap_or("Resource");
            let id = resource
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            json!({
                "fullUrl": format!("urn:uuid:{id}"),
                "resource": resource,
                "request": {
                    "method": "POST",
                    "url": resource_type
                }
            })
        })
        .collect();

    entries.push(json!({
        "fullUrl": format!("urn:uuid:{composition_id}"),
        "resource": composition,
        "request": {
            "method": "POST",
            "url": "Composition"
        }
    }));

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": entries
    })
}

/// Transaction bundle to replace section entry resources and update the Composition.
#[must_use]
pub fn op_consult_update_transaction(
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ConsultNoteSections,
) -> Value {
    let composition_id = composition
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let (updated, entry_resources) = build_op_consult_with_entries(
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        sections,
        "preliminary",
        None,
    );

    let mut entries: Vec<Value> = entry_resources
        .into_iter()
        .map(|resource| {
            let resource_type = resource
                .get("resourceType")
                .and_then(|v| v.as_str())
                .unwrap_or("Resource");
            let id = resource
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            json!({
                "resource": resource,
                "request": {
                    "method": "PUT",
                    "url": format!("{resource_type}/{id}")
                }
            })
        })
        .collect();

    entries.push(json!({
        "resource": updated,
        "request": {
            "method": "PUT",
            "url": format!("Composition/{composition_id}")
        }
    }));

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": entries
    })
}

/// Merge section updates into an existing preliminary Composition (and entry resource bodies).
#[must_use]
pub fn merge_consultation_sections(
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    sections: &ConsultNoteSections,
) -> (Value, Vec<Value>) {
    let composition_id = composition
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let title = composition
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Consultation Note")
        .to_string();
    build_op_consult_with_entries(
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        &title,
        sections,
        "preliminary",
        None,
    )
}

/// Mark a consultation Composition final with professional attestation.
#[must_use]
pub fn finalize_consultation_composition(composition: &Value, practitioner_id: &str) -> Value {
    let mut updated = composition.clone();
    let attested = now_datetime();
    updated["status"] = json!("final");
    updated["attester"] = json!([{
        "mode": "professional",
        "time": attested,
        "party": { "reference": format!("Practitioner/{practitioner_id}") }
    }]);
    updated
}

/// Extract the Composition from a transaction response bundle.
#[must_use]
pub fn composition_from_transaction_response(response: &Value) -> Option<Value> {
    response
        .get("entry")
        .and_then(|e| e.as_array())
        .and_then(|entries| {
            entries.iter().find_map(|entry| {
                let resource = entry.get("resource")?;
                if resource.get("resourceType")?.as_str()? == "Composition" {
                    Some(resource.clone())
                } else {
                    None
                }
            })
        })
}

/// Patient id from Composition.subject reference.
pub fn composition_patient_id(composition: &Value) -> Option<String> {
    composition
        .get("subject")
        .and_then(|s| s.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("Patient/"))
        .map(str::to_string)
}

/// Encounter id from Composition.encounter reference.
pub fn composition_encounter_id(composition: &Value) -> Option<String> {
    composition
        .get("encounter")
        .and_then(|e| e.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("Encounter/"))
        .map(str::to_string)
}

fn build_op_consult_with_entries(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ConsultNoteSections,
    status: &str,
    attester_practitioner_id: Option<&str>,
) -> (Value, Vec<Value>) {
    let mut section_entries = Vec::new();
    let mut entry_resources = Vec::new();

    for slice in OP_CONSULT_SLICES {
        let Some(text) = (slice.field)(sections).filter(|t| !t.trim().is_empty()) else {
            continue;
        };
        let resource_id = format!("{composition_id}-{}", slice.id_suffix);
        let resource = match slice.resource_type {
            "Condition" => narrative_condition(
                &resource_id,
                patient_id,
                encounter_id,
                text,
                slice.title,
            ),
            "Observation" => narrative_observation(
                &resource_id,
                patient_id,
                encounter_id,
                text,
                slice.title,
                slice.slice == "PhysicalExamination",
            ),
            "Appointment" => follow_up_appointment(
                &resource_id,
                patient_id,
                practitioner_id,
                text,
            ),
            _ => continue,
        };
        entry_resources.push(resource);
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
                "reference": format!("{}/{resource_id}", slice.resource_type)
            }]
        }));
    }

    let composition = build_op_consult_composition(
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        sections,
        status,
        attester_practitioner_id,
    );
    let mut composition = composition;
    composition["section"] = json!(section_entries);
    (composition, entry_resources)
}

fn build_op_consult_composition(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ConsultNoteSections,
    status: &str,
    attester_practitioner_id: Option<&str>,
) -> Value {
    let date = now_datetime();
    let mut composition = json!({
        "resourceType": "Composition",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_OP_CONSULT_RECORD] },
        "status": status,
        "type": {
            "coding": [{
                "system": SNOMED,
                "code": "371530004",
                "display": "Clinical consultation report"
            }]
        },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "date": date,
        "author": [{ "reference": format!("Practitioner/{practitioner_id}") }],
        "title": title,
        "text": composition_narrative_div(title, sections)
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

fn narrative_condition(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    text: &str,
    label: &str,
) -> Value {
    json!({
        "resourceType": "Condition",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_CONDITION] },
        "clinicalStatus": {
            "coding": [{ "system": CONDITION_CLINICAL, "code": "active", "display": "Active" }]
        },
        "verificationStatus": {
            "coding": [{ "system": CONDITION_VER, "code": "provisional", "display": "Provisional" }]
        },
        "code": { "text": text },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "recordedDate": now_datetime(),
        "text": section_text_div(&format!("{label}: {text}"))
    })
}

fn narrative_observation(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    text: &str,
    label: &str,
    exam_category: bool,
) -> Value {
    let category_code = if exam_category { "exam" } else { "survey" };
    json!({
        "resourceType": "Observation",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_OBSERVATION] },
        "status": "final",
        "category": [{
            "coding": [{
                "system": OBS_CATEGORY,
                "code": category_code,
                "display": if exam_category { "Exam" } else { "Survey" }
            }]
        }],
        "code": { "text": label },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "effectiveDateTime": now_datetime(),
        "valueString": text,
        "text": section_text_div(text)
    })
}

fn follow_up_appointment(
    id: &str,
    patient_id: &str,
    practitioner_id: &str,
    plan: &str,
) -> Value {
    json!({
        "resourceType": "Appointment",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_CONSULT_FOLLOW_UP_APPOINTMENT] },
        "status": "proposed",
        "description": plan,
        "participant": [
            {
                "actor": { "reference": format!("Patient/{patient_id}") },
                "status": "accepted"
            },
            {
                "actor": { "reference": format!("Practitioner/{practitioner_id}") },
                "status": "accepted"
            }
        ],
        "text": section_text_div(plan)
    })
}

fn composition_narrative_div(title: &str, sections: &ConsultNoteSections) -> Value {
    let mut parts = vec![format!("Composition {title}")];
    if let Some(s) = sections.chief_complaint.as_deref().filter(|t| !t.is_empty()) {
        parts.push(format!("Chief complaint: {s}"));
    }
    if let Some(s) = sections.assessment.as_deref().filter(|t| !t.is_empty()) {
        parts.push(format!("Assessment: {s}"));
    }
    section_text_div(&parts.join(". "))
}

fn section_text_div(text: &str) -> Value {
    const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    json!({
        "status": "generated",
        "div": format!(r#"<div xmlns="{XHTML_NS}"><p>{escaped}</p></div>"#)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_sections() -> ConsultNoteSections {
        ConsultNoteSections {
            chief_complaint: Some("Headache".into()),
            hpi: Some("3 days, non-focal".into()),
            exam: Some("Normal neuro exam".into()),
            assessment: Some("Tension headache".into()),
            plan: Some("Analgesia and follow up".into()),
        }
    }

    #[test]
    fn builds_preliminary_op_consult_composition_with_entries() {
        let sections = sample_sections();
        let (comp, entries) = build_op_consult_with_entries(
            "comp-1",
            "pat-1",
            "enc-1",
            "dr-patel",
            "OPD Consultation",
            &sections,
            "preliminary",
            None,
        );
        assert_eq!(comp["status"], "preliminary");
        assert_eq!(comp["meta"]["profile"][0], ATRIUS_IN_OP_CONSULT_RECORD);
        assert_eq!(comp["type"]["coding"][0]["code"], "371530004");
        assert_eq!(comp["section"].as_array().unwrap().len(), 5);
        assert_eq!(entries.len(), 5);
        assert!(entries.iter().any(|r| r["resourceType"] == "Condition"));
        assert!(entries.iter().any(|r| r["resourceType"] == "Observation"));
        assert!(entries.iter().any(|r| r["resourceType"] == "Appointment"));
    }

    #[test]
    fn op_consult_sections_follow_profile_slice_order() {
        let sections = sample_sections();
        let (comp, _) = build_op_consult_with_entries(
            "comp-1",
            "pat-1",
            "enc-1",
            "dr-patel",
            "OPD Consultation",
            &sections,
            "preliminary",
            None,
        );
        let codes: Vec<&str> = comp["section"]
            .as_array()
            .unwrap()
            .iter()
            .map(|section| {
                section["code"]["coding"][0]["code"]
                    .as_str()
                    .expect("section SNOMED code")
            })
            .collect();
        assert_eq!(
            codes,
            ["422843007", "425044008", "371529009", "390906007", "404684003"]
        );
    }

    #[test]
    fn op_consult_transaction_includes_composition_and_entries() {
        let bundle = op_consult_transaction(
            "comp-1",
            "pat-1",
            "enc-1",
            "dr-patel",
            "OPD Consultation",
            &sample_sections(),
        );
        assert_eq!(bundle["type"], "transaction");
        assert_eq!(bundle["entry"].as_array().unwrap().len(), 6);
    }

    #[test]
    fn finalize_adds_attester() {
        let comp = build_consultation_composition(
            "comp-1",
            "pat-1",
            "enc-1",
            "dr-patel",
            "Note",
            &ConsultNoteSections {
                plan: Some("Follow up".into()),
                ..Default::default()
            },
        );
        let final_comp = finalize_consultation_composition(&comp, "dr-patel");
        assert_eq!(final_comp["status"], "final");
        assert_eq!(final_comp["attester"][0]["mode"], "professional");
    }
}
