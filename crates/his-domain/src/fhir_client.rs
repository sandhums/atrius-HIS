use anyhow::{Context, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::Client;
use serde_json::Value;
use tracing::debug;

use crate::config::HisConfig;

/// Thin HTTP client for Clinical HFS REST interactions.
#[derive(Clone)]
pub struct FhirClient {
    http: Client,
    base_url: String,
    tenant: String,
    bearer_token: Option<String>,
}

impl FhirClient {
    pub fn new(config: &HisConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            http: Client::builder()
                .user_agent("atrius-his/0.1")
                .build()
                .context("build reqwest client")?,
            base_url: trim_trailing_slash(&config.fhir_base_url),
            tenant: config.default_tenant.clone(),
            bearer_token: config.fhir_bearer_token.clone(),
        })
    }

    pub fn with_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = tenant.into();
        self
    }

    pub fn with_bearer(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }

    pub fn with_bearer_opt(mut self, token: Option<String>) -> Self {
        self.bearer_token = token;
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn metadata(&self) -> Result<Value> {
        self.get_json("/metadata").await
    }

    pub async fn get_json(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        debug!(%url, tenant = %self.tenant, "FHIR GET");

        let response = self
            .http
            .get(&url)
            .headers(self.request_headers()?)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        let status = response.status();
        let body = response.text().await.context("read response body")?;

        if status.is_success() {
            serde_json::from_str(&body).with_context(|| format!("parse JSON from GET {url}"))
        } else {
            anyhow::bail!("GET {url} failed with {status}: {body}");
        }
    }

    pub async fn post_transaction(&self, bundle: &Value) -> Result<Value> {
        let url = format!("{}/", self.base_url);
        self.post_json(&url, bundle).await
    }

    pub async fn create_resource(&self, resource_type: &str, resource: &Value) -> Result<Value> {
        let url = format!("{}/{}", self.base_url, resource_type);
        self.post_json(&url, resource).await
    }

    pub async fn read_resource(&self, resource_type: &str, id: &str) -> Result<Value> {
        self.get_json(&format!("/{resource_type}/{id}")).await
    }

    pub async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        resource: &Value,
    ) -> Result<Value> {
        let url = format!("{}/{}/{}", self.base_url, resource_type, id);
        self.put_json(&url, resource).await
    }

    pub async fn search_resources(
        &self,
        resource_type: &str,
        query: &[(&str, &str)],
    ) -> Result<Value> {
        let qs = query
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding(k), urlencoding(v)))
            .collect::<Vec<_>>()
            .join("&");
        let path = if qs.is_empty() {
            format!("/{resource_type}")
        } else {
            format!("/{resource_type}?{qs}")
        };
        self.get_json(&path).await
    }

    async fn post_json(&self, url: &str, body: &Value) -> Result<Value> {
        self.send_json(reqwest::Method::POST, url, body).await
    }

    async fn put_json(&self, url: &str, body: &Value) -> Result<Value> {
        self.send_json(reqwest::Method::PUT, url, body).await
    }

    async fn send_json(&self, method: reqwest::Method, url: &str, body: &Value) -> Result<Value> {
        debug!(%url, tenant = %self.tenant, ?method, "FHIR write");

        let mut headers = self.request_headers()?;
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/fhir+json"),
        );
        headers.insert("Accept", HeaderValue::from_static("application/fhir+json"));

        let response = self
            .http
            .request(method.clone(), url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .with_context(|| format!("{method} {url}"))?;

        let status = response.status();
        let text = response.text().await.context("read response body")?;

        if status.is_success() {
            if text.trim().is_empty() {
                return Ok(Value::Null);
            }
            serde_json::from_str(&text).with_context(|| format!("parse JSON from {method} {url}"))
        } else {
            anyhow::bail!("{method} {url} failed with {status}: {text}");
        }
    }

    fn request_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Tenant-ID",
            HeaderValue::from_str(&self.tenant).context("invalid tenant header")?,
        );

        if let Some(token) = &self.bearer_token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}"))
                    .context("invalid bearer token header")?,
            );
        }

        Ok(headers)
    }
}

fn trim_trailing_slash(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

fn urlencoding(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}
