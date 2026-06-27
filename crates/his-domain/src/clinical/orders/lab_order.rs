//! LOINC-coded laboratory ServiceRequest builder for standalone CPOE orders.

use serde_json::{Value, json};

use crate::adt::now_datetime;
use crate::profiles::{ATRIUS_IN_DIAGNOSTIC_REPORT_LAB, ATRIUS_IN_OBSERVATION, ATRIUS_IN_SERVICE_REQUEST};

use super::super::entry_builders::section_text_div;

pub const LOINC: &str = "http://loinc.org";
const SR_CATEGORY: &str = "http://terminology.hl7.org/CodeSystem/service-request-category";
const OBS_CATEGORY: &str = "http://terminology.hl7.org/CodeSystem/observation-category";
const DIAGNOSTIC_SERVICE: &str = "http://terminology.hl7.org/CodeSystem/v2-0074";

/// Common lab tests from the Atrius IG ActivityDefinition catalog.
#[derive(Debug, Clone, Copy)]
pub struct LabCatalogEntry {
    pub loinc_code: &'static str,
    pub display: &'static str,
    pub title: &'static str,
}

pub const LAB_CATALOG: &[LabCatalogEntry] = &[
    LabCatalogEntry {
        loinc_code: "58410-2",
        display: "Complete blood count (hemogram) panel - Blood by Automated count",
        title: "Lab — Complete blood count",
    },
    LabCatalogEntry {
        loinc_code: "24323-8",
        display: "Comprehensive metabolic 2000 panel - Serum or Plasma",
        title: "Lab — Basic metabolic panel",
    },
    LabCatalogEntry {
        loinc_code: "10839-9",
        display: "Troponin I.cardiac [Mass/volume] in Serum or Plasma",
        title: "Lab — Cardiac troponin I",
    },
];

