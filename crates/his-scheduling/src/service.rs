use his_domain::{
    ExpandError, FhirClient, appointment_location_id, appointment_patient_id, appointment_slot_ids,
    book_appointment_transaction, build_appointment, cancel_appointment_transaction,
    expand_schedule_slots, expand_slots_transaction, parse_date_or_datetime, patient_display_name,
    reschedule_appointment_transaction, resources_from_search_bundle, slot_timing_from_resource,
};
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::error::{SchedulingError, resource_from_transaction_response};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandSlotsQuery {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandSlotsResponse {
    pub schedule_id: String,
    pub from: String,
    pub to: String,
    pub slots_created: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSlotsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub practitioner_id: Option<String>,
    /// Inclusive lower bound (FHIR date/time), e.g. `2026-06-20` or full instant.
    pub start: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotSummary {
    pub slot_id: String,
    pub schedule_id: Option<String>,
    pub status: String,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSlotsResponse {
    pub count: usize,
    pub slots: Vec<SlotSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookAppointmentRequest {
    pub patient_id: String,
    pub slot_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub practitioner_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookAppointmentResponse {
    pub appointment_id: String,
    pub slot_id: String,
    pub appointment: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAppointmentRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RescheduleAppointmentRequest {
    pub new_slot_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PractitionerAppointmentsQuery {
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppointmentSummary {
    pub appointment_id: String,
    pub patient_id: Option<String>,
    pub patient_name: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub status: String,
    pub description: Option<String>,
    pub location_id: Option<String>,
    pub encounter_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PractitionerAppointmentsResponse {
    pub practitioner_id: String,
    pub date: String,
    pub count: usize,
    pub appointments: Vec<AppointmentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingDoctorSummary {
    pub practitioner_id: String,
    pub name: String,
    pub schedule_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBookingDoctorsResponse {
    pub count: usize,
    pub doctors: Vec<BookingDoctorSummary>,
}

#[derive(Clone)]
pub struct SchedulingService {
    fhir: FhirClient,
}

impl SchedulingService {
    pub fn new(fhir: FhirClient) -> Self {
        Self { fhir }
    }

    pub async fn expand_schedule_slots(
        &self,
        schedule_id: &str,
        query: &ExpandSlotsQuery,
    ) -> Result<ExpandSlotsResponse, SchedulingError> {
        if query.from.trim().is_empty() || query.to.trim().is_empty() {
            return Err(SchedulingError::InvalidRequest(
                "from and to are required".into(),
            ));
        }

        let schedule = self
            .fhir
            .read_resource("Schedule", schedule_id)
            .await
            .map_err(|_| SchedulingError::ScheduleNotFound(schedule_id.to_string()))?;

        let from = parse_date_or_datetime(&query.from, false).map_err(map_expand_error)?;
        let to = parse_date_or_datetime(&query.to, true).map_err(map_expand_error)?;

        let slots = expand_schedule_slots(&schedule, from, to).map_err(map_expand_error)?;
        let slots_created = slots.len();

        if !slots.is_empty() {
            let bundle = expand_slots_transaction(&slots);
            debug!(schedule_id, slots_created, "expanding schedule slots transaction");
            self.fhir
                .post_transaction(&bundle)
                .await
                .map_err(SchedulingError::from_fhir)?;
        }

        Ok(ExpandSlotsResponse {
            schedule_id: schedule_id.to_string(),
            from: query.from.clone(),
            to: query.to.clone(),
            slots_created,
        })
    }

    pub async fn read_schedule(&self, id: &str) -> Result<Value, SchedulingError> {
        self.fhir
            .read_resource("Schedule", id)
            .await
            .map_err(|_| SchedulingError::ScheduleNotFound(id.to_string()))
    }

    pub async fn list_booking_doctors(&self) -> Result<ListBookingDoctorsResponse, SchedulingError> {
        let bundle = self
            .fhir
            .search_resources("Schedule", &[("active", "true")])
            .await
            .map_err(SchedulingError::from_fhir)?;

        let schedules =
            resources_from_search_bundle(&bundle).map_err(SchedulingError::from_fhir)?;

        let mut by_practitioner: BTreeMap<String, (String, Option<String>)> = BTreeMap::new();
        for schedule in schedules {
            let Some(schedule_id) = schedule.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(practitioner_id) = schedule_actor_ref(&schedule, "Practitioner/") else {
                continue;
            };
            let location_id = schedule_actor_ref(&schedule, "Location/");
            by_practitioner
                .entry(practitioner_id)
                .or_insert_with(|| (schedule_id.to_string(), location_id));
        }

        let mut doctors = Vec::with_capacity(by_practitioner.len());
        for (practitioner_id, (schedule_id, location_id)) in by_practitioner {
            let name = self
                .fhir
                .read_resource("Practitioner", &practitioner_id)
                .await
                .ok()
                .and_then(|p| patient_display_name(&p))
                .unwrap_or_else(|| practitioner_id.clone());

            doctors.push(BookingDoctorSummary {
                practitioner_id,
                name,
                schedule_id,
                location_id,
            });
        }

        doctors.sort_by(|a, b| a.name.cmp(&b.name));
        let count = doctors.len();
        Ok(ListBookingDoctorsResponse { count, doctors })
    }

    pub async fn find_available_slots(
        &self,
        query: &FindSlotsQuery,
    ) -> Result<FindSlotsResponse, SchedulingError> {
        if query.start.trim().is_empty() {
            return Err(SchedulingError::InvalidRequest(
                "start is required".into(),
            ));
        }

        let mut params: Vec<(&str, String)> = vec![
            ("status", "free".into()),
            ("start", format!("ge{}", query.start)),
        ];

        if let Some(end) = query.end.as_deref().filter(|s| !s.is_empty()) {
            params.push(("start", format!("le{end}")));
        }

        if let Some(schedule_id) = query.schedule_id.as_deref().filter(|s| !s.is_empty()) {
            params.push(("schedule", format!("Schedule/{schedule_id}")));
        }

        let param_refs: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();

        let mut slots = resources_from_search_bundle(
            &self
                .fhir
                .search_resources("Slot", &param_refs)
                .await
                .map_err(SchedulingError::from_fhir)?,
        )
        .map_err(SchedulingError::from_fhir)?;

        if let Some(practitioner_id) = query.practitioner_id.as_deref().filter(|s| !s.is_empty()) {
            let schedule_ids = self.schedule_ids_for_practitioner(practitioner_id).await?;
            if schedule_ids.is_empty() {
                slots.clear();
            } else {
                slots.retain(|slot| {
                    slot.get("schedule")
                        .and_then(|s| s.get("reference"))
                        .and_then(|r| r.as_str())
                        .and_then(|r| r.strip_prefix("Schedule/"))
                        .is_some_and(|id| schedule_ids.iter().any(|sid| sid == id))
                });
            }
        }

        let summaries: Vec<SlotSummary> = slots.iter().map(slot_to_summary).collect();
        Ok(FindSlotsResponse {
            count: summaries.len(),
            slots: summaries,
        })
    }

    pub async fn book_appointment(
        &self,
        req: &BookAppointmentRequest,
    ) -> Result<BookAppointmentResponse, SchedulingError> {
        validate_book_request(req)?;

        self.fhir
            .read_resource("Patient", &req.patient_id)
            .await
            .map_err(|_| SchedulingError::PatientNotFound(req.patient_id.clone()))?;

        let slot = self
            .fhir
            .read_resource("Slot", &req.slot_id)
            .await
            .map_err(SchedulingError::from_fhir)?;

        ensure_slot_free(&slot, &req.slot_id)?;

        let appointment_id = new_appointment_id();
        let timing = slot_timing_from_resource(&slot);
        let appointment = build_appointment(
            &appointment_id,
            &req.patient_id,
            &timing,
            req.practitioner_id.as_deref(),
            req.location_id.as_deref(),
            req.description.as_deref(),
        );

        let bundle = book_appointment_transaction(&appointment, &slot);
        debug!(%appointment_id, slot_id = %req.slot_id, "booking appointment transaction");
        let response = self
            .fhir
            .post_transaction(&bundle)
            .await
            .map_err(SchedulingError::from_fhir)?;

        let created = resource_from_transaction_response(&response)
            .unwrap_or(appointment);

        let appointment_id = created
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&appointment_id)
            .to_string();

        Ok(BookAppointmentResponse {
            appointment_id: appointment_id.clone(),
            slot_id: req.slot_id.clone(),
            appointment: created,
        })
    }

    pub async fn read_appointment(&self, id: &str) -> Result<Value, SchedulingError> {
        self.fhir
            .read_resource("Appointment", id)
            .await
            .map_err(SchedulingError::from_fhir)
    }

    pub async fn cancel_appointment(
        &self,
        id: &str,
        req: &CancelAppointmentRequest,
    ) -> Result<Value, SchedulingError> {
        let mut appointment = self.read_appointment(id).await?;
        ensure_cancellable(&appointment)?;

        if let Some(reason) = req.reason.as_deref().filter(|s| !s.is_empty()) {
            appointment["cancelationReason"] = serde_json::json!({
                "text": reason
            });
        }

        let slot_ids = appointment_slot_ids(&appointment);
        let mut slots = Vec::new();
        for slot_id in slot_ids {
            let slot = self
                .fhir
                .read_resource("Slot", &slot_id)
                .await
                .map_err(SchedulingError::from_fhir)?;
            slots.push(slot);
        }

        let bundle = cancel_appointment_transaction(&appointment, &slots);
        debug!(appointment_id = %id, "cancelling appointment transaction");
        self.fhir
            .post_transaction(&bundle)
            .await
            .map_err(SchedulingError::from_fhir)?;

        self.read_appointment(id).await
    }

    pub async fn reschedule_appointment(
        &self,
        id: &str,
        req: &RescheduleAppointmentRequest,
    ) -> Result<Value, SchedulingError> {
        if req.new_slot_id.trim().is_empty() {
            return Err(SchedulingError::InvalidRequest(
                "new_slot_id is required".into(),
            ));
        }

        let appointment = self.read_appointment(id).await?;
        ensure_cancellable(&appointment)?;

        let new_slot = self
            .fhir
            .read_resource("Slot", &req.new_slot_id)
            .await
            .map_err(SchedulingError::from_fhir)?;
        ensure_slot_free(&new_slot, &req.new_slot_id)?;

        let old_slot_ids = appointment_slot_ids(&appointment);
        let mut old_slots = Vec::new();
        for slot_id in old_slot_ids {
            let slot = self
                .fhir
                .read_resource("Slot", &slot_id)
                .await
                .map_err(SchedulingError::from_fhir)?;
            old_slots.push(slot);
        }

        let bundle = reschedule_appointment_transaction(&appointment, &old_slots, &new_slot);
        debug!(appointment_id = %id, new_slot = %req.new_slot_id, "reschedule transaction");
        self.fhir
            .post_transaction(&bundle)
            .await
            .map_err(SchedulingError::from_fhir)?;

        self.read_appointment(id).await
    }

    pub async fn list_practitioner_appointments(
        &self,
        practitioner_id: &str,
        query: &PractitionerAppointmentsQuery,
    ) -> Result<PractitionerAppointmentsResponse, SchedulingError> {
        if practitioner_id.trim().is_empty() {
            return Err(SchedulingError::InvalidRequest(
                "practitioner_id is required".into(),
            ));
        }
        if query.date.trim().is_empty() {
            return Err(SchedulingError::InvalidRequest("date is required".into()));
        }

        let date_start = format!("{}T00:00:00", query.date.trim());
        let date_end = format!("{}T23:59:59", query.date.trim());
        let practitioner_ref = format!("Practitioner/{practitioner_id}");

        let bundle = self
            .fhir
            .search_resources(
                "Appointment",
                &[
                    ("practitioner", practitioner_ref.as_str()),
                    ("date", &format!("ge{date_start}")),
                    ("date", &format!("le{date_end}")),
                ],
            )
            .await
            .map_err(SchedulingError::from_fhir)?;

        let mut appointments =
            resources_from_search_bundle(&bundle).map_err(SchedulingError::from_fhir)?;

        appointments.retain(|appt| {
            appt.get("status")
                .and_then(|v| v.as_str())
                .is_some_and(|status| status != "cancelled" && status != "entered-in-error")
        });

        appointments.sort_by(|a, b| {
            a.get("start")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("start").and_then(|v| v.as_str()).unwrap_or(""))
        });

        let mut summaries = Vec::with_capacity(appointments.len());
        for appt in appointments {
            let appointment_id = appt
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let patient_id = appointment_patient_id(&appt);
            let patient_name = if let Some(ref pid) = patient_id {
                self.fhir
                    .read_resource("Patient", pid)
                    .await
                    .ok()
                    .and_then(|patient| patient_display_name(&patient))
            } else {
                None
            };
            let encounter_id = self
                .active_encounter_for_appointment(&appointment_id)
                .await
                .ok()
                .flatten();

            summaries.push(AppointmentSummary {
                appointment_id,
                patient_id,
                patient_name,
                start: appt
                    .get("start")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                end: appt.get("end").and_then(|v| v.as_str()).map(str::to_string),
                status: appt
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                description: appt
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                location_id: appointment_location_id(&appt),
                encounter_id,
            });
        }

        let count = summaries.len();
        Ok(PractitionerAppointmentsResponse {
            practitioner_id: practitioner_id.to_string(),
            date: query.date.clone(),
            count,
            appointments: summaries,
        })
    }

    async fn active_encounter_for_appointment(
        &self,
        appointment_id: &str,
    ) -> Result<Option<String>, SchedulingError> {
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
            .map_err(SchedulingError::from_fhir)?;

        let encounters =
            resources_from_search_bundle(&bundle).map_err(SchedulingError::from_fhir)?;
        Ok(encounters
            .first()
            .and_then(|enc| enc.get("id"))
            .and_then(|v| v.as_str())
            .map(str::to_string))
    }

    async fn schedule_ids_for_practitioner(
        &self,
        practitioner_id: &str,
    ) -> Result<Vec<String>, SchedulingError> {
        let bundle = self
            .fhir
            .search_resources(
                "Schedule",
                &[("actor", &format!("Practitioner/{practitioner_id}"))],
            )
            .await
            .map_err(SchedulingError::from_fhir)?;

        Ok(resources_from_search_bundle(&bundle)
            .map_err(SchedulingError::from_fhir)?
            .iter()
            .filter_map(|s| s.get("id").and_then(|v| v.as_str()).map(str::to_string))
            .collect())
    }
}

fn schedule_actor_ref(schedule: &Value, prefix: &str) -> Option<String> {
    schedule
        .get("actor")
        .and_then(|v| v.as_array())
        .and_then(|actors| {
            actors.iter().find_map(|actor| {
                actor
                    .get("reference")
                    .and_then(|v| v.as_str())
                    .and_then(|r| r.strip_prefix(prefix))
                    .map(str::to_string)
            })
        })
}

fn map_expand_error(err: ExpandError) -> SchedulingError {
    SchedulingError::Expand(err.to_string())
}

fn validate_book_request(req: &BookAppointmentRequest) -> Result<(), SchedulingError> {
    if req.patient_id.trim().is_empty() {
        return Err(SchedulingError::InvalidRequest(
            "patient_id is required".into(),
        ));
    }
    if req.slot_id.trim().is_empty() {
        return Err(SchedulingError::InvalidRequest(
            "slot_id is required".into(),
        ));
    }
    Ok(())
}

fn ensure_slot_free(slot: &Value, slot_id: &str) -> Result<(), SchedulingError> {
    let status = slot
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    if status != "free" {
        return Err(SchedulingError::SlotNotAvailable {
            slot_id: slot_id.to_string(),
            status: status.to_string(),
        });
    }
    Ok(())
}

fn ensure_cancellable(appointment: &Value) -> Result<(), SchedulingError> {
    let status = appointment
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    if matches!(status, "cancelled" | "entered-in-error" | "fulfilled") {
        return Err(SchedulingError::AppointmentNotActive(status.to_string()));
    }
    Ok(())
}

fn slot_to_summary(slot: &Value) -> SlotSummary {
    SlotSummary {
        slot_id: slot
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        schedule_id: slot
            .get("schedule")
            .and_then(|s| s.get("reference"))
            .and_then(|r| r.as_str())
            .and_then(|r| r.strip_prefix("Schedule/"))
            .map(str::to_string),
        status: slot
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        start: slot
            .get("start")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        end: slot
            .get("end")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    }
}

fn new_appointment_id() -> String {
    format!("appt-{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_patient_on_book() {
        let req = BookAppointmentRequest {
            patient_id: "  ".into(),
            slot_id: "s1".into(),
            practitioner_id: None,
            location_id: None,
            description: None,
        };
        assert!(validate_book_request(&req).is_err());
    }
}
