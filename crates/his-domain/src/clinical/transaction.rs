//! FHIR transaction bundle assembly for clinical documents.

use serde_json::{Value, json};

/// POST transaction: entry resources + Composition.
#[must_use]
pub fn create_transaction(composition_id: &str, composition: Value, entry_resources: Vec<Value>) -> Value {
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

/// PUT transaction: replace entry resources and update Composition.
#[must_use]
pub fn update_transaction(composition_id: &str, composition: Value, entry_resources: Vec<Value>) -> Value {
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
        "resource": composition,
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
