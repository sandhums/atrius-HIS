//! ADT (Admit / Transfer / Discharge) FHIR resource builders and transaction bundles.

use chrono::{FixedOffset, Utc};
use serde_json::{Value, json};

use crate::narrative::generate_encounter_narrative;
use crate::profiles::{ATRIUS_IN_ENCOUNTER, ATRIUS_IN_EPISODE_OF_CARE, ATRIUS_IN_LOCATION_BED};

const V2_BED_STATUS: &str = "http://terminology.hl7.org/CodeSystem/v2-0116";
const LOC_PHYSICAL: &str = "http://terminology.hl7.org/CodeSystem/location-physical-type";
const V3_ACT_CODE: &str = "http://terminology.hl7.org/CodeSystem/v3-ActCode";
const V3_PARTICIPATION_TYPE: &str = "http://terminology.hl7.org/CodeSystem/v3-ParticipationType";
const ADMIT_SOURCE: &str = "http://terminology.hl7.org/CodeSystem/admit-source";
const EPISODE_OF_CARE_TYPE: &str = "http://terminology.hl7.org/CodeSystem/episodeofcare-type";
const SNOMED: &str = "http://snomed.info/sct";

/// Current timestamp as FHIR `dateTime` in IST (+05:30).
#[must_use]
pub fn now_datetime() -> String {
    let ist = FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap_or(FixedOffset::east_opt(0).unwrap());
    Utc::now().with_timezone(&ist).format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

/// Build an inpatient encounter at admit time.
#[must_use]
pub fn build_inpatient_encounter(
    id: &str,
    patient_id: &str,
    bed_id: &str,
    organization_id: &str,
    practitioner_id: Option<&str>,
    appointment_id: Option<&str>,
    admit_source: Option<&str>,
    reason: Option<&str>,
    episode_of_care_ref: Option<&str>,
) -> Value {
    let start = now_datetime();
    let mut encounter = build_encounter_core(
        id,
        patient_id,
        organization_id,
        "IMP",
        "inpatient encounter",
        "Hospital admission",
        Some(json!([{
            "coding": [{
                "system": SNOMED,
                "code": "32485007",
                "display": "Hospital admission"
            }]
        }])),
        "in-progress",
        &start,
        None,
        practitioner_id,
        appointment_id,
        reason,
    );

    if let Some(episode_ref) = episode_of_care_ref.filter(|s| !s.is_empty()) {
        encounter["episodeOfCare"] = json!([{ "reference": episode_ref }]);
    }

    encounter["location"] = json!([{
        "location": { "reference": format!("Location/{bed_id}") },
        "status": "active",
        "period": { "start": start }
    }]);

    encounter["hospitalization"] = json!({
        "admitSource": {
            "coding": [{
                "system": ADMIT_SOURCE,
                "code": admit_source.unwrap_or("other"),
                "display": "Other"
            }]
        }
    });

    encounter
}

/// Build an active inpatient EpisodeOfCare at admit time.
#[must_use]
pub fn build_inpatient_episode_of_care(
    id: &str,
    patient_id: &str,
    organization_id: &str,
) -> Value {
    let start = now_datetime();
    json!({
        "resourceType": "EpisodeOfCare",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_EPISODE_OF_CARE] },
        "status": "active",
        "type": [{
            "coding": [{
                "system": EPISODE_OF_CARE_TYPE,
                "code": "inp",
                "display": "Inpatient"
            }]
        }],
        "patient": { "reference": format!("Patient/{patient_id}") },
        "managingOrganization": { "reference": format!("Organization/{organization_id}") },
        "period": { "start": start }
    })
}

/// Mark an EpisodeOfCare finished at discharge time.
#[must_use]
pub fn finish_episode_of_care(episode: &Value) -> Value {
    let mut updated = episode.clone();
    let end = now_datetime();
    updated["status"] = json!("finished");
    if let Some(period) = updated.get_mut("period").and_then(|p| p.as_object_mut()) {
        period.insert("end".into(), json!(end));
    } else {
        updated["period"] = json!({ "end": end });
    }
    updated
}

