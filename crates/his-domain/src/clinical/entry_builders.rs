//! Narrative entry resource builders shared across clinical document specs.

use serde_json::{Value, json};

use crate::adt::now_datetime;
use crate::profiles::{
    ATRIUS_IN_ALLERGY_INTOLERANCE, ATRIUS_IN_CARE_PLAN, ATRIUS_IN_CONDITION,
    ATRIUS_IN_CONSULT_FOLLOW_UP_APPOINTMENT, ATRIUS_IN_DIAGNOSTIC_REPORT_LAB,
    ATRIUS_IN_IMMUNIZATION, ATRIUS_IN_INVOICE, ATRIUS_IN_MEDICATION_REQUEST,
    ATRIUS_IN_MEDICATION_STATEMENT, ATRIUS_IN_OBSERVATION, ATRIUS_IN_PROCEDURE,
    ATRIUS_IN_SERVICE_REQUEST,
};

pub const SNOMED: &str = "http://snomed.info/sct";
const CONDITION_CLINICAL: &str = "http://terminology.hl7.org/CodeSystem/condition-clinical";
const CONDITION_VER: &str = "http://terminology.hl7.org/CodeSystem/condition-ver-status";
const OBS_CATEGORY: &str = "http://terminology.hl7.org/CodeSystem/observation-category";
const ALLERGY_CLINICAL: &str = "http://terminology.hl7.org/CodeSystem/allergyintolerance-clinical";
const ALLERGY_VER: &str = "http://terminology.hl7.org/CodeSystem/allergyintolerance-verification";
const SR_CATEGORY: &str = "http://terminology.hl7.org/CodeSystem/service-request-category";
const DIAGNOSTIC_SERVICE: &str = "http://terminology.hl7.org/CodeSystem/v2-0074";

#[must_use]
pub fn section_text_div(text: &str) -> Value {
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

#[must_use]
pub fn narrative_condition(
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

#[must_use]
pub fn narrative_observation(
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

#[must_use]
pub fn follow_up_appointment(
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

#[must_use]
pub fn narrative_allergy_intolerance(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    text: &str,
) -> Value {
    json!({
        "resourceType": "AllergyIntolerance",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_ALLERGY_INTOLERANCE] },
        "clinicalStatus": {
            "coding": [{ "system": ALLERGY_CLINICAL, "code": "active", "display": "Active" }]
        },
        "verificationStatus": {
            "coding": [{ "system": ALLERGY_VER, "code": "unconfirmed", "display": "Unconfirmed" }]
        },
        "code": { "text": text },
        "patient": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "recordedDate": now_datetime(),
        "text": section_text_div(text)
    })
}

#[must_use]
pub fn narrative_service_request(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    text: &str,
    category_code: &str,
    category_display: &str,
) -> Value {
    json!({
        "resourceType": "ServiceRequest",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_SERVICE_REQUEST] },
        "status": "active",
        "intent": "order",
        "category": [{
            "coding": [{
                "system": SR_CATEGORY,
                "code": category_code,
                "display": category_display
            }]
        }],
        "code": { "text": text },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "requester": { "reference": format!("Practitioner/{practitioner_id}") },
        "authoredOn": now_datetime(),
        "text": section_text_div(text)
    })
}

#[must_use]
pub fn narrative_medication_statement(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    text: &str,
) -> Value {
    json!({
        "resourceType": "MedicationStatement",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_MEDICATION_STATEMENT] },
        "status": "active",
        "medicationCodeableConcept": { "text": text },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "context": { "reference": format!("Encounter/{encounter_id}") },
        "dateAsserted": now_datetime(),
        "text": section_text_div(text)
    })
}

#[must_use]
pub fn narrative_medication_request(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    text: &str,
) -> Value {
    json!({
        "resourceType": "MedicationRequest",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_MEDICATION_REQUEST] },
        "status": "active",
        "intent": "order",
        "medicationCodeableConcept": {
            "text": text,
            "coding": [{
                "system": SNOMED,
                "code": "410942007",
                "display": "Drug or medicament"
            }]
        },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "authoredOn": now_datetime(),
        "requester": { "reference": format!("Practitioner/{practitioner_id}") },
        "reasonCode": [{
            "text": "Discharge medication",
            "coding": [{
                "system": SNOMED,
                "code": "308335008",
                "display": "Patient discharge"
            }]
        }],
        "dosageInstruction": [{
            "text": text,
            "additionalInstruction": [{
                "text": "As directed",
                "coding": [{
                    "system": SNOMED,
                    "code": "420848005",
                    "display": "Not applicable"
                }]
            }],
            "route": [{
                "text": "Oral route",
                "coding": [{
                    "system": SNOMED,
                    "code": "26643006",
                    "display": "Oral route"
                }]
            }],
            "method": [{
                "text": "Take",
                "coding": [{
                    "system": SNOMED,
                    "code": "419652001",
                    "display": "Take"
                }]
            }]
        }],
        "text": section_text_div(text)
    })
}