#[must_use]
pub fn lab_service_request_id() -> String {
    format!("lab-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}

#[must_use]
pub fn lab_fulfillment_task_id() -> String {
    format!("task-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}

#[must_use]
pub fn lab_result_observation_id() -> String {
    format!("obs-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}

#[must_use]
pub fn lab_result_report_id() -> String {
    format!("dr-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}

#[must_use]
pub fn resolve_lab_display(loinc_code: &str) -> Option<&'static str> {
    LAB_CATALOG
        .iter()
        .find(|entry| entry.loinc_code == loinc_code)
        .map(|entry| entry.display)
}

#[must_use]
pub fn is_lab_service_request(resource: &Value) -> bool {
    if resource.get("resourceType").and_then(|v| v.as_str()) != Some("ServiceRequest") {
        return false;
    }
    let profile = resource
        .get("meta")
        .and_then(|m| m.get("profile"))
        .and_then(|p| p.as_array())
        .and_then(|profiles| profiles.first())
        .and_then(|v| v.as_str());
    if profile != Some(ATRIUS_IN_SERVICE_REQUEST) {
        return false;
    }
    resource
        .get("category")
        .and_then(|c| c.as_array())
        .map(|categories| {
            categories.iter().any(|cat| {
                cat.get("coding")
                    .and_then(|c| c.as_array())
                    .map(|codings| {
                        codings.iter().any(|coding| {
                            coding.get("system").and_then(|s| s.as_str()) == Some(SR_CATEGORY)
                                && coding.get("code").and_then(|c| c.as_str()) == Some("laboratory")
                        })
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Build an AtriusIn ServiceRequest for a LOINC-coded laboratory order.
#[must_use]
pub fn build_lab_service_request(
    id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    loinc_code: &str,
    loinc_display: &str,
    note: Option<&str>,
) -> Value {
    let narrative = note
        .filter(|s| !s.trim().is_empty())
        .map(str::trim)
        .unwrap_or(loinc_display);

    json!({
        "resourceType": "ServiceRequest",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_SERVICE_REQUEST] },
        "status": "active",
        "intent": "order",
        "category": [{
            "coding": [{
                "system": SR_CATEGORY,
                "code": "laboratory",
                "display": "Laboratory"
            }]
        }],
        "code": {
            "coding": [{
                "system": LOINC,
                "code": loinc_code,
                "display": loinc_display
            }],
            "text": loinc_display
        },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "requester": { "reference": format!("Practitioner/{practitioner_id}") },
        "authoredOn": now_datetime(),
        "text": section_text_div(narrative)
    })
}

/// Fulfillment Task for a laboratory ServiceRequest (specimen collection / processing).
#[must_use]
pub fn build_lab_fulfillment_task(
    task_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    order_id: &str,
    display: &str,
) -> Value {
    json!({
        "resourceType": "Task",
        "id": task_id,
        "status": "requested",
        "intent": "order",
        "code": { "text": format!("Laboratory: {display}") },
        "description": format!("Collect/process specimen for {display}"),
        "for": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "focus": { "reference": format!("ServiceRequest/{order_id}") },
        "authoredOn": now_datetime(),
        "requester": { "reference": format!("Practitioner/{practitioner_id}") },
        "text": section_text_div(&format!("Lab task: {display}"))
    })
}

/// POST transaction: ServiceRequest + fulfillment Task.
#[must_use]
pub fn lab_order_place_transaction(service_request: Value, task: Value) -> Value {
    let sr_type = service_request
        .get("resourceType")
        .and_then(|v| v.as_str())
        .unwrap_or("ServiceRequest");
    let sr_id = service_request
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let task_id = task
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": [
            {
                "fullUrl": format!("urn:uuid:{sr_id}"),
                "resource": service_request,
                "request": { "method": "POST", "url": sr_type }
            },
            {
                "fullUrl": format!("urn:uuid:{task_id}"),
                "resource": task,
                "request": { "method": "POST", "url": "Task" }
            }
        ]
    })
}

#[must_use]
pub fn is_lab_fulfillment_task(resource: &Value, order_id: Option<&str>) -> bool {
    if resource.get("resourceType").and_then(|v| v.as_str()) != Some("Task") {
        return false;
    }
    let focus = resource
        .get("focus")
        .and_then(|f| f.get("reference"))
        .and_then(|r| r.as_str());
    match (focus, order_id) {
        (Some(reference), Some(order)) => reference == &format!("ServiceRequest/{order}"),
        (Some(reference), None) => reference.starts_with("ServiceRequest/"),
        _ => false,
    }
}

/// Build a final laboratory Observation result linked to an order.
#[must_use]
pub fn build_lab_result_observation(
    observation_id: &str,
    patient_id: &str,
    encounter_id: &str,
    order_id: &str,
    loinc_code: &str,
    loinc_display: &str,
    value: &str,
    unit: Option<&str>,
) -> Value {
    let effective = now_datetime();
    let mut observation = json!({
        "resourceType": "Observation",
        "id": observation_id,
        "meta": { "profile": [ATRIUS_IN_OBSERVATION] },
        "status": "final",
        "category": [{
            "coding": [{
                "system": OBS_CATEGORY,
                "code": "laboratory",
                "display": "Laboratory"
            }]
        }],
        "code": {
            "coding": [{
                "system": LOINC,
                "code": loinc_code,
                "display": loinc_display
            }],
            "text": loinc_display
        },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "effectiveDateTime": effective,
        "issued": effective,
        "basedOn": [{ "reference": format!("ServiceRequest/{order_id}") }],
        "text": section_text_div(&format!("{loinc_display}: {value}"))
    });

    if let Some(unit) = unit.filter(|u| !u.is_empty()) {
        if let Ok(num) = value.parse::<f64>() {
            observation["valueQuantity"] = json!({
                "value": num,
                "unit": unit,
                "system": "http://unitsofmeasure.org",
                "code": unit
            });
        } else {
            observation["valueString"] = json!(value);
        }
    } else {
        observation["valueString"] = json!(value);
    }

    observation
}

/// Build a final laboratory DiagnosticReport for an order + result Observation.
#[must_use]
pub fn build_lab_diagnostic_report(
    report_id: &str,
    patient_id: &str,
    encounter_id: &str,
    order_id: &str,
    observation_id: &str,
    loinc_code: &str,
    loinc_display: &str,
) -> Value {
    let effective = now_datetime();
    json!({
        "resourceType": "DiagnosticReport",
        "id": report_id,
        "meta": { "profile": [ATRIUS_IN_DIAGNOSTIC_REPORT_LAB] },
        "status": "final",
        "category": [{
            "coding": [{
                "system": DIAGNOSTIC_SERVICE,
                "code": "LAB",
                "display": "Laboratory"
            }]
        }],
        "code": {
            "coding": [{
                "system": LOINC,
                "code": loinc_code,
                "display": loinc_display
            }],
            "text": loinc_display
        },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "encounter": { "reference": format!("Encounter/{encounter_id}") },
        "effectiveDateTime": effective,
        "issued": effective,
        "basedOn": [{ "reference": format!("ServiceRequest/{order_id}") }],
        "result": [{ "reference": format!("Observation/{observation_id}") }],
        "text": section_text_div(&format!("Laboratory result: {loinc_display}"))
    })
}

/// Transaction: Observation + DiagnosticReport + completed Task + completed ServiceRequest.
#[must_use]
pub fn lab_result_transaction(
    observation: Value,
    diagnostic_report: Value,
    task: Value,
    service_request: Value,
) -> Value {
    let obs_id = observation
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let dr_id = diagnostic_report
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let sr_id = service_request
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": [
            {
                "fullUrl": format!("urn:uuid:{obs_id}"),
                "resource": observation,
                "request": { "method": "POST", "url": "Observation" }
            },
            {
                "fullUrl": format!("urn:uuid:{dr_id}"),
                "resource": diagnostic_report,
                "request": { "method": "POST", "url": "DiagnosticReport" }
            },
            {
                "resource": task,
                "request": { "method": "PUT", "url": format!("Task/{task_id}") }
            },
            {
                "resource": service_request,
                "request": { "method": "PUT", "url": format!("ServiceRequest/{sr_id}") }
            }
        ]
    })
}

#[must_use]
pub fn is_lab_diagnostic_report(resource: &Value) -> bool {
    if resource.get("resourceType").and_then(|v| v.as_str()) != Some("DiagnosticReport") {
        return false;
    }
    resource
        .get("meta")
        .and_then(|m| m.get("profile"))
        .and_then(|p| p.as_array())
        .and_then(|profiles| profiles.first())
        .and_then(|v| v.as_str())
        == Some(ATRIUS_IN_DIAGNOSTIC_REPORT_LAB)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lab_service_request_has_loinc_and_laboratory_category() {
        let sr = build_lab_service_request(
            "lab-test-1",
            "patient-1",
            "enc-1",
            "dr-patel",
            "58410-2",
            "Complete blood count (hemogram) panel - Blood by Automated count",
            Some("Routine CBC"),
        );
        assert!(is_lab_service_request(&sr));
        assert_eq!(
            sr["code"]["coding"][0]["code"].as_str(),
            Some("58410-2")
        );
        assert_eq!(sr["category"][0]["coding"][0]["code"].as_str(), Some("laboratory"));
    }

    #[test]
    fn narrative_referral_is_not_lab_order() {
        let sr = crate::clinical::entry_builders::narrative_service_request(
            "sr-1",
            "patient-1",
            "enc-1",
            "dr-patel",
            "Cardiology referral",
            "3457005",
            "Patient referral",
        );
        assert!(!is_lab_service_request(&sr));
    }
}