/// EpisodeOfCare id from the first `Encounter.episodeOfCare` reference, if any.
pub fn primary_episode_of_care_id(encounter: &Value) -> Option<String> {
    encounter
        .get("episodeOfCare")
        .and_then(|v| v.as_array())
        .and_then(|refs| refs.first())
        .and_then(|r| r.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(parse_episode_of_care_reference)
}

fn parse_episode_of_care_reference(reference: &str) -> Option<String> {
    reference
        .strip_prefix("EpisodeOfCare/")
        .map(str::to_string)
        .or_else(|| {
            reference
                .strip_prefix("urn:uuid:")
                .map(str::to_string)
        })
}

/// Build an ambulatory (OPD) encounter when starting a visit from a booked Appointment.
#[must_use]
pub fn build_ambulatory_encounter(
    id: &str,
    patient_id: &str,
    organization_id: &str,
    practitioner_id: &str,
    appointment_id: &str,
    period_start: &str,
    period_end: Option<&str>,
    location_id: Option<&str>,
    reason: Option<&str>,
) -> Value {
    let mut encounter = build_encounter_core(
        id,
        patient_id,
        organization_id,
        "AMB",
        "ambulatory",
        "General consultation",
        Some(json!([{ "text": "General consultation" }])),
        "in-progress",
        period_start,
        period_end,
        Some(practitioner_id),
        Some(appointment_id),
        reason,
    );

    if let Some(lid) = location_id.filter(|s| !s.is_empty()) {
        encounter["location"] = json!([{
            "location": { "reference": format!("Location/{lid}") },
            "status": "active",
            "period": { "start": period_start }
        }]);
    }

    encounter
}

fn build_encounter_core(
    id: &str,
    patient_id: &str,
    organization_id: &str,
    class_code: &str,
    class_display: &str,
    default_reason_display: &str,
    encounter_type: Option<Value>,
    status: &str,
    period_start: &str,
    period_end: Option<&str>,
    practitioner_id: Option<&str>,
    appointment_id: Option<&str>,
    reason: Option<&str>,
) -> Value {
    let mut period = json!({ "start": period_start });
    if let Some(end) = period_end.filter(|s| !s.is_empty()) {
        period["end"] = json!(end);
    }

    let reason_for_narrative = reason.filter(|s| !s.is_empty());

    let mut encounter = json!({
        "resourceType": "Encounter",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_ENCOUNTER] },
        "text": {
            "status": "generated",
            "div": generate_encounter_narrative(
                class_display,
                status,
                patient_id,
                period_start,
                reason_for_narrative,
            )
        },
        "status": status,
        "class": {
            "system": V3_ACT_CODE,
            "code": class_code,
            "display": class_display
        },
        "subject": { "reference": format!("Patient/{patient_id}") },
        "period": period,
        "serviceProvider": { "reference": format!("Organization/{organization_id}") },
    });

    if let Some(type_value) = encounter_type {
        encounter["type"] = type_value;
    }

    if let Some(reason_text) = reason_for_narrative {
        encounter["reasonCode"] = json!([{ "text": reason_text }]);
    } else {
        encounter["reasonCode"] = json!([{
            "coding": [{
                "system": SNOMED,
                "code": "185347001",
                "display": default_reason_display
            }]
        }]);
    }

    if let Some(pid) = practitioner_id.filter(|s| !s.is_empty()) {
        encounter["participant"] = json!([attender_participant(pid)]);
    }

    if let Some(appt) = appointment_id.filter(|s| !s.is_empty()) {
        encounter["appointment"] = json!([{ "reference": format!("Appointment/{appt}") }]);
    }

    encounter
}

fn attender_participant(practitioner_id: &str) -> Value {
    json!({
        "type": [{
            "coding": [{
                "system": V3_PARTICIPATION_TYPE,
                "code": "ATND",
                "display": "attender"
            }]
        }],
        "individual": { "reference": format!("Practitioner/{practitioner_id}") }
    })
}

/// Mark a bed Location occupied or vacant via HL7 v2-0116 operationalStatus.
#[must_use]
pub fn bed_with_occupancy(location: &Value, occupied: bool) -> Value {
    let mut updated = location.clone();
    updated["meta"] = json!({ "profile": [ATRIUS_IN_LOCATION_BED] });
    updated["status"] = json!("active");
    updated["mode"] = json!("instance");
    if updated.get("physicalType").is_none() {
        updated["physicalType"] = json!({
            "coding": [{
                "system": LOC_PHYSICAL,
                "code": "bd",
                "display": "Bed"
            }]
        });
    }
    updated["operationalStatus"] = json!({
        "system": V2_BED_STATUS,
        "code": if occupied { "O" } else { "U" },
        "display": if occupied { "Occupied" } else { "Unoccupied" }
    });
    updated
}

