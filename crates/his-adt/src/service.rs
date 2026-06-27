use his_domain::{
    FhirClient, active_bed_id, admit_transaction, appointment_location_id,
    appointment_patient_id, appointment_practitioner_id, build_ambulatory_encounter,
    build_inpatient_encounter, build_inpatient_episode_of_care, discharge_transaction,
    encounter_active_location_id, encounter_appointment_id, encounter_class_code,
    encounter_patient_id, encounter_reason_text, finish_episode_of_care,
    finish_visit_transaction, is_bed_available, operational_status_code,
    patient_display_name, primary_episode_of_care_id, resources_from_search_bundle,
    start_visit_transaction, transfer_transaction,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::debug;

use crate::error::{AdtError, encounter_from_transaction_response, episode_from_transaction_response};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmitPatientRequest {
    pub patient_id: String,
    pub bed_id: String,
    #[serde(default = "default_organization_id")]
    pub organization_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub practitioner_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appointment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admit_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// When true, omit EpisodeOfCare creation on admit (default: create episode).
    #[serde(default)]
    pub skip_episode_of_care: bool,
}

fn default_organization_id() -> String {
    "atrius-demo-hospital".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmitPatientResponse {
    pub encounter_id: String,
    pub bed_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<String>,
    pub encounter: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferPatientRequest {
    pub new_bed_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DischargePatientRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discharge_disposition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartVisitRequest {
    pub appointment_id: String,
    #[serde(default = "default_organization_id")]
    pub organization_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub practitioner_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartVisitResponse {
    pub encounter_id: String,
    pub appointment_id: String,
    pub encounter: Value,
    pub appointment: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishVisitRequest {
    pub encounter_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishVisitResponse {
    pub encounter_id: String,
    pub appointment_id: String,
    pub encounter: Value,
    pub appointment: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedBoardQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ward_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedBoardEntry {
    pub bed_id: String,
    pub bed_name: String,
    pub ward_id: Option<String>,
    pub operational_status: Option<String>,
    pub occupied: bool,
    pub encounter_id: Option<String>,
    pub patient_id: Option<String>,
    pub patient_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedBoardResponse {
    pub count: usize,
    pub beds: Vec<BedBoardEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PractitionerEncountersQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterSummary {
    pub encounter_id: String,
    pub patient_id: Option<String>,
    pub patient_name: Option<String>,
    pub status: String,
    pub class_code: Option<String>,
    pub class_display: Option<String>,
    pub reason: Option<String>,
    pub location_id: Option<String>,
    pub location_name: Option<String>,
    pub bed_id: Option<String>,
    pub ward_id: Option<String>,
    pub period_start: Option<String>,
    pub appointment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PractitionerEncountersResponse {
    pub practitioner_id: String,
    pub count: usize,
    pub encounters: Vec<EncounterSummary>,
}

#[derive(Clone)]
pub struct AdtService {
    fhir: FhirClient,
}

impl AdtService {
    pub fn new(fhir: FhirClient) -> Self {
        Self { fhir }
    }

    pub async fn admit(&self, req: &AdmitPatientRequest) -> Result<AdmitPatientResponse, AdtError> {
        if req.patient_id.trim().is_empty() {
            return Err(AdtError::InvalidRequest("patient_id is required".into()));
        }
        if req.bed_id.trim().is_empty() {
            return Err(AdtError::InvalidRequest("bed_id is required".into()));
        }

        self.fhir
            .read_resource("Patient", &req.patient_id)
            .await
            .map_err(|_| AdtError::PatientNotFound(req.patient_id.clone()))?;

        let bed = self.read_bed(&req.bed_id).await?;
        self.ensure_bed_available(&bed, &req.bed_id).await?;

        let encounter_id = new_encounter_id();
        let episode = if req.skip_episode_of_care {
            None
        } else {
            let episode_id = new_episode_id();
            Some(build_inpatient_episode_of_care(
                &episode_id,
                &req.patient_id,
                &req.organization_id,
            ))
        };

        let episode_ref = episode.as_ref().map(|ep| {
            format!(
                "urn:uuid:{}",
                ep.get("id").and_then(|v| v.as_str()).unwrap_or("episode")
            )
        });

        let encounter = build_inpatient_encounter(
            &encounter_id,
            &req.patient_id,
            &req.bed_id,
            &req.organization_id,
            req.practitioner_id.as_deref(),
            req.appointment_id.as_deref(),
            req.admit_source.as_deref(),
            req.reason.as_deref(),
            episode_ref.as_deref(),
        );

        let bundle = admit_transaction(&encounter, &bed, episode.as_ref());
        debug!(%encounter_id, bed_id = %req.bed_id, "admit transaction");
        let response = self
            .fhir
            .post_transaction(&bundle)
            .await
            .map_err(AdtError::from_fhir)?;

        let created = encounter_from_transaction_response(&response).unwrap_or(encounter);
        let encounter_id = created
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&encounter_id)
            .to_string();

        let created_episode = episode
            .as_ref()
            .and_then(|_| episode_from_transaction_response(&response))
            .or(episode);
        let episode_id = created_episode
            .as_ref()
            .and_then(|ep| ep.get("id"))
            .and_then(|v| v.as_str())
            .map(str::to_string);

        Ok(AdmitPatientResponse {
            encounter_id,
            bed_id: req.bed_id.clone(),
            episode_id,
            encounter: created,
            episode: created_episode,
        })
    }

    pub async fn start_visit(&self, req: &StartVisitRequest) -> Result<StartVisitResponse, AdtError> {
        if req.appointment_id.trim().is_empty() {
            return Err(AdtError::InvalidRequest("appointment_id is required".into()));
        }

        let appointment = self
            .read_appointment(&req.appointment_id)
            .await?;

        let status = appointment
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        if status != "booked" {
            return Err(AdtError::AppointmentNotBookable {
                appointment_id: req.appointment_id.clone(),
                status: status.to_string(),
            });
        }

        if let Some(existing_id) = self
            .active_encounter_for_appointment(&req.appointment_id)
            .await?
        {
            return Err(AdtError::VisitAlreadyStarted {
                appointment_id: req.appointment_id.clone(),
                encounter_id: existing_id,
            });
        }

        let patient_id = appointment_patient_id(&appointment).ok_or_else(|| {
            AdtError::InvalidRequest(
                "appointment has no Patient participant; cannot start visit".into(),
            )
        })?;

        self.fhir
            .read_resource("Patient", &patient_id)
            .await
            .map_err(|_| AdtError::PatientNotFound(patient_id.clone()))?;

        let practitioner_id = req
            .practitioner_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| appointment_practitioner_id(&appointment))
            .ok_or_else(|| {
                AdtError::InvalidRequest(
                    "practitioner_id is required when appointment has no Practitioner participant"
                        .into(),
                )
            })?;

        let location_id = appointment_location_id(&appointment);
        let period_start = appointment
            .get("start")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| his_domain::now_datetime());
        let period_end = appointment
            .get("end")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let encounter_id = new_encounter_id();
        let encounter = build_ambulatory_encounter(
            &encounter_id,
            &patient_id,
            &req.organization_id,
            &practitioner_id,
            &req.appointment_id,
            &period_start,
            period_end.as_deref(),
            location_id.as_deref(),
            req.reason.as_deref(),
        );

        let bundle = start_visit_transaction(&encounter, &appointment);
        debug!(
            %encounter_id,
            appointment_id = %req.appointment_id,
            "start-visit transaction"
        );
        let response = self
            .fhir
            .post_transaction(&bundle)
            .await
            .map_err(AdtError::from_fhir)?;

        let created_encounter =
            encounter_from_transaction_response(&response).unwrap_or(encounter);
        let encounter_id = created_encounter
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&encounter_id)
            .to_string();

        let updated_appointment = self.read_appointment(&req.appointment_id).await?;

        Ok(StartVisitResponse {
            encounter_id,
            appointment_id: req.appointment_id.clone(),
            encounter: created_encounter,
            appointment: updated_appointment,
        })
    }

    pub async fn finish_visit(
        &self,
        req: &FinishVisitRequest,
    ) -> Result<FinishVisitResponse, AdtError> {
        if req.encounter_id.trim().is_empty() {
            return Err(AdtError::InvalidRequest("encounter_id is required".into()));
        }

        let encounter = self.read_encounter(&req.encounter_id).await?;
        let status = encounter
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        if status == "finished" {
            return Err(AdtError::VisitAlreadyFinished {
                encounter_id: req.encounter_id.clone(),
                status: status.to_string(),
            });
        }
        if status != "in-progress" {
            return Err(AdtError::EncounterNotActive(status.to_string()));
        }

        let class_code = encounter
            .get("class")
            .and_then(|c| c.get("code"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        if class_code != "AMB" {
            return Err(AdtError::InvalidRequest(format!(
                "finish-visit applies to ambulatory encounters (class=AMB), got {class_code}"
            )));
        }

        let appointment_id = encounter_appointment_id(&encounter).ok_or_else(|| {
            AdtError::InvalidRequest(
                "encounter has no linked Appointment; cannot finish OPD visit".into(),
            )
        })?;

        let appointment = self.read_appointment(&appointment_id).await?;
        let appt_status = appointment
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        if appt_status != "arrived" && appt_status != "fulfilled" {
            return Err(AdtError::AppointmentNotBookable {
                appointment_id: appointment_id.clone(),
                status: appt_status.to_string(),
            });
        }

        let bundle = finish_visit_transaction(&encounter, &appointment);
        debug!(
            encounter_id = %req.encounter_id,
            appointment_id = %appointment_id,
            "finish-visit transaction"
        );
        self.fhir
            .post_transaction(&bundle)
            .await
            .map_err(AdtError::from_fhir)?;

        let finished_encounter = self.read_encounter(&req.encounter_id).await?;
        let updated_appointment = self.read_appointment(&appointment_id).await?;

        Ok(FinishVisitResponse {
            encounter_id: req.encounter_id.clone(),
            appointment_id,
            encounter: finished_encounter,
            appointment: updated_appointment,
        })
    }

    pub async fn read_encounter(&self, id: &str) -> Result<Value, AdtError> {
        self.fhir
            .read_resource("Encounter", id)
            .await
            .map_err(AdtError::from_fhir)
    }

    pub async fn transfer(
        &self,
        encounter_id: &str,
        req: &TransferPatientRequest,
    ) -> Result<Value, AdtError> {
        if req.new_bed_id.trim().is_empty() {
            return Err(AdtError::InvalidRequest("new_bed_id is required".into()));
        }

        let encounter = self.read_encounter(encounter_id).await?;
        ensure_in_progress(&encounter)?;

        let old_bed_id = active_bed_id(&encounter).ok_or_else(|| {
            AdtError::InvalidRequest("encounter has no active bed location".into())
        })?;

        if old_bed_id == req.new_bed_id {
            return Err(AdtError::InvalidRequest(
                "new_bed_id is the same as current bed".into(),
            ));
        }

        let old_bed = self.read_bed(&old_bed_id).await?;
        let new_bed = self.read_bed(&req.new_bed_id).await?;
        self.ensure_bed_available(&new_bed, &req.new_bed_id).await?;

        let mut encounter = encounter;
        if let Some(reason) = req.reason.as_deref().filter(|s| !s.is_empty()) {
            encounter["reasonCode"] = json!([{ "text": reason }]);
        }

        let bundle = transfer_transaction(&encounter, &old_bed, &new_bed);
        debug!(%encounter_id, new_bed = %req.new_bed_id, "transfer transaction");
        self.fhir
            .post_transaction(&bundle)
            .await
            .map_err(AdtError::from_fhir)?;

        self.read_encounter(encounter_id).await
    }

    pub async fn discharge(
        &self,
        encounter_id: &str,
        req: &DischargePatientRequest,
    ) -> Result<Value, AdtError> {
        let encounter = self.read_encounter(encounter_id).await?;
        ensure_in_progress(&encounter)?;

        let bed_id = active_bed_id(&encounter).ok_or_else(|| {
            AdtError::InvalidRequest("encounter has no active bed to release".into())
        })?;
        let bed = self.read_bed(&bed_id).await?;

        let mut encounter = encounter;
        if let Some(dest) = req.destination_id.as_deref().filter(|s| !s.is_empty()) {
            if let Some(hosp) = encounter.get_mut("hospitalization").and_then(|h| h.as_object_mut())
            {
                hosp.insert(
                    "destination".into(),
                    json!({ "reference": format!("Location/{dest}") }),
                );
            } else {
                encounter["hospitalization"] = json!({
                    "destination": { "reference": format!("Location/{dest}") }
                });
            }
        }

        let finished_episode = if let Some(episode_id) = primary_episode_of_care_id(&encounter) {
            let episode = self
                .fhir
                .read_resource("EpisodeOfCare", &episode_id)
                .await
                .ok();
            episode.as_ref().map(finish_episode_of_care)
        } else {
            None
        };

        let bundle = discharge_transaction(
            &encounter,
            &bed,
            finished_episode.as_ref(),
            req.discharge_disposition.as_deref(),
        );
        debug!(%encounter_id, %bed_id, "discharge transaction");
        self.fhir
            .post_transaction(&bundle)
            .await
            .map_err(AdtError::from_fhir)?;

        self.read_encounter(encounter_id).await
    }

    pub async fn bed_board(&self, query: &BedBoardQuery) -> Result<BedBoardResponse, AdtError> {
        let locations = resources_from_search_bundle(
            &self
                .fhir
                .search_resources("Location", &[("status", "active")])
                .await
                .map_err(AdtError::from_fhir)?,
        )
        .map_err(AdtError::from_fhir)?;

        let ward_filter = query
            .ward_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let beds: Vec<Value> = locations
            .into_iter()
            .filter(|loc| is_bed_location(loc))
            .filter(|loc| {
                ward_filter.as_ref().is_none_or(|ward| bed_in_ward(loc, ward))
            })
            .collect();

        let active_encounters = resources_from_search_bundle(
            &self
                .fhir
                .search_resources("Encounter", &[("status", "in-progress")])
                .await
                .map_err(AdtError::from_fhir)?,
        )
        .map_err(AdtError::from_fhir)?;

        let mut entries = Vec::new();
        for bed in beds {
            let bed_id = bed.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let op_status = operational_status_code(&bed).map(str::to_string);

            let ward_id = bed
                .get("partOf")
                .and_then(|p| p.get("reference"))
                .and_then(|r| r.as_str())
                .and_then(|r| r.strip_prefix("Location/"))
                .map(str::to_string);

            let matching_enc = active_encounters.iter().find(|enc| {
                active_bed_id(enc).as_deref() == Some(bed_id.as_str())
            });

            let (encounter_id, patient_id, patient_name) = if let Some(enc) = matching_enc {
                let patient_ref = enc
                    .get("subject")
                    .and_then(|s| s.get("reference"))
                    .and_then(|r| r.as_str())
                    .and_then(|r| r.strip_prefix("Patient/"))
                    .map(str::to_string);
                (
                    enc.get("id")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    patient_ref.clone(),
                    patient_ref,
                )
            } else {
                (None, None, None)
            };

            entries.push(BedBoardEntry {
                bed_id: bed_id.clone(),
                bed_name: bed
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&bed_id)
                    .to_string(),
                ward_id,
                operational_status: op_status.clone(),
                occupied: op_status.as_deref() == Some("O") || matching_enc.is_some(),
                encounter_id,
                patient_id,
                patient_name,
            });
        }

        entries.sort_by(|a, b| a.bed_id.cmp(&b.bed_id));
        let count = entries.len();
        Ok(BedBoardResponse { count, beds: entries })
    }

    pub async fn list_practitioner_encounters(
        &self,
        practitioner_id: &str,
        query: &PractitionerEncountersQuery,
    ) -> Result<PractitionerEncountersResponse, AdtError> {
        if practitioner_id.trim().is_empty() {
            return Err(AdtError::InvalidRequest(
                "practitioner_id is required".into(),
            ));
        }

        let status = query
            .status
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("in-progress");
        let practitioner_ref = format!("Practitioner/{practitioner_id}");

        let bundle = self
            .fhir
            .search_resources(
                "Encounter",
                &[
                    ("participant", practitioner_ref.as_str()),
                    ("status", status),
                ],
            )
            .await
            .map_err(AdtError::from_fhir)?;

        let mut encounters =
            resources_from_search_bundle(&bundle).map_err(AdtError::from_fhir)?;

        if let Some(class) = query.class.as_deref().filter(|s| !s.is_empty()) {
            encounters.retain(|enc| encounter_class_code(enc).as_deref() == Some(class));
        }

        encounters.sort_by(|a, b| {
            b.get("period")
                .and_then(|p| p.get("start"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(
                    a.get("period")
                        .and_then(|p| p.get("start"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                )
        });

        let mut summaries = Vec::with_capacity(encounters.len());
        for enc in encounters {
            let encounter_id = enc
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let patient_id = encounter_patient_id(&enc);
            let patient_name = if let Some(ref pid) = patient_id {
                self.fhir
                    .read_resource("Patient", pid)
                    .await
                    .ok()
                    .and_then(|patient| patient_display_name(&patient))
            } else {
                None
            };

            let bed_id = active_bed_id(&enc);
            let location_id = encounter_active_location_id(&enc);
            let (location_name, ward_id) = if let Some(ref loc_id) = location_id {
                match self.fhir.read_resource("Location", loc_id).await {
                    Ok(loc) => {
                        let name = loc
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                        let ward = loc
                            .get("partOf")
                            .and_then(|p| p.get("reference"))
                            .and_then(|r| r.as_str())
                            .and_then(|r| r.strip_prefix("Location/"))
                            .map(str::to_string);
                        (name, ward)
                    }
                    Err(_) => (None, None),
                }
            } else {
                (None, None)
            };

            summaries.push(EncounterSummary {
                encounter_id,
                patient_id,
                patient_name,
                status: enc
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                class_code: encounter_class_code(&enc),
                class_display: enc
                    .get("class")
                    .and_then(|c| c.get("display"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                reason: encounter_reason_text(&enc),
                location_id: location_id.clone(),
                location_name,
                bed_id: bed_id.or(location_id),
                ward_id,
                period_start: enc
                    .get("period")
                    .and_then(|p| p.get("start"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                appointment_id: encounter_appointment_id(&enc),
            });
        }

        let count = summaries.len();
        Ok(PractitionerEncountersResponse {
            practitioner_id: practitioner_id.to_string(),
            count,
            encounters: summaries,
        })
    }

    async fn read_appointment(&self, appointment_id: &str) -> Result<Value, AdtError> {
        self.fhir
            .read_resource("Appointment", appointment_id)
            .await
            .map_err(|_| AdtError::AppointmentNotFound(appointment_id.to_string()))
    }

    async fn active_encounter_for_appointment(
        &self,
        appointment_id: &str,
    ) -> Result<Option<String>, AdtError> {
        let bundle = self
            .fhir
            .search_resources(
                "Encounter",
                &[
                    ("appointment", &format!("Appointment/{appointment_id}")),
                    ("status", "in-progress"),
                ],
            )
            .await
            .map_err(AdtError::from_fhir)?;

        let encounters = resources_from_search_bundle(&bundle).map_err(AdtError::from_fhir)?;
        Ok(encounters
            .first()
            .and_then(|enc| enc.get("id"))
            .and_then(|v| v.as_str())
            .map(str::to_string))
    }

    async fn read_bed(&self, bed_id: &str) -> Result<Value, AdtError> {
        self.fhir
            .read_resource("Location", bed_id)
            .await
            .map_err(|_| AdtError::BedNotFound(bed_id.to_string()))
    }

    async fn ensure_bed_available(&self, bed: &Value, bed_id: &str) -> Result<(), AdtError> {
        if !is_bed_available(bed) {
            return Err(AdtError::BedNotAvailable {
                bed_id: bed_id.to_string(),
            });
        }

        let bundle = self
            .fhir
            .search_resources(
                "Encounter",
                &[
                    ("location", &format!("Location/{bed_id}")),
                    ("status", "in-progress"),
                ],
            )
            .await
            .map_err(AdtError::from_fhir)?;

        let active = resources_from_search_bundle(&bundle).map_err(AdtError::from_fhir)?;
        if !active.is_empty() {
            return Err(AdtError::BedNotAvailable {
                bed_id: bed_id.to_string(),
            });
        }

        Ok(())
    }
}

fn ensure_in_progress(encounter: &Value) -> Result<(), AdtError> {
    let status = encounter
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    if status != "in-progress" {
        return Err(AdtError::EncounterNotActive(status.to_string()));
    }
    Ok(())
}

fn is_bed_location(location: &Value) -> bool {
    location
        .get("physicalType")
        .and_then(|pt| pt.get("coding"))
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("code"))
        .and_then(|c| c.as_str())
        == Some("bd")
}

fn bed_in_ward(location: &Value, ward_id: &str) -> bool {
    location
        .get("partOf")
        .and_then(|p| p.get("reference"))
        .and_then(|r| r.as_str())
        .is_some_and(|r| r == format!("Location/{ward_id}"))
}

fn new_encounter_id() -> String {
    format!("enc-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}

fn new_episode_id() -> String {
    format!("ep-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}
