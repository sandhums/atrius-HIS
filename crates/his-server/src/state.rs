use his_adt::AdtService;
use his_documentation::DocumentationService;
use his_domain::{FhirClient, HisConfig};
use his_registration::RegistrationService;
use his_scheduling::SchedulingService;

#[derive(Clone)]
pub struct AppState {
    pub registration: RegistrationService,
    pub scheduling: SchedulingService,
    pub adt: AdtService,
    pub documentation: DocumentationService,
}

impl AppState {
    pub fn from_config(config: &HisConfig) -> anyhow::Result<Self> {
        let fhir = FhirClient::new(config)?;
        Ok(Self {
            registration: RegistrationService::new(fhir.clone()),
            scheduling: SchedulingService::new(fhir.clone()),
            adt: AdtService::new(fhir.clone()),
            documentation: DocumentationService::new(fhir),
        })
    }
}