#[must_use]
pub fn is_bed_available(location: &Value) -> bool {
    operational_status_code(location)
        .map(|code| code == "U" || code.is_empty())
        .unwrap_or(true)
}

pub fn operational_status_code(location: &Value) -> Option<&str> {
    location
        .get("operationalStatus")
        .and_then(|os| {
            os.get("code")
                .and_then(|c| c.as_str())
                .or_else(|| {
                    os.get("coding")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("code"))
                        .and_then(|c| c.as_str())
                })
        })
}

/// Transaction: occupy bed + optional EpisodeOfCare + create in-progress encounter.
#[must_use]
pub fn admit_transaction(encounter: &Value, bed: &Value, episode: Option<&Value>) -> Value {
    let bed_id = bed.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let mut entries = vec![json!({
        "resource": bed_with_occupancy(bed, true),
        "request": { "method": "PUT", "url": format!("Location/{bed_id}") }
    })];

    if let Some(episode) = episode {
        let episode_id = episode
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("episode");
        entries.push(json!({
            "fullUrl": format!("urn:uuid:{episode_id}"),
            "resource": episode,
            "request": { "method": "POST", "url": "EpisodeOfCare" }
        }));
    }

    entries.push(json!({
        "fullUrl": format!("urn:uuid:{}", encounter.get("id").and_then(|v| v.as_str()).unwrap_or("enc")),
        "resource": encounter,
        "request": { "method": "POST", "url": "Encounter" }
    }));

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": entries
    })
}

/// Appointment id from Encounter.appointment (first reference).
pub fn encounter_appointment_id(encounter: &Value) -> Option<String> {
    encounter
        .get("appointment")
        .and_then(|a| a.as_array())
        .and_then(|arr| arr.first())
        .and_then(|ap| ap.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("Appointment/"))
        .map(str::to_string)
}

/// Finish an in-progress ambulatory encounter and mark the linked appointment fulfilled.
#[must_use]
pub fn finish_visit_transaction(encounter: &Value, appointment: &Value) -> Value {
    let enc_id = encounter
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let appt_id = appointment
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let end = now_datetime();

    let mut finished = encounter.clone();
    finished["status"] = json!("finished");
    if let Some(period) = finished.get_mut("period").and_then(|p| p.as_object_mut()) {
        period.insert("end".into(), json!(end));
    } else {
        finished["period"] = json!({ "end": end });
    }
    if let Some(locations) = finished.get_mut("location").and_then(|v| v.as_array_mut()) {
        for loc in locations.iter_mut() {
            let active = loc.get("status").and_then(|v| v.as_str()) == Some("active")
                || loc.get("period").and_then(|p| p.get("end")).is_none();
            if active {
                loc["status"] = json!("completed");
                if let Some(period) = loc.get_mut("period").and_then(|p| p.as_object_mut()) {
                    period.insert("end".into(), json!(end));
                } else {
                    loc["period"] = json!({ "end": end });
                }
            }
        }
    }

    let mut fulfilled = appointment.clone();
    fulfilled["status"] = json!("fulfilled");

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": [
            {
                "resource": finished,
                "request": { "method": "PUT", "url": format!("Encounter/{enc_id}") }
            },
            {
                "resource": fulfilled,
                "request": { "method": "PUT", "url": format!("Appointment/{appt_id}") }
            }
        ]
    })
}

/// Transaction: Appointment → arrived + create ambulatory Encounter.
#[must_use]
pub fn start_visit_transaction(encounter: &Value, appointment: &Value) -> Value {
    let appt_id = appointment
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let mut arrived = appointment.clone();
    arrived["status"] = json!("arrived");

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": [
            {
                "resource": arrived,
                "request": { "method": "PUT", "url": format!("Appointment/{appt_id}") }
            },
            {
                "fullUrl": format!("urn:uuid:{}", encounter.get("id").and_then(|v| v.as_str()).unwrap_or("enc")),
                "resource": encounter,
                "request": { "method": "POST", "url": "Encounter" }
            }
        ]
    })
}

