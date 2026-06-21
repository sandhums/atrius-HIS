use his_domain::{
    Address, BirthPlace, FhirClient, Telecom, build_patient, resources_from_search_bundle,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::error::RegistrationError;
use crate::mrn::generate_mrn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterPatientRequest {
    pub family_name: String,
    #[serde(default)]
    pub given_names: Vec<String>,
    pub gender: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub telecom: Vec<TelecomInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub address: Vec<AddressInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth_place: Option<BirthPlaceInput>,
    /// When false (default), registration fails if potential duplicates are found.
    #[serde(default)]
    pub allow_duplicates: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelecomInput {
    pub system: String,
    pub value: String,
    #[serde(default = "default_telecom_use")]
    pub use_: String,
}

fn default_telecom_use() -> String {
    "mobile".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInput {
    #[serde(default = "default_address_use")]
    pub use_: String,
    pub line: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

fn default_address_use() -> String {
    "home".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirthPlaceInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterPatientResponse {
    pub patient_id: String,
    pub mrn: String,
    pub patient: Value,
    pub duplicates: Vec<DuplicateMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateMatch {
    pub patient_id: String,
    pub mrn: Option<String>,
    pub name: Option<String>,
    pub birth_date: Option<String>,
    pub match_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateSummary {
    pub count: usize,
    pub matches: Vec<DuplicateMatch>,
}

#[derive(Clone)]
pub struct RegistrationService {
    fhir: FhirClient,
}

impl RegistrationService {
    pub fn new(fhir: FhirClient) -> Self {
        Self { fhir }
    }

    pub async fn register(&self, req: RegisterPatientRequest) -> Result<RegisterPatientResponse, RegistrationError> {
        validate_request(&req)?;

        let duplicates = self.find_duplicates(&req).await?;
        if !req.allow_duplicates && !duplicates.is_empty() {
            return Err(RegistrationError::Duplicate { matches: duplicates });
        }

        let patient_id = new_patient_id();
        let mrn = generate_mrn();
        let patient = build_patient_resource(&patient_id, &mrn, &req);

        debug!(%patient_id, %mrn, "creating Patient in Clinical HFS");
        let created = self
            .fhir
            .create_resource("Patient", &patient)
            .await
            .map_err(RegistrationError::Fhir)?;

        let patient_id = created
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&patient_id)
            .to_string();

        Ok(RegisterPatientResponse {
            patient_id,
            mrn,
            patient: created,
            duplicates,
        })
    }

    pub async fn read_patient(&self, id: &str) -> Result<Value, RegistrationError> {
        self.fhir
            .read_resource("Patient", id)
            .await
            .map_err(RegistrationError::Fhir)
    }

    pub async fn find_duplicates(
        &self,
        req: &RegisterPatientRequest,
    ) -> Result<Vec<DuplicateMatch>, RegistrationError> {
        let mut matches = Vec::new();

        if let Some(bd) = req.birth_date.as_deref() {
            let given = req.given_names.first().map(String::as_str).unwrap_or("");
            let bundle = self
                .fhir
                .search_resources(
                    "Patient",
                    &[
                        ("family", req.family_name.as_str()),
                        ("given", given),
                        ("birthdate", bd),
                    ],
                )
                .await
                .map_err(RegistrationError::Fhir)?;

            for resource in resources_from_search_bundle(&bundle).map_err(RegistrationError::Fhir)? {
                matches.push(patient_to_duplicate(&resource, "name+birthdate"));
            }
        }

        Ok(dedupe_matches(matches))
    }

    pub async fn check_duplicates(
        &self,
        req: &RegisterPatientRequest,
    ) -> Result<DuplicateSummary, RegistrationError> {
        validate_request(req)?;
        let matches = self.find_duplicates(req).await?;
        Ok(DuplicateSummary {
            count: matches.len(),
            matches,
        })
    }
}

fn validate_request(req: &RegisterPatientRequest) -> Result<(), RegistrationError> {
    if req.family_name.trim().is_empty() {
        return Err(RegistrationError::InvalidRequest(
            "family_name is required".into(),
        ));
    }
    if req.gender.trim().is_empty() {
        return Err(RegistrationError::InvalidRequest(
            "gender is required".into(),
        ));
    }
    if !matches!(req.gender.as_str(), "male" | "female" | "other" | "unknown") {
        return Err(RegistrationError::InvalidRequest(format!(
            "gender must be male, female, other, or unknown (got {})",
            req.gender
        )));
    }
    Ok(())
}

fn build_patient_resource(id: &str, mrn: &str, req: &RegisterPatientRequest) -> Value {
    let given: Vec<&str> = req.given_names.iter().map(String::as_str).collect();
    let telecom: Vec<Telecom> = req
        .telecom
        .iter()
        .map(|t| Telecom {
            system: t.system.clone(),
            value: t.value.clone(),
            use_: t.use_.clone(),
        })
        .collect();
    let address: Vec<Address> = req
        .address
        .iter()
        .map(|a| Address {
            use_: a.use_.clone(),
            line: a.line.clone(),
            city: a.city.clone(),
            state: a.state.clone(),
            postal_code: a.postal_code.clone(),
            country: a.country.clone(),
        })
        .collect();

    let birth_place = req.birth_place.as_ref().map(|bp| BirthPlace {
        city: bp.city.clone(),
        state: bp.state.clone(),
        country: bp.country.clone(),
    });

    build_patient(
        id,
        mrn,
        &req.family_name,
        &given,
        &req.gender,
        req.birth_date.as_deref(),
        if telecom.is_empty() {
            None
        } else {
            Some(&telecom)
        },
        if address.is_empty() {
            None
        } else {
            Some(&address)
        },
        birth_place.as_ref(),
    )
}

fn new_patient_id() -> String {
    format!("pat-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}

fn patient_to_duplicate(resource: &Value, reason: &str) -> DuplicateMatch {
    DuplicateMatch {
        patient_id: resource
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        mrn: extract_mrn(resource),
        name: resource
            .get("name")
            .and_then(|n| n.get(0))
            .and_then(|n| n.get("text"))
            .and_then(|t| t.as_str())
            .map(str::to_string),
        birth_date: resource
            .get("birthDate")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        match_reason: reason.to_string(),
    }
}

fn extract_mrn(patient: &Value) -> Option<String> {
    patient
        .get("identifier")
        .and_then(|v| v.as_array())
        .and_then(|ids| {
            ids.iter().find_map(|id| {
                let system = id.get("system").and_then(|s| s.as_str()).unwrap_or("");
                if system == his_domain::ATRIUS_MRN_SYSTEM {
                    id.get("value").and_then(|v| v.as_str()).map(str::to_string)
                } else {
                    None
                }
            })
        })
}

fn dedupe_matches(matches: Vec<DuplicateMatch>) -> Vec<DuplicateMatch> {
    let mut out = Vec::new();
    for m in matches {
        if !out.iter().any(|existing: &DuplicateMatch| {
            existing.patient_id == m.patient_id
        }) {
            out.push(m);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_family_name() {
        let req = RegisterPatientRequest {
            family_name: "  ".into(),
            given_names: vec!["A".into()],
            gender: "female".into(),
            birth_date: Some("1990-01-01".into()),
            telecom: vec![],
            address: vec![],
            birth_place: None,
            allow_duplicates: false,
        };
        assert!(validate_request(&req).is_err());
    }

    #[test]
    fn builds_profile_compliant_patient() {
        let req = RegisterPatientRequest {
            family_name: "Sharma".into(),
            given_names: vec!["Priya".into()],
            gender: "female".into(),
            birth_date: Some("1965-06-15".into()),
            telecom: vec![],
            address: vec![],
            birth_place: None,
            allow_duplicates: false,
        };
        let p = build_patient_resource("p1", "MRN-ABC", &req);
        assert_eq!(
            p["meta"]["profile"][0],
            his_domain::ATRIUS_IN_PATIENT
        );
        assert_eq!(p["identifier"][0]["value"], "MRN-ABC");
    }
}
