//! Shared SNOMED section-sliced Composition builder (Pattern A).

use serde_json::{Value, json};

use crate::adt::now_datetime;
use crate::clinical::entry_builders::{
    SNOMED, follow_up_appointment, narrative_allergy_intolerance, narrative_care_plan,
    narrative_condition, narrative_diagnostic_report_lab, narrative_lab_result_observation,
    narrative_medication_request, narrative_medication_statement, narrative_observation,
    narrative_procedure, narrative_service_request, section_text_div,
};
use crate::clinical::slice::{EntryKind, SnomedSliceDef};
use crate::clinical::transaction::{create_transaction, update_transaction};

pub struct SnomedDocumentMeta {
    pub profile: &'static str,
    pub type_code: &'static str,
    pub type_display: &'static str,
}

#[must_use]
pub fn snomed_document_transaction<S>(
    meta: &SnomedDocumentMeta,
    slices: &[SnomedSliceDef<S>],
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &S,
) -> Value {
    let (composition, entry_resources) = build_with_entries(
        meta,
        slices,
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
pub fn snomed_document_update_transaction<S>(
    meta: &SnomedDocumentMeta,
    slices: &[SnomedSliceDef<S>],
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &S,
) -> Value {
    let composition_id = composition
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let (updated, entry_resources) = build_with_entries(
        meta,
        slices,
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

fn build_with_entries<S>(
    meta: &SnomedDocumentMeta,
    slices: &[SnomedSliceDef<S>],
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &S,
    status: &str,
    attester_practitioner_id: Option<&str>,
) -> (Value, Vec<Value>) {
    let mut section_entries = Vec::new();
    let mut entry_resources = Vec::new();

    for slice in slices {
        let Some(text) = (slice.field)(sections).filter(|t| !t.trim().is_empty()) else {
            continue;
        };
        let resource_id = format!("{composition_id}-{}", slice.id_suffix);
        match slice.entry {
            EntryKind::DiagnosticReportLab => {
                let result_id = format!("{resource_id}-result");
                entry_resources.push(narrative_lab_result_observation(
                    &result_id,
                    patient_id,
                    encounter_id,
                    text,
                    slice.title,
                ));
                entry_resources.push(narrative_diagnostic_report_lab(
                    &resource_id,
                    patient_id,
                    encounter_id,
                    practitioner_id,
                    text,
                    slice.title,
                    &result_id,
                ));
            }
            _ => {
                entry_resources.push(build_entry_resource(
                    slice,
                    &resource_id,
                    patient_id,
                    encounter_id,
                    practitioner_id,
                    text,
                ));
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
        meta,
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

fn build_entry_resource<S>(
    slice: &SnomedSliceDef<S>,
    resource_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    text: &str,
) -> Value {
    match slice.entry {
        EntryKind::Condition => {
            narrative_condition(resource_id, patient_id, encounter_id, text, slice.title)
        }
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
        EntryKind::MedicationRequest => narrative_medication_request(
            resource_id,
            patient_id,
            encounter_id,
            practitioner_id,
            text,
        ),
        EntryKind::Appointment => {
            follow_up_appointment(resource_id, patient_id, practitioner_id, text)
        }
        EntryKind::Procedure => {
            narrative_procedure(resource_id, patient_id, encounter_id, text, slice.title)
        }
        EntryKind::CarePlan => {
            narrative_care_plan(resource_id, patient_id, encounter_id, text, slice.title)
        }
        EntryKind::DiagnosticReportLab => {
            unreachable!("handled in build_with_entries loop")
        }
    }
}

fn build_composition_shell(
    meta: &SnomedDocumentMeta,
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
        "meta": { "profile": [meta.profile] },
        "status": status,
        "type": {
            "coding": [{
                "system": SNOMED,
                "code": meta.type_code,
                "display": meta.type_display
            }]
        },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "date": date,
        "author": [{ "reference": format!("Practitioner/{practitioner_id}") }],
        "title": title,
        "text": section_text_div(&format!("{}: {title}", meta.type_display))
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