/// End active location on encounter, append new bed location, swap bed occupancy.
#[must_use]
pub fn transfer_transaction(
    encounter: &Value,
    old_bed: &Value,
    new_bed: &Value,
) -> Value {
    let enc_id = encounter.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let new_bed_id = new_bed.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let old_bed_id = old_bed.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let end = now_datetime();

    let mut updated = encounter.clone();
    if let Some(locations) = updated.get_mut("location").and_then(|v| v.as_array_mut()) {
        for loc in locations.iter_mut() {
            let is_active = loc.get("status").and_then(|v| v.as_str()) == Some("active")
                || loc
                    .get("period")
                    .and_then(|p| p.get("end"))
                    .is_none();
            if is_active {
                loc["status"] = json!("completed");
                if let Some(period) = loc.get_mut("period").and_then(|p| p.as_object_mut()) {
                    period.insert("end".into(), json!(end));
                } else {
                    loc["period"] = json!({ "end": end });
                }
            }
        }
        locations.push(json!({
            "location": { "reference": format!("Location/{new_bed_id}") },
            "status": "active",
            "period": { "start": end }
        }));
    }

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": [
            {
                "resource": bed_with_occupancy(old_bed, false),
                "request": { "method": "PUT", "url": format!("Location/{old_bed_id}") }
            },
            {
                "resource": bed_with_occupancy(new_bed, true),
                "request": { "method": "PUT", "url": format!("Location/{new_bed_id}") }
            },
            {
                "resource": updated,
                "request": { "method": "PUT", "url": format!("Encounter/{enc_id}") }
            }
        ]
    })
}

/// Finish encounter and free the current bed.
#[must_use]
pub fn discharge_transaction(
    encounter: &Value,
    bed: &Value,
    episode: Option<&Value>,
    discharge_disposition: Option<&str>,
) -> Value {
    let enc_id = encounter.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let bed_id = bed.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let end = now_datetime();

    let mut updated = encounter.clone();
    updated["status"] = json!("finished");
    if let Some(period) = updated.get_mut("period").and_then(|p| p.as_object_mut()) {
        period.insert("end".into(), json!(end));
    } else {
        updated["period"] = json!({ "end": end });
    }

    if let Some(locations) = updated.get_mut("location").and_then(|v| v.as_array_mut()) {
        for loc in locations.iter_mut() {
            if loc.get("status").and_then(|v| v.as_str()) == Some("active") {
                loc["status"] = json!("completed");
                if let Some(period) = loc.get_mut("period").and_then(|p| p.as_object_mut()) {
                    period.insert("end".into(), json!(end));
                }
            }
        }
    }

    if let Some(code) = discharge_disposition.filter(|s| !s.is_empty()) {
        if let Some(hosp) = updated.get_mut("hospitalization").and_then(|h| h.as_object_mut()) {
            hosp.insert(
                "dischargeDisposition".into(),
                json!({
                    "coding": [{
                        "system": "http://terminology.hl7.org/CodeSystem/discharge-disposition",
                        "code": code
                    }]
                }),
            );
        } else {
            updated["hospitalization"] = json!({
                "dischargeDisposition": {
                    "coding": [{
                        "system": "http://terminology.hl7.org/CodeSystem/discharge-disposition",
                        "code": code
                    }]
                }
            });
        }
    }

    let mut entries = vec![
        json!({
            "resource": bed_with_occupancy(bed, false),
            "request": { "method": "PUT", "url": format!("Location/{bed_id}") }
        }),
        json!({
            "resource": updated,
            "request": { "method": "PUT", "url": format!("Encounter/{enc_id}") }
        }),
    ];

    if let Some(episode) = episode {
        let episode_id = episode
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        entries.insert(
            1,
            json!({
                "resource": episode,
                "request": { "method": "PUT", "url": format!("EpisodeOfCare/{episode_id}") }
            }),
        );
    }

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": entries
    })
}

/// Patient subject reference from an Encounter.
#[must_use]
pub fn encounter_patient_id(encounter: &Value) -> Option<String> {
    encounter
        .get("subject")
        .and_then(|s| s.get("reference"))
        .and_then(|r| r.as_str())
        .and_then(|r| r.strip_prefix("Patient/"))
        .map(str::to_string)
}

