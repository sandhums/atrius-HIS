use anyhow::{Context, Result};
use serde_json::Value;

/// Extract resource bodies from a FHIR search Bundle.
pub fn resources_from_search_bundle(bundle: &Value) -> Result<Vec<Value>> {
    let resource_type = bundle
        .get("resourceType")
        .and_then(|v| v.as_str())
        .context("search response missing resourceType")?;
    if resource_type != "Bundle" {
        anyhow::bail!("expected Bundle search response, got {resource_type}");
    }

    let Some(entries) = bundle.get("entry").and_then(|e| e.as_array()) else {
        return Ok(Vec::new());
    };

    Ok(entries
        .iter()
        .filter_map(|entry| entry.get("resource").cloned())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_resources_from_bundle() {
        let bundle = json!({
            "resourceType": "Bundle",
            "type": "searchset",
            "entry": [
                { "resource": { "resourceType": "Patient", "id": "a" } },
                { "resource": { "resourceType": "Patient", "id": "b" } }
            ]
        });
        let resources = resources_from_search_bundle(&bundle).unwrap();
        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0]["id"], "a");
    }
}
