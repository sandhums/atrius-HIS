use anyhow::{Context, Result};
use reqwest::Client;
use serde::Serialize;

use crate::config::HisConfig;

/// Aggregated readiness of the hardened platform stack.
#[derive(Debug, Clone, Serialize)]
pub struct PlatformHealth {
    pub fhir: ComponentHealth,
    pub terminology: ComponentHealth,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    pub url: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

pub struct PlatformProbe {
    http: Client,
    config: HisConfig,
}

impl PlatformProbe {
    pub fn new(config: HisConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            http: Client::builder()
                .user_agent("atrius-his/0.1")
                .build()
                .context("build reqwest client")?,
            config,
        })
    }

    pub async fn check(&self) -> PlatformHealth {
        PlatformHealth {
            fhir: self.check_fhir().await,
            terminology: self.check_terminology().await,
        }
    }

    async fn check_fhir(&self) -> ComponentHealth {
        let url = format!("{}/metadata", self.config.fhir_base_url.trim_end_matches('/'));
        match self.http.get(&url).send().await {
            Ok(response) if response.status().is_success() => ComponentHealth {
                url: self.config.fhir_base_url.clone(),
                ok: true,
                detail: Some(format!("HTTP {}", response.status())),
            },
            Ok(response) => ComponentHealth {
                url: self.config.fhir_base_url.clone(),
                ok: false,
                detail: Some(format!("HTTP {}", response.status())),
            },
            Err(err) => ComponentHealth {
                url: self.config.fhir_base_url.clone(),
                ok: false,
                detail: Some(err.to_string()),
            },
        }
    }

    async fn check_terminology(&self) -> ComponentHealth {
        let url = format!(
            "{}/health",
            self.config.terminology_url.trim_end_matches('/')
        );
        match self.http.get(&url).send().await {
            Ok(response) if response.status().is_success() => ComponentHealth {
                url: self.config.terminology_url.clone(),
                ok: true,
                detail: Some(format!("HTTP {}", response.status())),
            },
            Ok(response) => ComponentHealth {
                url: self.config.terminology_url.clone(),
                ok: false,
                detail: Some(format!("HTTP {}", response.status())),
            },
            Err(err) => ComponentHealth {
                url: self.config.terminology_url.clone(),
                ok: false,
                detail: Some(err.to_string()),
            },
        }
    }
}

impl PlatformHealth {
    pub fn ready(&self) -> bool {
        self.fhir.ok && self.terminology.ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_requires_both_components() {
        let health = PlatformHealth {
            fhir: ComponentHealth {
                url: "http://fhir".into(),
                ok: true,
                detail: None,
            },
            terminology: ComponentHealth {
                url: "http://hts".into(),
                ok: false,
                detail: Some("down".into()),
            },
        };
        assert!(!health.ready());
    }
}