/// Attending practitioner from an Encounter participant list.
#[must_use]
pub fn encounter_practitioner_id(encounter: &Value) -> Option<String> {
    encounter
        .get("participant")
        .and_then(|v| v.as_array())
        .and_then(|parts| {
            parts.iter().find_map(|part| {
                part.get("individual")
                    .and_then(|i| i.get("reference"))
                    .and_then(|r| r.as_str())
                    .and_then(|r| r.strip_prefix("Practitioner/"))
                    .map(str::to_string)
            })
        })
}

/// Encounter class code (e.g. `AMB`, `IMP`).
#[must_use]
pub fn encounter_class_code(encounter: &Value) -> Option<String> {
    encounter
        .get("class")
        .and_then(|c| c.get("code"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// Primary human-readable reason from an Encounter.
#[must_use]
pub fn encounter_reason_text(encounter: &Value) -> Option<String> {
    encounter
        .get("reasonCode")
        .and_then(|v| v.as_array())
        .and_then(|codes| codes.first())
        .and_then(|reason| {
            reason
                .get("text")
                .and_then(|t| t.as_str())
                .map(str::to_string)
                .or_else(|| {
                    reason
                        .get("coding")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("display"))
                        .and_then(|d| d.as_str())
                        .map(str::to_string)
                })
        })
}

/// Active location reference (bed or clinic location) on an encounter.
#[must_use]
pub fn encounter_active_location_id(encounter: &Value) -> Option<String> {
    active_bed_id(encounter).or_else(|| {
        encounter
            .get("location")
            .and_then(|v| v.as_array())
            .and_then(|locs| {
                locs.iter().rev().find_map(|loc| {
                    let active = loc.get("status").and_then(|v| v.as_str()) == Some("active")
                        || loc
                            .get("period")
                            .and_then(|p| p.get("end"))
                            .is_none();
                    if !active {
                        return None;
                    }
                    loc.get("location")
                        .and_then(|l| l.get("reference"))
                        .and_then(|r| r.as_str())
                        .and_then(|r| r.strip_prefix("Location/"))
                        .map(str::to_string)
                })
            })
    })
}

/// Index of the active bed location on an in-progress encounter, if any.
pub fn active_bed_id(encounter: &Value) -> Option<String> {
    encounter
        .get("location")
        .and_then(|v| v.as_array())
        .and_then(|locs| {
            locs.iter().rev().find_map(|loc| {
                let active = loc.get("status").and_then(|v| v.as_str()) == Some("active")
                    || loc
                        .get("period")
                        .and_then(|p| p.get("end"))
                        .is_none();
                if !active {
                    return None;
                }
                loc.get("location")
                    .and_then(|l| l.get("reference"))
                    .and_then(|r| r.as_str())
                    .and_then(|r| r.strip_prefix("Location/"))
                    .map(str::to_string)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admit_encounter_has_inpatient_class() {
        let enc = build_inpatient_encounter(
            "e1",
            "pat-1",
            "bed-1",
            "org-1",
            Some("dr-patel"),
            None,
            None,
            Some("Chest pain"),
            None,
        );
        assert_eq!(enc["class"]["code"], "IMP");
        assert_eq!(enc["status"], "in-progress");
        assert_eq!(enc["location"][0]["location"]["reference"], "Location/bed-1");
        assert_eq!(enc["meta"]["profile"][0], ATRIUS_IN_ENCOUNTER);
        assert_eq!(enc["participant"][0]["individual"]["reference"], "Practitioner/dr-patel");
        assert_eq!(enc["text"]["status"], "generated");
        assert!(enc["text"]["div"].as_str().unwrap().contains("Patient pat-1"));
    }

    #[test]
    fn ambulatory_encounter_links_appointment_and_practitioner() {
        let enc = build_ambulatory_encounter(
            "e-opd",
            "pat-1",
            "org-1",
            "dr-patel",
            "appt-1",
            "2026-06-20T09:00:00+05:30",
            Some("2026-06-20T09:30:00+05:30"),
            Some("campus-1"),
            Some("Follow-up"),
        );
        assert_eq!(enc["class"]["code"], "AMB");
        assert_eq!(enc["appointment"][0]["reference"], "Appointment/appt-1");
        assert_eq!(enc["type"][0]["text"], "General consultation");
        assert_eq!(enc["location"][0]["location"]["reference"], "Location/campus-1");
    }

    #[test]
    fn start_visit_transaction_updates_appointment_and_creates_encounter() {
        let appt = json!({
            "resourceType": "Appointment",
            "id": "appt-1",
            "status": "booked"
        });
        let enc = build_ambulatory_encounter(
            "e1",
            "pat-1",
            "org-1",
            "dr-patel",
            "appt-1",
            "2026-06-20T09:00:00+05:30",
            None,
            None,
            None,
        );
        let bundle = start_visit_transaction(&enc, &appt);
        let entries = bundle["entry"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["resource"]["status"], "arrived");
        assert_eq!(entries[1]["resource"]["resourceType"], "Encounter");
    }

    #[test]
    fn finish_visit_transaction_finishes_encounter_and_fulfills_appointment() {
        let appt = json!({
            "resourceType": "Appointment",
            "id": "appt-1",
            "status": "arrived"
        });
        let enc = build_ambulatory_encounter(
            "e1",
            "pat-1",
            "org-1",
            "dr-patel",
            "appt-1",
            "2026-06-20T09:00:00+05:30",
            None,
            None,
            None,
        );
        let bundle = finish_visit_transaction(&enc, &appt);
        let entries = bundle["entry"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["resource"]["status"], "finished");
        assert!(entries[0]["resource"]["period"]["end"].is_string());
        assert_eq!(entries[1]["resource"]["status"], "fulfilled");
    }

    #[test]
    fn admit_transaction_creates_episode_and_encounter() {
        let episode = build_inpatient_episode_of_care("ep-1", "pat-1", "org-1");
        let enc = build_inpatient_encounter(
            "e1",
            "pat-1",
            "bed-1",
            "org-1",
            Some("dr-patel"),
            None,
            None,
            None,
            Some("urn:uuid:ep-1"),
        );
        let bed = json!({
            "resourceType": "Location",
            "id": "bed-1",
            "name": "Bed 1",
            "partOf": { "reference": "Location/ward-1" }
        });
        let bundle = admit_transaction(&enc, &bed, Some(&episode));
        let entries = bundle["entry"].as_array().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0]["resource"]["operationalStatus"]["code"], "O");
        assert_eq!(entries[1]["resource"]["resourceType"], "EpisodeOfCare");
        assert_eq!(entries[2]["resource"]["episodeOfCare"][0]["reference"], "urn:uuid:ep-1");
    }

    #[test]
    fn bed_with_occupancy_uses_bed_profile() {
        let bed = json!({
            "resourceType": "Location",
            "id": "bed-1",
            "name": "Bed 1",
            "partOf": { "reference": "Location/ward-1" }
        });
        let updated = bed_with_occupancy(&bed, true);
        assert_eq!(updated["meta"]["profile"][0], ATRIUS_IN_LOCATION_BED);
        assert_eq!(updated["physicalType"]["coding"][0]["code"], "bd");
    }

    #[test]
    fn discharge_transaction_finishes_linked_episode() {
        let enc = json!({
            "resourceType": "Encounter",
            "id": "enc-1",
            "status": "in-progress",
            "location": [{ "status": "active", "period": { "start": "2026-01-01" } }]
        });
        let bed = json!({ "resourceType": "Location", "id": "bed-1" });
        let episode = json!({
            "resourceType": "EpisodeOfCare",
            "id": "ep-1",
            "status": "active",
            "period": { "start": "2026-01-01" }
        });
        let finished = finish_episode_of_care(&episode);
        let bundle = discharge_transaction(&enc, &bed, Some(&finished), Some("home"));
        let entries = bundle["entry"].as_array().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1]["resource"]["status"], "finished");
        assert_eq!(entries[1]["resource"]["resourceType"], "EpisodeOfCare");
    }

    #[test]
    fn finds_active_bed_on_encounter() {
        let enc = json!({
            "location": [
                {
                    "location": { "reference": "Location/bed-old" },
                    "status": "completed",
                    "period": { "start": "2026-01-01", "end": "2026-01-02" }
                },
                {
                    "location": { "reference": "Location/bed-new" },
                    "status": "active",
                    "period": { "start": "2026-01-02" }
                }
            ]
        });
        assert_eq!(active_bed_id(&enc).as_deref(), Some("bed-new"));
    }
}
