use his_domain::{resources_from_search_bundle, FhirClient};
use his_scheduling::{BookingDoctorSummary, SchedulingService};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::FoundationError;

#[derive(Clone)]
pub struct FoundationService {
    fhir: FhirClient,
    scheduling: SchedulingService,
    tenant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSummary {
    pub id: String,
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedSummary {
    pub id: String,
    pub name: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardSummary {
    pub id: String,
    pub name: String,
    pub campus_id: Option<String>,
    pub beds: Vec<BedSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampusSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthcareServiceSummary {
    pub id: String,
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundationConfigResponse {
    pub tenant_id: String,
    pub organization: OrganizationSummary,
    pub campus: Option<CampusSummary>,
    pub wards: Vec<WardSummary>,
    pub healthcare_services: Vec<HealthcareServiceSummary>,
    pub opd_doctors: Vec<BookingDoctorSummary>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrganizationRequest {
    pub name: String,
}

impl FoundationService {
    pub fn new(fhir: FhirClient, scheduling: SchedulingService, tenant_id: String) -> Self {
        Self {
            fhir,
            scheduling,
            tenant_id,
        }
    }

    pub async fn get_config(&self) -> Result<FoundationConfigResponse, FoundationError> {
        let organization = self.primary_organization().await?;
        let locations = self.list_locations().await?;
        let campus = locations
            .iter()
            .find(|loc| location_physical_type(loc) != Some("wa") && location_physical_type(loc) != Some("bd"))
            .map(location_to_campus);
        let wards = build_ward_tree(&locations);
        let healthcare_services = self.list_healthcare_services().await?;
        let doctors = self
            .scheduling
            .list_booking_doctors()
            .await
            .map_err(|e| FoundationError::Fhir(anyhow::anyhow!(e.to_string())))?;

        Ok(FoundationConfigResponse {
            tenant_id: self.tenant_id.clone(),
            organization,
            campus,
            wards,
            healthcare_services,
            opd_doctors: doctors.doctors,
        })
    }

    pub async fn update_organization_name(
        &self,
        id: &str,
        req: &UpdateOrganizationRequest,
    ) -> Result<OrganizationSummary, FoundationError> {
        let name = req.name.trim();
        if name.is_empty() {
            return Err(FoundationError::InvalidRequest(
                "organization name is required".into(),
            ));
        }

        let mut org = self
            .fhir
            .read_resource("Organization", id)
            .await
            .map_err(|_| FoundationError::OrganizationNotFound)?;

        org["name"] = json!(name);
        let updated = self
            .fhir
            .update_resource("Organization", id, &org)
            .await
            .map_err(FoundationError::Fhir)?;

        organization_summary(&updated).ok_or(FoundationError::OrganizationNotFound)
    }

    async fn primary_organization(&self) -> Result<OrganizationSummary, FoundationError> {
        let bundle = self
            .fhir
            .search_resources("Organization", &[("active", "true")])
            .await
            .map_err(FoundationError::Fhir)?;
        let orgs = resources_from_search_bundle(&bundle).map_err(FoundationError::Fhir)?;
        orgs.into_iter()
            .find_map(|org| organization_summary(&org))
            .ok_or(FoundationError::OrganizationNotFound)
    }

    async fn list_locations(&self) -> Result<Vec<Value>, FoundationError> {
        let bundle = self
            .fhir
            .search_resources("Location", &[("status", "active")])
            .await
            .map_err(FoundationError::Fhir)?;
        resources_from_search_bundle(&bundle).map_err(FoundationError::Fhir)
    }

    async fn list_healthcare_services(&self) -> Result<Vec<HealthcareServiceSummary>, FoundationError> {
        let bundle = self
            .fhir
            .search_resources("HealthcareService", &[])
            .await
            .map_err(FoundationError::Fhir)?;
        let services = resources_from_search_bundle(&bundle).map_err(FoundationError::Fhir)?;
        let mut summaries: Vec<HealthcareServiceSummary> = services
            .iter()
            .filter_map(healthcare_service_summary)
            .collect();
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(summaries)
    }
}

fn organization_summary(org: &Value) -> Option<OrganizationSummary> {
    Some(OrganizationSummary {
        id: org.get("id")?.as_str()?.to_string(),
        name: org.get("name")?.as_str()?.to_string(),
        active: org.get("active").and_then(|v| v.as_bool()).unwrap_or(true),
    })
}

fn healthcare_service_summary(svc: &Value) -> Option<HealthcareServiceSummary> {
    Some(HealthcareServiceSummary {
        id: svc.get("id")?.as_str()?.to_string(),
        name: svc.get("name")?.as_str()?.to_string(),
        active: svc.get("active").and_then(|v| v.as_bool()).unwrap_or(true),
    })
}

fn location_to_campus(loc: &Value) -> CampusSummary {
    CampusSummary {
        id: loc
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        name: loc
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Campus")
            .to_string(),
    }
}

fn location_physical_type(loc: &Value) -> Option<&str> {
    loc.get("physicalType")
        .and_then(|pt| pt.get("coding"))
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("code"))
        .and_then(|v| v.as_str())
}

fn location_part_of_id(loc: &Value) -> Option<String> {
    loc.get("partOf")
        .and_then(|p| p.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("Location/"))
        .map(str::to_string)
}

fn location_name(loc: &Value) -> String {
    loc.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Location")
        .to_string()
}

fn bed_status(loc: &Value) -> Option<String> {
    loc.get("operationalStatus")
        .and_then(|s| s.get("code"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn build_ward_tree(locations: &[Value]) -> Vec<WardSummary> {
    let mut wards: Vec<WardSummary> = locations
        .iter()
        .filter(|loc| location_physical_type(loc) == Some("wa"))
        .map(|ward| {
            let ward_id = ward.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let beds = locations
                .iter()
                .filter(|loc| {
                    location_physical_type(loc) == Some("bd")
                        && location_part_of_id(loc).as_deref() == Some(ward_id)
                })
                .map(|bed| BedSummary {
                    id: bed
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    name: location_name(bed),
                    status: bed_status(bed),
                })
                .collect();
            WardSummary {
                id: ward_id.to_string(),
                name: location_name(ward),
                campus_id: location_part_of_id(ward),
                beds,
            }
        })
        .collect();
    wards.sort_by(|a, b| a.name.cmp(&b.name));
    wards
}