#[must_use]
pub fn narrative_lab_result_observation(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    text: &str,
    label: &str,
) -> Value {
    json!({
        "resourceType": "Observation",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_OBSERVATION] },
        "status": "final",
        "category": [{
            "coding": [{
                "system": OBS_CATEGORY,
                "code": "laboratory",
                "display": "Laboratory"
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

#[must_use]
pub fn narrative_diagnostic_report_lab(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    text: &str,
    label: &str,
    result_observation_id: &str,
) -> Value {
    let issued = now_datetime();
    json!({
        "resourceType": "DiagnosticReport",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_DIAGNOSTIC_REPORT_LAB] },
        "status": "final",
        "category": [
            {
                "coding": [{
                    "system": DIAGNOSTIC_SERVICE,
                    "code": "LAB",
                    "display": "Laboratory"
                }]
            },
            {
                "coding": [{
                    "system": OBS_CATEGORY,
                    "code": "laboratory",
                    "display": "Laboratory"
                }]
            }
        ],
        "code": { "text": label },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "effectiveDateTime": issued,
        "issued": issued,
        "performer": [{ "reference": format!("Practitioner/{practitioner_id}") }],
        "result": [{ "reference": format!("Observation/{result_observation_id}") }],
        "conclusion": text,
        "text": section_text_div(text)
    })
}

#[must_use]
pub fn narrative_procedure(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    text: &str,
    label: &str,
) -> Value {
    json!({
        "resourceType": "Procedure",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_PROCEDURE] },
        "status": "completed",
        "code": { "text": text },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "performedDateTime": now_datetime(),
        "text": section_text_div(&format!("{label}: {text}"))
    })
}

#[must_use]
pub fn narrative_care_plan(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    text: &str,
    title: &str,
) -> Value {
    json!({
        "resourceType": "CarePlan",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_CARE_PLAN] },
        "status": "active",
        "intent": "plan",
        "title": title,
        "description": text,
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "created": now_datetime(),
        "text": section_text_div(text)
    })
}

#[must_use]
pub fn narrative_observation_with_profile(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    text: &str,
    label: &str,
    profile: &str,
    category_code: &str,
    category_display: &str,
) -> Value {
    json!({
        "resourceType": "Observation",
        "id": id,
        "meta": { "profile": [profile] },
        "status": "final",
        "category": [{
            "coding": [{
                "system": OBS_CATEGORY,
                "code": category_code,
                "display": category_display
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

#[must_use]
pub fn narrative_immunization(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    vaccine_text: &str,
) -> Value {
    let now = now_datetime();
    json!({
        "resourceType": "Immunization",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_IMMUNIZATION] },
        "status": "completed",
        "statusReason": {
            "text": "Routine immunization",
            "coding": [{
                "system": SNOMED,
                "code": "33879002",
                "display": "Administration of vaccine to produce active immunity"
            }]
        },
        "vaccineCode": {
            "text": vaccine_text,
            "coding": [{
                "system": SNOMED,
                "code": "840536004",
                "display": "COVID-19 vaccine"
            }]
        },
        "patient": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "occurrenceDateTime": now,
        "recorded": now,
        "location": { "reference": format!("Encounter/{encounter_id}") },
        "manufacturer": { "display": "Vaccine manufacturer" },
        "lotNumber": "LOT-UNKNOWN",
        "expirationDate": "2099-12-31",
        "site": {
            "text": "Left upper arm",
            "coding": [{
                "system": SNOMED,
                "code": "368208006",
                "display": "Left upper arm structure"
            }]
        },
        "route": {
            "text": "Intramuscular route",
            "coding": [{
                "system": SNOMED,
                "code": "78421000",
                "display": "Intramuscular route"
            }]
        },
        "performer": [{
            "function": {
                "text": "Administering provider",
                "coding": [{
                    "system": SNOMED,
                    "code": "17561000",
                    "display": "Cardiologist"
                }]
            },
            "actor": { "reference": format!("Practitioner/{practitioner_id}") }
        }],
        "reasonCode": [{
            "text": "Routine immunization",
            "coding": [{
                "system": SNOMED,
                "code": "33879002",
                "display": "Administration of vaccine to produce active immunity"
            }]
        }],
        "protocolApplied": [{
            "doseNumberPositiveInt": 1
        }],
        "text": section_text_div(vaccine_text)
    })
}

#[must_use]
pub fn narrative_invoice(
    id: &str,
    patient_id: &str,
    summary: &str,
    amount_inr: Option<&str>,
) -> Value {
    let amount = amount_inr
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
    let now = now_datetime();
    json!({
        "resourceType": "Invoice",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_INVOICE] },
        "status": "issued",
        "subject": { "reference": format!("Patient/{patient_id}") },
        "date": now,
        "issuer": { "display": "Atrius Hospital" },
        "lineItem": [{
            "chargeItemCodeableConcept": { "text": summary },
            "priceComponent": [{
                "type": "base",
                "amount": { "value": amount, "currency": "INR" }
            }]
        }],
        "totalNet": { "value": amount, "currency": "INR" },
        "totalGross": { "value": amount, "currency": "INR" },
        "text": section_text_div(summary)
    })
}
