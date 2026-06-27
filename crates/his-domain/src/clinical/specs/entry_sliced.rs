//! Entry-type sliced Composition builders (Pattern C).

use serde_json::{Value, json};

use crate::adt::now_datetime;
use crate::clinical::entry_builders::{
    narrative_immunization, narrative_invoice, narrative_medication_request,
    narrative_observation_with_profile, section_text_div,
};
use crate::clinical::transaction::{create_transaction, update_transaction};

pub enum CompositionTypeCoding {
    Snomed {
        code: &'static str,
        display: &'static str,
    },
    Loinc {
        code: &'static str,
        display: &'static str,
    },
    Text(&'static str),
}

pub struct EntrySlicedMeta {
    pub profile: &'static str,
    pub composition_type: CompositionTypeCoding,
    pub title_narrative: &'static str,
}

impl EntrySlicedMeta {
    fn type_value(&self) -> Value {
        match self.composition_type {
            CompositionTypeCoding::Snomed { code, display } => json!({
                "coding": [{
                    "system": "http://snomed.info/sct",
                    "code": code,
                    "display": display
                }]
            }),
            CompositionTypeCoding::Loinc { code, display } => json!({
                "coding": [{
                    "system": "http://loinc.org",
                    "code": code,
                    "display": display
                }]
            }),
            CompositionTypeCoding::Text(text) => json!({ "text": text }),
        }
    }
}

#[must_use]
pub fn entry_sliced_transaction(
    meta: &EntrySlicedMeta,
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    section_text: &str,
    entry_resources: Vec<Value>,
    entry_refs: Vec<Value>,
) -> Value {
    let composition = build_composition(
        meta,
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        section_text,
        entry_refs,
        "preliminary",
        None,
    );
    create_transaction(composition_id, composition, entry_resources)
}

#[must_use]
pub fn entry_sliced_update_transaction(
    meta: &EntrySlicedMeta,
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    section_text: &str,
    entry_resources: Vec<Value>,
    entry_refs: Vec<Value>,
) -> Value {
    let composition_id = composition
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let updated = build_composition(
        meta,
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        section_text,
        entry_refs,
        "preliminary",
        None,
    );
    update_transaction(composition_id, updated, entry_resources)
}

fn build_composition(
    meta: &EntrySlicedMeta,
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    section_text: &str,
    entry_refs: Vec<Value>,
    status: &str,
    attester_practitioner_id: Option<&str>,
) -> Value {
    let date = now_datetime();
    let mut composition = json!({
        "resourceType": "Composition",
        "id": id,
        "meta": { "profile": [meta.profile] },
        "status": status,
        "type": meta.type_value(),
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "date": date,
        "author": [{ "reference": format!("Practitioner/{practitioner_id}") }],
        "title": title,
        "text": section_text_div(&format!("{}: {title}", meta.title_narrative)),
        "section": [{
            "text": section_text_div(section_text),
            "entry": entry_refs
        }]
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

pub fn build_medication_entries(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    medications: &[String],
) -> (Vec<Value>, Vec<Value>) {
    let mut resources = Vec::new();
    let mut refs = Vec::new();
    for (idx, text) in medications.iter().filter(|m| !m.trim().is_empty()).enumerate() {
        let id = format!("{composition_id}-rx-{idx}");
        resources.push(narrative_medication_request(
            &id,
            patient_id,
            encounter_id,
            practitioner_id,
            text,
        ));
        refs.push(json!({
            "type": "MedicationRequest",
            "reference": format!("MedicationRequest/{id}")
        }));
    }
    (resources, refs)
}

pub fn build_immunization_entries(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    vaccines: &[String],
) -> (Vec<Value>, Vec<Value>) {
    let mut resources = Vec::new();
    let mut refs = Vec::new();
    for (idx, text) in vaccines.iter().filter(|v| !v.trim().is_empty()).enumerate() {
        let id = format!("{composition_id}-imm-{idx}");
        resources.push(narrative_immunization(
            &id,
            patient_id,
            encounter_id,
            practitioner_id,
            text,
        ));
        refs.push(json!({
            "type": "Immunization",
            "reference": format!("Immunization/{id}")
        }));
    }
    (resources, refs)
}

pub fn build_invoice_entry(
    composition_id: &str,
    patient_id: &str,
    summary: &str,
    amount_inr: Option<&str>,
) -> (Vec<Value>, Vec<Value>) {
    let id = format!("{composition_id}-invoice");
    let invoice = narrative_invoice(&id, patient_id, summary, amount_inr);
    (
        vec![invoice],
        vec![json!({
            "type": "Invoice",
            "reference": format!("Invoice/{id}")
        })],
    )
}

pub struct TitleSliceDef<S> {
    pub title: &'static str,
    pub profile: &'static str,
    pub category_code: &'static str,
    pub category_display: &'static str,
    pub field: fn(&S) -> Option<&String>,
    pub id_suffix: &'static str,
}

pub fn build_title_sliced_sections<S>(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    slices: &[TitleSliceDef<S>],
    sections: &S,
) -> (Vec<Value>, Vec<Value>) {
    let mut section_entries = Vec::new();
    let mut entry_resources = Vec::new();

    for slice in slices {
        let Some(text) = (slice.field)(sections).filter(|t| !t.trim().is_empty()) else {
            continue;
        };
        let resource_id = format!("{composition_id}-{}", slice.id_suffix);
        entry_resources.push(narrative_observation_with_profile(
            &resource_id,
            patient_id,
            encounter_id,
            text,
            slice.title,
            slice.profile,
            slice.category_code,
            slice.category_display,
        ));
        section_entries.push(json!({
            "title": slice.title,
            "text": section_text_div(text),
            "entry": [{ "reference": format!("Observation/{resource_id}") }]
        }));
    }

    (section_entries, entry_resources)
}

#[must_use]
pub fn title_sliced_transaction<S>(
    meta: &EntrySlicedMeta,
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &S,
    slices: &[TitleSliceDef<S>],
) -> Value {
    let (section_entries, entry_resources) =
        build_title_sliced_sections(composition_id, patient_id, encounter_id, slices, sections);
    let date = now_datetime();
    let composition = json!({
        "resourceType": "Composition",
        "id": composition_id,
        "meta": { "profile": [meta.profile] },
        "status": "preliminary",
        "type": meta.type_value(),
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "date": date,
        "author": [{ "reference": format!("Practitioner/{practitioner_id}") }],
        "title": title,
        "text": section_text_div(&format!("{}: {title}", meta.title_narrative)),
        "section": section_entries
    });
    create_transaction(composition_id, composition, entry_resources)
}

#[must_use]
pub fn title_sliced_update_transaction<S>(
    meta: &EntrySlicedMeta,
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &S,
    slices: &[TitleSliceDef<S>],
) -> Value {
    let composition_id = composition
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let (section_entries, entry_resources) =
        build_title_sliced_sections(composition_id, patient_id, encounter_id, slices, sections);
    let date = now_datetime();
    let updated = json!({
        "resourceType": "Composition",
        "id": composition_id,
        "meta": { "profile": [meta.profile] },
        "status": "preliminary",
        "type": meta.type_value(),
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "date": date,
        "author": [{ "reference": format!("Practitioner/{practitioner_id}") }],
        "title": title,
        "text": section_text_div(&format!("{}: {title}", meta.title_narrative)),
        "section": section_entries
    });
    update_transaction(composition_id, updated, entry_resources)
}
