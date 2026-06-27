//! NDHM DocumentBundle assembly for ABDM clinical record exchange.

use serde_json::{Value, json};

/// Build a document Bundle wrapping a Composition and referenced entry resources.
///
/// `referenced_resources` should include Patient, Encounter, Practitioner, and all
/// section entry resources referenced by the Composition (deduplicated by id).
#[must_use]
pub fn export_document_bundle(composition: &Value, referenced_resources: &[Value]) -> Value {
    let mut entries = vec![json!({
        "fullUrl": format!(
            "urn:uuid:{}",
            composition
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("composition")
        ),
        "resource": composition
    })];

    for resource in referenced_resources {
        let resource_type = resource
            .get("resourceType")
            .and_then(|v| v.as_str())
            .unwrap_or("Resource");
        let id = resource
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        entries.push(json!({
            "fullUrl": format!("urn:uuid:{id}"),
            "resource": resource
        }));
        let _ = (resource_type,);
    }

    json!({
        "resourceType": "Bundle",
        "type": "document",
        "timestamp": crate::adt::now_datetime(),
        "entry": entries
    })
}

/// Collect resource references from Composition.section.entry for follow-up reads.
pub fn section_entry_references(composition: &Value) -> Vec<String> {
    composition
        .get("section")
        .and_then(|s| s.as_array())
        .map(|sections| {
            sections
                .iter()
                .flat_map(|section| {
                    section
                        .get("entry")
                        .and_then(|entries| entries.as_array())
                        .into_iter()
                        .flatten()
                })
                .filter_map(|entry| entry.get("reference"))
                .filter_map(|r| r.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_bundle_starts_with_composition() {
        let composition = json!({
            "resourceType": "Composition",
            "id": "comp-1",
            "section": [{
                "entry": [{ "reference": "Condition/cc-1" }]
            }]
        });
        let condition = json!({
            "resourceType": "Condition",
            "id": "cc-1"
        });
        let bundle = export_document_bundle(&composition, &[condition]);
        assert_eq!(bundle["type"], "document");
        assert_eq!(bundle["entry"].as_array().unwrap().len(), 2);
        assert_eq!(
            bundle["entry"][0]["resource"]["resourceType"],
            "Composition"
        );
    }

    #[test]
    fn extracts_section_entry_references() {
        let composition = json!({
            "section": [
                { "entry": [{ "reference": "Condition/a" }] },
                { "entry": [{ "reference": "Observation/b" }] }
            ]
        });
        assert_eq!(
            section_entry_references(&composition),
            vec!["Condition/a".to_string(), "Observation/b".to_string()]
        );
    }
}
