use serde_json::{Value, json};

use crate::narrative::generate_patient_narrative;
use crate::profiles::{ATRIUS_IN_PATIENT, ATRIUS_MRN_SYSTEM, PATIENT_BIRTH_PLACE_EXTENSION};

/// Build a minimal Atrius-in-Patient resource for registration.
#[must_use]
pub fn build_patient(
    id: &str,
    mrn: &str,
    family: &str,
    given: &[&str],
    gender: &str,
    birth_date: Option<&str>,
    telecom: Option<&[Telecom]>,
    address: Option<&[Address]>,
    birth_place: Option<&BirthPlace>,
) -> Value {
    let mut patient = json!({
        "resourceType": "Patient",
        "id": id,
        "meta": {
            "profile": [ATRIUS_IN_PATIENT]
        },
        "text": {
            "status": "generated",
            "div": generate_patient_narrative(
                family,
                given,
                gender,
                birth_date,
                Some(mrn),
                birth_place,
                address,
            )
        },
        "identifier": [mrn_identifier(mrn)],
        "name": [{
            "use": "official",
            "family": family,
            "given": given,
            "text": display_name(family, given)
        }],
        "gender": gender,
        "deceasedBoolean": false
    });

    if let Some(bd) = birth_date {
        patient["birthDate"] = json!(bd);
    }

    if let Some(items) = telecom {
        patient["telecom"] = json!(items
            .iter()
            .map(|t| json!({ "system": t.system, "value": t.value, "use": t.use_ }))
            .collect::<Vec<_>>());
    }

    if let Some(items) = address {
        patient["address"] = json!(items
            .iter()
            .map(|a| {
                json!({
                    "use": a.use_,
                    "line": a.line,
                    "city": a.city,
                    "state": a.state,
                    "postalCode": a.postal_code,
                    "country": a.country
                })
            })
            .collect::<Vec<_>>());
    }

    if let Some(bp) = birth_place {
        patient["extension"] = json!([birth_place_extension(bp)]);
    }

    patient
}

fn birth_place_extension(place: &BirthPlace) -> Value {
    json!({
        "url": PATIENT_BIRTH_PLACE_EXTENSION,
        "valueAddress": {
            "city": place.city,
            "state": place.state,
            "country": place.country
        }
    })
}

#[must_use]
pub fn mrn_identifier(mrn: &str) -> Value {
    json!({
        "use": "usual",
        "type": {
            "coding": [{
                "system": "http://terminology.hl7.org/CodeSystem/v2-0203",
                "code": "MR",
                "display": "Medical record number"
            }]
        },
        "system": ATRIUS_MRN_SYSTEM,
        "value": mrn
    })
}

#[derive(Debug, Clone)]
pub struct Telecom {
    pub system: String,
    pub value: String,
    pub use_: String,
}

#[derive(Debug, Clone)]
pub struct Address {
    pub use_: String,
    pub line: Vec<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BirthPlace {
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
}

fn display_name(family: &str, given: &[&str]) -> String {
    if given.is_empty() {
        family.to_string()
    } else {
        format!("{} {}", given.join(" "), family)
    }
}

/// Human-readable name from a FHIR Patient resource.
#[must_use]
pub fn patient_display_name(patient: &Value) -> Option<String> {
    let name = patient.get("name")?.as_array()?.first()?;
    if let Some(text) = name.get("text").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    let family = name.get("family").and_then(|v| v.as_str()).unwrap_or("");
    let given: Vec<&str> = name
        .get("given")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|g| g.as_str()).collect())
        .unwrap_or_default();
    let full = display_name(family, &given);
    if full.is_empty() {
        None
    } else {
        Some(full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patient_has_required_profile_fields() {
        let p = build_patient(
            "p1",
            "MRN-TEST-001",
            "Sharma",
            &["Priya"],
            "female",
            Some("1965-06-15"),
            None,
            None,
            None,
        );
        assert_eq!(p["resourceType"], "Patient");
        assert_eq!(p["meta"]["profile"][0], ATRIUS_IN_PATIENT);
        assert_eq!(p["identifier"][0]["system"], ATRIUS_MRN_SYSTEM);
        assert_eq!(p["gender"], "female");
        assert!(p["name"][0]["family"].is_string());
        assert_eq!(p["text"]["status"], "generated");
        assert!(p["text"]["div"].as_str().unwrap_or("").contains("Priya Sharma"));
    }

    #[test]
    fn patient_includes_birth_place_extension() {
        let bp = BirthPlace {
            city: Some("Mysuru".into()),
            state: Some("KA".into()),
            country: Some("IN".into()),
        };
        let p = build_patient(
            "p1",
            "MRN-TEST-001",
            "Sharma",
            &["Priya"],
            "female",
            Some("1965-06-15"),
            None,
            None,
            Some(&bp),
        );
        let ext = &p["extension"][0];
        assert_eq!(ext["url"], PATIENT_BIRTH_PLACE_EXTENSION);
        assert_eq!(ext["valueAddress"]["city"], "Mysuru");
        assert_eq!(ext["valueAddress"]["country"], "IN");
    }
}
