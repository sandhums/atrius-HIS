use std::env;

use anyhow::{Context, Result};

/// Runtime configuration for HIS domain services.
#[derive(Clone, Debug)]
pub struct HisConfig {
    /// Clinical HFS base URL (e.g. `http://127.0.0.1:8082`).
    pub fhir_base_url: String,
    /// HTS base URL (e.g. `http://127.0.0.1:9091`).
    pub terminology_url: String,
    /// Default tenant when auth is disabled (`X-Tenant-ID`).
    pub default_tenant: String,
    /// Optional bearer token for authenticated HFS (SMART backend services).
    pub fhir_bearer_token: Option<String>,
}

impl HisConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            fhir_base_url: env::var("HIS_FHIR_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8082".to_string()),
            terminology_url: env::var("HIS_TERMINOLOGY_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:9091".to_string()),
            default_tenant: env::var("HIS_DEFAULT_TENANT")
                .unwrap_or_else(|_| "atrius-hospital".to_string()),
            fhir_bearer_token: env::var("HIS_FHIR_BEARER_TOKEN").ok(),
        })
    }

    pub fn validate(&self) -> Result<()> {
        url_must_be_http(&self.fhir_base_url).context("HIS_FHIR_BASE_URL")?;
        url_must_be_http(&self.terminology_url).context("HIS_TERMINOLOGY_URL")?;
        Ok(())
    }
}

fn url_must_be_http(url: &str) -> Result<()> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(())
    } else {
        anyhow::bail!("expected http(s) URL, got {url}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_urls_are_valid() {
        let cfg = HisConfig {
            fhir_base_url: "http://127.0.0.1:8082".into(),
            terminology_url: "http://127.0.0.1:9091".into(),
            default_tenant: "default".into(),
            fhir_bearer_token: None,
        };
        cfg.validate().unwrap();
    }
}
