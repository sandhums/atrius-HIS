//! OP consult Composition (NDHM OPConsultRecord shape).

use serde_json::{Value, json};

use crate::adt::now_datetime;
use crate::clinical::entry_builders::{
    SNOMED, follow_up_appointment, narrative_allergy_intolerance, narrative_condition,
    narrative_medication_statement, narrative_observation, narrative_service_request,
    section_text_div,
};
use crate::clinical::slice::{EntryKind, SnomedSliceDef};
use crate::clinical::transaction::{create_transaction, update_transaction};
use crate::profiles::ATRIUS_IN_OP_CONSULT_RECORD;

/// SOAP-style consultation note sections (plain text → section narrative + entry resources).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ConsultNoteSections {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chief_complaint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hpi: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exam: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allergies: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub investigations: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub medications: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assessment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referral: Option<String>,
}

impl ConsultNoteSections {
    pub fn has_content(&self) -> bool {
        [
            &self.chief_complaint,
            &self.hpi,
            &self.exam,
            &self.allergies,
            &self.investigations,
            &self.medications,
            &self.assessment,
            &self.plan,
            &self.referral,
        ]
        .iter()
        .any(|s| s.as_deref().is_some_and(|t| !t.trim().is_empty()))
    }
}

// Slice order matches atrius-in-op-consult-record (ordered openAtEnd section slicing).
const OP_CONSULT_SLICES: [SnomedSliceDef<ConsultNoteSections>; 9] = [
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
        slice: "Allergies",
        title: "Allergies",
        code: "722446000",
        display: "Allergy record",
        field: |s| s.allergies.as_ref(),
        entry: EntryKind::AllergyIntolerance,
        id_suffix: "allergy",
    },
    SnomedSliceDef {
        slice: "MedicalHistory",
        title: "Medical history",
        code: "371529009",
        display: "History and physical report",
        field: |s| s.hpi.as_ref(),
        entry: EntryKind::Condition,
        id_suffix: "hpi",
    },
    SnomedSliceDef {
        slice: "InvestigationAdvice",
        title: "Investigations advised",
        code: "721963009",
        display: "Order document",
        field: |s| s.investigations.as_ref(),
        entry: EntryKind::ServiceRequest {
            category: "108252007",
            display: "Laboratory procedure",
        },
        id_suffix: "investigations",
    },
    SnomedSliceDef {
        slice: "Medications",
        title: "Medications",
        code: "721912009",
        display: "Medication summary document",
        field: |s| s.medications.as_ref(),
        entry: EntryKind::MedicationStatement,
        id_suffix: "medications",
    },
    SnomedSliceDef {
        slice: "FollowUp",
        title: "Follow up",
        code: "390906007",
        display: "Follow-up encounter",
        field: |s| s.plan.as_ref(),
        entry: EntryKind::Appointment,
        id_suffix: "plan",
    },
    SnomedSliceDef {
        slice: "Referral",
        title: "Referral",
        code: "306206005",
        display: "Referral to service",
        field: |s| s.referral.as_ref(),
        entry: EntryKind::ServiceRequest {
            category: "3457005",
            display: "Patient referral",
        },
        id_suffix: "referral",
    },
    SnomedSliceDef {
        slice: "OtherObservations",
        title: "Assessment",
        code: "404684003",
        display: "Clinical finding",
        field: |s| s.assessment.as_ref(),
        entry: EntryKind::Observation { exam: false },
        id_suffix: "assessment",
    },
];

pub fn profile_slice_codes() -> &'static [&'static str] {
    &[
        "422843007", "425044008", "722446000", "371529009", "721963009", "721912009",
        "390906007", "306206005", "404684003",
    ]
}

#[must_use]
pub fn build_consultation_composition(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ConsultNoteSections,
) -> Value {
    build_with_entries(
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

#[must_use]
pub fn op_consult_transaction(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &ConsultNoteSections,
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
    build_with_entries(
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

fn build_with_entries(
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
        let resource = build_entry_resource(
            &slice,
            &resource_id,
            patient_id,
            encounter_id,
            practitioner_id,
            text,
        );
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
        sections,
        status,
        attester_practitioner_id,
    );
    composition["section"] = json!(section_entries);
    (composition, entry_resources)
}

fn build_entry_resource(
    slice: &SnomedSliceDef<ConsultNoteSections>,
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
        EntryKind::AllergyIntolerance => {
            narrative_allergy_intolerance(resource_id, patient_id, encounter_id, text)
        }
        EntryKind::ServiceRequest { category, display } => narrative_service_request(
            resource_id,
            patient_id,
            encounter_id,
            practitioner_id,
            text,
            category,
            display,
        ),
        EntryKind::MedicationStatement => {
            narrative_medication_statement(resource_id, patient_id, encounter_id, text)
        }
        EntryKind::Appointment => follow_up_appointment(
            resource_id,
            patient_id,
            practitioner_id,
            text,
        ),
        EntryKind::Procedure
        | EntryKind::CarePlan
        | EntryKind::DiagnosticReportLab
        | EntryKind::MedicationRequest => {
            unreachable!("OP consult slices do not use Procedure or CarePlan entries")
        }
    }
}

fn build_composition_shell(
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

fn composition_narrative_div(title: &str, sections: &ConsultNoteSections) -> Value {
    let mut parts = vec![format!("Composition {title}")];
    for (label, text) in [
        ("Chief complaint", sections.chief_complaint.as_deref()),
        ("History", sections.hpi.as_deref()),
        ("Examination", sections.exam.as_deref()),
        ("Allergies", sections.allergies.as_deref()),
        ("Investigations", sections.investigations.as_deref()),
        ("Medications", sections.medications.as_deref()),
        ("Assessment", sections.assessment.as_deref()),
        ("Plan", sections.plan.as_deref()),
        ("Referral", sections.referral.as_deref()),
    ] {
        if let Some(text) = text.filter(|t| !t.is_empty()) {
            parts.push(format!("{label}: {text}"));
        }
    }
    section_text_div(&parts.join(". "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_sections() -> ConsultNoteSections {
        ConsultNoteSections {
            chief_complaint: Some("Headache".into()),
            hpi: Some("3 days, non-focal".into()),
            exam: Some("Normal neuro exam".into()),
            allergies: Some("No known drug allergies".into()),
            investigations: Some("CBC if persistent".into()),
            medications: Some("Paracetamol PRN".into()),
            assessment: Some("Tension headache".into()),
            plan: Some("Analgesia and follow up".into()),
            referral: Some("Neurology if worsening".into()),
        }
    }

    #[test]
    fn builds_preliminary_op_consult_with_all_slices() {
        let sections = sample_sections();
        let (comp, entries) = build_with_entries(
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
        assert_eq!(comp["section"].as_array().unwrap().len(), 9);
        assert_eq!(entries.len(), 9);
    }

    #[test]
    fn op_consult_sections_follow_profile_slice_order() {
        let sections = sample_sections();
        let (comp, _) = build_with_entries(
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
        assert_eq!(codes, profile_slice_codes());
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
        assert_eq!(bundle["entry"].as_array().unwrap().len(), 10);
    }
}
