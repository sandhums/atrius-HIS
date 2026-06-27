use his_domain::{
    FhirClient, LAB_CATALOG, build_lab_diagnostic_report, build_lab_fulfillment_task,
    build_lab_result_observation, build_lab_service_request, is_lab_diagnostic_report,
    is_lab_fulfillment_task, is_lab_service_request, lab_fulfillment_task_id,
    lab_order_place_transaction, lab_result_observation_id, lab_result_report_id,
    lab_result_transaction, lab_service_request_id, resolve_lab_display,
    resources_from_search_bundle,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::error::{
    OrderError, encounter_patient_id, ensure_encounter_in_progress, ensure_lab_order_active,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceLabOrderRequest {
    pub encounter_id: String,
    pub practitioner_id: String,
    pub loinc_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabOrderResponse {
    pub order_id: String,
    pub task_id: String,
    pub encounter_id: String,
    pub loinc_code: String,
    pub display: String,
    pub status: String,
    pub service_request: Value,
    pub task: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabOrderSummary {
    pub order_id: String,
    pub loinc_code: Option<String>,
    pub display: Option<String>,
    pub status: String,
    pub authored_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListLabOrdersResponse {
    pub count: usize,
    pub orders: Vec<LabOrderSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeLabOrderResponse {
    pub order_id: String,
    pub status: String,
    pub service_request: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabCatalogResponse {
    pub count: usize,
    pub tests: Vec<LabCatalogEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabCatalogEntryDto {
    pub loinc_code: String,
    pub display: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabTaskSummary {
    pub task_id: String,
    pub order_id: Option<String>,
    pub status: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListLabTasksResponse {
    pub count: usize,
    pub tasks: Vec<LabTaskSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabResultSummary {
    pub report_id: String,
    pub order_id: Option<String>,
    pub loinc_code: Option<String>,
    pub display: Option<String>,
    pub status: String,
    pub issued: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListLabResultsResponse {
    pub count: usize,
    pub results: Vec<LabResultSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostLabResultRequest {
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostLabResultResponse {
    pub order_id: String,
    pub task_id: String,
    pub observation_id: String,
    pub report_id: String,
    pub service_request: Value,
    pub task: Value,
    pub observation: Value,
    pub diagnostic_report: Value,
}

#[derive(Clone)]
pub struct OrderService {
    fhir: FhirClient,
}

impl OrderService {
    pub fn new(fhir: FhirClient) -> Self {
        Self { fhir }
    }

    pub fn lab_catalog(&self) -> LabCatalogResponse {
        let tests = LAB_CATALOG
            .iter()
            .map(|entry| LabCatalogEntryDto {
                loinc_code: entry.loinc_code.to_string(),
                display: entry.display.to_string(),
                title: entry.title.to_string(),
            })
            .collect::<Vec<_>>();
        LabCatalogResponse {
            count: tests.len(),
            tests,
        }
    }

    pub async fn place_lab_order(
        &self,
        req: &PlaceLabOrderRequest,
    ) -> Result<LabOrderResponse, OrderError> {
        if req.encounter_id.trim().is_empty() {
            return Err(OrderError::InvalidRequest("encounter_id is required".into()));
        }
        if req.practitioner_id.trim().is_empty() {
            return Err(OrderError::InvalidRequest("practitioner_id is required".into()));
        }
        if req.loinc_code.trim().is_empty() {
            return Err(OrderError::InvalidRequest("loinc_code is required".into()));
        }

        let encounter = self.read_encounter(&req.encounter_id).await?;
        ensure_encounter_in_progress(&encounter)?;
        let patient_id = encounter_patient_id(&encounter)?;

        let display = req
            .display
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
            .or_else(|| resolve_lab_display(&req.loinc_code).map(str::to_string))
            .ok_or_else(|| OrderError::UnknownLoincCode(req.loinc_code.clone()))?;

        let order_id = lab_service_request_id();
        let task_id = lab_fulfillment_task_id();
        let service_request = build_lab_service_request(
            &order_id,
            &patient_id,
            &req.encounter_id,
            &req.practitioner_id,
            &req.loinc_code,
            &display,
            req.note.as_deref(),
        );
        let task = build_lab_fulfillment_task(
            &task_id,
            &patient_id,
            &req.encounter_id,
            &req.practitioner_id,
            &order_id,
            &display,
        );
        let bundle = lab_order_place_transaction(service_request, task);

        debug!(
            encounter_id = %req.encounter_id,
            loinc_code = %req.loinc_code,
            %order_id,
            %task_id,
            "place lab order"
        );

        let response = self
            .fhir
            .post_transaction(&bundle)
            .await
            .map_err(OrderError::from_fhir)?;

        let persisted_order_id = resource_id_from_transaction_response(&response, "ServiceRequest")
            .unwrap_or(order_id);
        let persisted_task_id =
            resource_id_from_transaction_response(&response, "Task").unwrap_or(task_id);

        let service_request = self.read_lab_order(&persisted_order_id).await?;
        let task = self.read_lab_task(&persisted_task_id).await?;

        Ok(LabOrderResponse {
            order_id: persisted_order_id.clone(),
            task_id: persisted_task_id,
            encounter_id: req.encounter_id.clone(),
            loinc_code: req.loinc_code.clone(),
            display,
            status: service_request
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            service_request,
            task,
        })
    }

    pub async fn list_lab_tasks(
        &self,
        encounter_id: &str,
    ) -> Result<ListLabTasksResponse, OrderError> {
        if encounter_id.trim().is_empty() {
            return Err(OrderError::InvalidRequest("encounter_id is required".into()));
        }

        let bundle = self
            .fhir
            .search_resources(
                "Task",
                &[("encounter", &format!("Encounter/{encounter_id}"))],
            )
            .await
            .map_err(OrderError::from_fhir)?;

        let resources = resources_from_search_bundle(&bundle).map_err(OrderError::from_fhir)?;
        let tasks = resources
            .into_iter()
            .filter(|resource| is_lab_fulfillment_task(resource, None))
            .filter_map(|resource| lab_task_summary_from_resource(&resource))
            .collect::<Vec<_>>();

        Ok(ListLabTasksResponse {
            count: tasks.len(),
            tasks,
        })
    }

    pub async fn list_lab_results(
        &self,
        encounter_id: &str,
    ) -> Result<ListLabResultsResponse, OrderError> {
        if encounter_id.trim().is_empty() {
            return Err(OrderError::InvalidRequest("encounter_id is required".into()));
        }

        let bundle = self
            .fhir
            .search_resources(
                "DiagnosticReport",
                &[("encounter", &format!("Encounter/{encounter_id}"))],
            )
            .await
            .map_err(OrderError::from_fhir)?;

        let resources = resources_from_search_bundle(&bundle).map_err(OrderError::from_fhir)?;
        let results = resources
            .into_iter()
            .filter(|resource| is_lab_diagnostic_report(resource))
            .filter_map(|resource| lab_result_summary_from_resource(&resource))
            .collect::<Vec<_>>();

        Ok(ListLabResultsResponse {
            count: results.len(),
            results,
        })
    }

    pub async fn post_lab_result(
        &self,
        order_id: &str,
        req: &PostLabResultRequest,
    ) -> Result<PostLabResultResponse, OrderError> {
        if req.value.trim().is_empty() {
            return Err(OrderError::InvalidRequest("value is required".into()));
        }

        let mut order = self.read_lab_order(order_id).await?;
        ensure_lab_order_active(&order)?;

        let encounter_id = order
            .get("encounter")
            .and_then(|e| e.get("reference"))
            .and_then(|r| r.as_str())
            .and_then(|r| r.strip_prefix("Encounter/"))
            .ok_or_else(|| OrderError::InvalidRequest("order has no encounter reference".into()))?
            .to_string();
        let patient_id = order
            .get("subject")
            .and_then(|s| s.get("reference"))
            .and_then(|r| r.as_str())
            .and_then(|r| r.strip_prefix("Patient/"))
            .ok_or_else(|| OrderError::InvalidRequest("order has no patient reference".into()))?
            .to_string();

        let (loinc_code, display) = lab_code_from_service_request(&order);
        let loinc_code = loinc_code.unwrap_or_else(|| "unknown".into());
        let display = display.unwrap_or_else(|| "Laboratory test".into());

        let task = self.find_task_for_order(order_id).await?;
        let task_id = task
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OrderError::LabTaskNotFound(order_id.to_string()))?
            .to_string();

        let observation_id = lab_result_observation_id();
        let report_id = lab_result_report_id();
        let observation = build_lab_result_observation(
            &observation_id,
            &patient_id,
            &encounter_id,
            order_id,
            &loinc_code,
            &display,
            req.value.trim(),
            req.unit.as_deref(),
        );
        let diagnostic_report = build_lab_diagnostic_report(
            &report_id,
            &patient_id,
            &encounter_id,
            order_id,
            &observation_id,
            &loinc_code,
            &display,
        );

        let mut completed_task = task;
        completed_task["status"] = Value::String("completed".into());
        order["status"] = Value::String("completed".into());

        let bundle = lab_result_transaction(observation, diagnostic_report, completed_task, order);
        debug!(%order_id, %task_id, "post lab result (LIS stub)");
        let response = self
            .fhir
            .post_transaction(&bundle)
            .await
            .map_err(OrderError::from_fhir)?;

        let persisted_obs_id =
            resource_id_from_transaction_response(&response, "Observation").unwrap_or(observation_id);
        let persisted_dr_id = resource_id_from_transaction_response(&response, "DiagnosticReport")
            .unwrap_or(report_id);

        let service_request = self.read_lab_order(order_id).await?;
        let task = self.read_lab_task(&task_id).await?;
        let observation = self
            .fhir
            .read_resource("Observation", &persisted_obs_id)
            .await
            .map_err(OrderError::from_fhir)?;
        let diagnostic_report = self
            .fhir
            .read_resource("DiagnosticReport", &persisted_dr_id)
            .await
            .map_err(OrderError::from_fhir)?;

        Ok(PostLabResultResponse {
            order_id: order_id.to_string(),
            task_id,
            observation_id: persisted_obs_id,
            report_id: persisted_dr_id,
            service_request,
            task,
            observation,
            diagnostic_report,
        })
    }

    pub async fn read_lab_task(&self, task_id: &str) -> Result<Value, OrderError> {
        let resource = self
            .fhir
            .read_resource("Task", task_id)
            .await
            .map_err(|err| {
                if err.to_string().contains("404") {
                    OrderError::LabTaskNotFound(task_id.to_string())
                } else {
                    OrderError::from_fhir(err)
                }
            })?;
        if !is_lab_fulfillment_task(&resource, None) {
            return Err(OrderError::LabTaskNotFound(task_id.to_string()));
        }
        Ok(resource)
    }

    async fn find_task_for_order(&self, order_id: &str) -> Result<Value, OrderError> {
        let bundle = self
            .fhir
            .search_resources(
                "Task",
                &[("focus", &format!("ServiceRequest/{order_id}"))],
            )
            .await
            .map_err(OrderError::from_fhir)?;
        let resources = resources_from_search_bundle(&bundle).map_err(OrderError::from_fhir)?;
        resources
            .into_iter()
            .find(|resource| is_lab_fulfillment_task(resource, Some(order_id)))
            .ok_or_else(|| OrderError::LabTaskNotFound(order_id.to_string()))
    }

    pub async fn list_lab_orders(
        &self,
        encounter_id: &str,
    ) -> Result<ListLabOrdersResponse, OrderError> {
        if encounter_id.trim().is_empty() {
            return Err(OrderError::InvalidRequest("encounter_id is required".into()));
        }

        let bundle = self
            .fhir
            .search_resources(
                "ServiceRequest",
                &[
                    ("encounter", &format!("Encounter/{encounter_id}")),
                    ("category", "laboratory"),
                ],
            )
            .await
            .map_err(OrderError::from_fhir)?;

        let resources = resources_from_search_bundle(&bundle).map_err(OrderError::from_fhir)?;
        let orders = resources
            .into_iter()
            .filter(|resource| is_lab_service_request(resource))
            .filter_map(|resource| lab_order_summary_from_resource(&resource))
            .collect::<Vec<_>>();

        Ok(ListLabOrdersResponse {
            count: orders.len(),
            orders,
        })
    }

    pub async fn read_lab_order(&self, order_id: &str) -> Result<Value, OrderError> {
        let resource = self
            .fhir
            .read_resource("ServiceRequest", order_id)
            .await
            .map_err(|err| {
                if err.to_string().contains("404") {
                    OrderError::LabOrderNotFound(order_id.to_string())
                } else {
                    OrderError::from_fhir(err)
                }
            })?;

        if !is_lab_service_request(&resource) {
            return Err(OrderError::LabOrderNotFound(order_id.to_string()));
        }

        Ok(resource)
    }

    pub async fn revoke_lab_order(
        &self,
        order_id: &str,
    ) -> Result<RevokeLabOrderResponse, OrderError> {
        let mut order = self.read_lab_order(order_id).await?;
        ensure_lab_order_active(&order)?;

        order["status"] = Value::String("revoked".into());
        debug!(%order_id, "revoke lab order");

        let updated = self
            .fhir
            .update_resource("ServiceRequest", order_id, &order)
            .await
            .map_err(OrderError::from_fhir)?;

        Ok(RevokeLabOrderResponse {
            order_id: order_id.to_string(),
            status: updated
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("revoked")
                .to_string(),
            service_request: updated,
        })
    }

    async fn read_encounter(&self, encounter_id: &str) -> Result<Value, OrderError> {
        self.fhir
            .read_resource("Encounter", encounter_id)
            .await
            .map_err(|err| {
                if err.to_string().contains("404") {
                    OrderError::EncounterNotFound(encounter_id.to_string())
                } else {
                    OrderError::from_fhir(err)
                }
            })
    }
}

fn lab_order_summary_from_resource(resource: &Value) -> Option<LabOrderSummary> {
    let order_id = resource.get("id")?.as_str()?.to_string();
    let status = resource
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let authored_on = resource
        .get("authoredOn")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let (loinc_code, display) = lab_code_from_service_request(resource);

    Some(LabOrderSummary {
        order_id,
        loinc_code,
        display,
        status,
        authored_on,
    })
}

fn lab_code_from_service_request(resource: &Value) -> (Option<String>, Option<String>) {
    resource
        .get("code")
        .map(|code| {
            let loinc = code
                .get("coding")
                .and_then(|c| c.as_array())
                .and_then(|codings| {
                    codings.iter().find_map(|coding| {
                        if coding.get("system").and_then(|s| s.as_str())
                            == Some("http://loinc.org")
                        {
                            coding.get("code").and_then(|c| c.as_str()).map(str::to_string)
                        } else {
                            None
                        }
                    })
                });
            let text = code
                .get("text")
                .and_then(|t| t.as_str())
                .map(str::to_string);
            (loinc, text)
        })
        .unwrap_or((None, None))
}

fn lab_task_summary_from_resource(resource: &Value) -> Option<LabTaskSummary> {
    let task_id = resource.get("id")?.as_str()?.to_string();
    let status = resource
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let order_id = resource
        .get("focus")
        .and_then(|f| f.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("ServiceRequest/"))
        .map(str::to_string);
    let description = resource
        .get("description")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Some(LabTaskSummary {
        task_id,
        order_id,
        status,
        description,
    })
}

fn lab_result_summary_from_resource(resource: &Value) -> Option<LabResultSummary> {
    let report_id = resource.get("id")?.as_str()?.to_string();
    let status = resource
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let issued = resource
        .get("issued")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let order_id = resource
        .get("basedOn")
        .and_then(|b| b.as_array())
        .and_then(|arr| arr.first())
        .and_then(|ref_val| ref_val.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("ServiceRequest/"))
        .map(str::to_string);
    let (loinc_code, display) = lab_code_from_service_request(resource);
    Some(LabResultSummary {
        report_id,
        order_id,
        loinc_code,
        display,
        status,
        issued,
    })
}

fn resource_id_from_transaction_response(response: &Value, resource_type: &str) -> Option<String> {
    response
        .get("entry")
        .and_then(|e| e.as_array())
        .and_then(|entries| {
            entries.iter().find_map(|entry| {
                let resource = entry.get("resource")?;
                if resource.get("resourceType")?.as_str()? == resource_type {
                    resource.get("id")?.as_str().map(str::to_string)
                } else {
                    None
                }
            })
        })
}
