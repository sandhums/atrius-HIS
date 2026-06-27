use std::sync::Arc;

use his_adt::AdtService;
use his_documentation::DocumentationService;
use his_domain::{FhirClient, HisConfig};
use his_foundation::FoundationService;
use his_orders::OrderService;
use his_registration::RegistrationService;
use his_scheduling::SchedulingService;

use crate::auth::AuthState;
use crate::request_auth::RequestAuth;

/// Per-request domain services bound to the caller bearer token.
pub struct RequestServices {
    pub registration: RegistrationService,
    pub scheduling: SchedulingService,
    pub adt: AdtService,
    pub documentation: DocumentationService,
    pub orders: OrderService,
    pub foundation: FoundationService,
}

impl RequestServices {
    fn from_fhir(fhir: FhirClient, tenant: String) -> Self {
        let scheduling = SchedulingService::new(fhir.clone());
        Self {
            registration: RegistrationService::new(fhir.clone()),
            scheduling: scheduling.clone(),
            adt: AdtService::new(fhir.clone()),
            documentation: DocumentationService::new(fhir.clone()),
            orders: OrderService::new(fhir.clone()),
            foundation: FoundationService::new(fhir, scheduling, tenant),
        }
    }
}

pub struct AppState {
    pub config: HisConfig,
    fhir_template: FhirClient,
    auth: Option<Arc<AuthState>>,
}

impl AppState {
    pub fn from_config(config: &HisConfig) -> anyhow::Result<Self> {
        let fhir_template = FhirClient::new(config)?;
        Ok(Self {
            config: config.clone(),
            fhir_template,
            auth: None,
        })
    }

    pub fn with_auth(mut self, auth: Arc<AuthState>) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn auth_enabled(&self) -> bool {
        self.auth.as_ref().is_some_and(|a| a.config.enabled)
    }

    pub fn auth_state(&self) -> Option<Arc<AuthState>> {
        self.auth.clone()
    }

    pub fn services(&self, auth: &RequestAuth) -> RequestServices {
        let mut fhir = self.fhir_template.clone().with_tenant(auth.tenant_id.clone());
        if !auth.bearer_token.is_empty() {
            fhir = fhir.with_bearer(auth.bearer_token.clone());
        }
        RequestServices::from_fhir(fhir, auth.tenant_id.clone())
    }
}
