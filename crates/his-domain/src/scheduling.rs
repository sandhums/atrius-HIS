use crate::profiles::{
    ATRIUS_IN_APPOINTMENT, ATRIUS_IN_APPOINTMENT_VISIT_MODE, ATRIUS_IN_SLOT,
};
use serde_json::{Value, json};

/// Build a booked Appointment linked to a Slot and participants.
#[must_use]
pub fn build_appointment(
    id: &str,
    patient_id: &str,
    slot: &SlotTiming,
    practitioner_id: Option<&str>,
    location_id: Option<&str>,
    description: Option<&str>,
) -> Value {
    let mut participants = vec![json!({
        "actor": { "reference": format!("Patient/{patient_id}") },
        "status": "accepted"
    })];

    if let Some(pid) = practitioner_id {
        participants.push(json!({
            "actor": { "reference": format!("Practitioner/{pid}") },
            "status": "accepted"
        }));
    }

    if let Some(lid) = location_id {
        participants.push(json!({
            "actor": { "reference": format!("Location/{lid}") },
            "status": "accepted"
        }));
    }

    let mut appt = json!({
        "resourceType": "Appointment",
        "id": id,
        "meta": {
            "profile": [ATRIUS_IN_APPOINTMENT]
        },
        "status": "booked",
        "slot": [{ "reference": format!("Slot/{}", slot.id) }],
        "start": slot.start,
        "end": slot.end,
        "extension": [{
            "url": ATRIUS_IN_APPOINTMENT_VISIT_MODE,
            "valueCode": "in-person"
        }],
        "participant": participants,
    });

    if let Some(desc) = description.filter(|s| !s.is_empty()) {
        appt["description"] = json!(desc);
    }

    appt
}

/// Slot timing fields used when constructing appointments.
#[derive(Debug, Clone)]
pub struct SlotTiming {
    pub id: String,
    pub start: String,
    pub end: String,
}

/// Mark a Slot as busy (or free) while preserving schedule linkage and times.
#[must_use]
pub fn slot_with_status(slot: &Value, status: &str) -> Value {
    let mut updated = slot.clone();
    updated["meta"] = json!({ "profile": [ATRIUS_IN_SLOT] });
    updated["status"] = json!(status);
    updated
}

/// FHIR transaction Bundle for atomic book: Slot → busy + Appointment → booked.
#[must_use]
pub fn book_appointment_transaction(appointment: &Value, slot: &Value) -> Value {
    let slot_id = slot
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let busy_slot = slot_with_status(slot, "busy");

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": [
            {
                "resource": busy_slot,
                "request": { "method": "PUT", "url": format!("Slot/{slot_id}") }
            },
            {
                "fullUrl": format!("urn:uuid:{}", appointment.get("id").and_then(|v| v.as_str()).unwrap_or("appt")),
                "resource": appointment,
                "request": { "method": "POST", "url": "Appointment" }
            }
        ]
    })
}

/// Transaction: Appointment → cancelled + linked Slot(s) → free.
#[must_use]
pub fn cancel_appointment_transaction(
    appointment: &Value,
    slots: &[Value],
) -> Value {
    let mut cancelled = appointment.clone();
    cancelled["status"] = json!("cancelled");

    let appt_id = appointment
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let mut entries = vec![json!({
        "resource": cancelled,
        "request": { "method": "PUT", "url": format!("Appointment/{appt_id}") }
    })];

    for slot in slots {
        let slot_id = slot.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        entries.push(json!({
            "resource": slot_with_status(slot, "free"),
            "request": { "method": "PUT", "url": format!("Slot/{slot_id}") }
        }));
    }

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": entries
    })
}

/// Transaction: release old slot, occupy new slot, update appointment times/slot link.
#[must_use]
pub fn reschedule_appointment_transaction(
    appointment: &Value,
    old_slots: &[Value],
    new_slot: &Value,
) -> Value {
    let new_timing = slot_timing_from_resource(new_slot);
    let mut updated = appointment.clone();
    updated["status"] = json!("booked");
    updated["slot"] = json!([{ "reference": format!("Slot/{}", new_timing.id) }]);
    updated["start"] = json!(new_timing.start);
    updated["end"] = json!(new_timing.end);

    let appt_id = appointment
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let mut entries = Vec::new();

    for slot in old_slots {
        let slot_id = slot.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        entries.push(json!({
            "resource": slot_with_status(slot, "free"),
            "request": { "method": "PUT", "url": format!("Slot/{slot_id}") }
        }));
    }

    let new_slot_id = new_slot.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    entries.push(json!({
        "resource": slot_with_status(new_slot, "busy"),
        "request": { "method": "PUT", "url": format!("Slot/{new_slot_id}") }
    }));

    entries.push(json!({
        "resource": updated,
        "request": { "method": "PUT", "url": format!("Appointment/{appt_id}") }
    }));

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": entries
    })
}

#[must_use]
pub fn slot_timing_from_resource(slot: &Value) -> SlotTiming {
    SlotTiming {
        id: slot
            .get("id")
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

/// Parse Slot references from an Appointment (`Appointment.slot`).
pub fn appointment_slot_ids(appointment: &Value) -> Vec<String> {
    appointment
        .get("slot")
        .and_then(|v| v.as_array())
        .map(|refs| {
            refs.iter()
                .filter_map(|r| {
                    r.get("reference")
                        .and_then(|v| v.as_str())
                        .and_then(|ref_str| ref_str.strip_prefix("Slot/"))
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Patient actor reference from Appointment participants.
pub fn appointment_patient_id(appointment: &Value) -> Option<String> {
    appointment_participant_ref(appointment, "Patient/")
}

/// Practitioner actor reference from Appointment participants.
pub fn appointment_practitioner_id(appointment: &Value) -> Option<String> {
    appointment_participant_ref(appointment, "Practitioner/")
}

/// Location actor reference from Appointment participants.
pub fn appointment_location_id(appointment: &Value) -> Option<String> {
    appointment_participant_ref(appointment, "Location/")
}

fn appointment_participant_ref(appointment: &Value, prefix: &str) -> Option<String> {
    appointment
        .get("participant")
        .and_then(|v| v.as_array())
        .and_then(|parts| {
            parts.iter().find_map(|part| {
                part.get("actor")
                    .and_then(|a| a.get("reference"))
                    .and_then(|r| r.as_str())
                    .and_then(|r| r.strip_prefix(prefix))
                    .map(str::to_string)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_appointment_declares_atrius_profile() {
        let slot = json!({
            "resourceType": "Slot",
            "id": "s1",
            "start": "2026-06-20T09:00:00+05:30",
            "end": "2026-06-20T09:30:00+05:30"
        });
        let appt = build_appointment(
            "a1",
            "pat-1",
            &slot_timing_from_resource(&slot),
            None,
            None,
            None,
        );
        assert_eq!(appt["meta"]["profile"][0], ATRIUS_IN_APPOINTMENT);
        assert_eq!(
            appt["extension"][0]["url"],
            ATRIUS_IN_APPOINTMENT_VISIT_MODE
        );
        assert_eq!(appt["extension"][0]["valueCode"], "in-person");
    }

    #[test]
    fn slot_with_status_sets_atrius_profile() {
        let slot = json!({
            "resourceType": "Slot",
            "id": "s1",
            "status": "free"
        });
        let busy = slot_with_status(&slot, "busy");
        assert_eq!(busy["meta"]["profile"][0], ATRIUS_IN_SLOT);
        assert_eq!(busy["status"], "busy");
    }

    #[test]
    fn book_transaction_updates_slot_before_appointment() {
        let slot = json!({
            "resourceType": "Slot",
            "id": "s1",
            "status": "free",
            "schedule": { "reference": "Schedule/sch1" },
            "start": "2026-06-20T09:00:00+05:30",
            "end": "2026-06-20T09:30:00+05:30"
        });
        let appt = build_appointment("a1", "pat-1", &slot_timing_from_resource(&slot), Some("dr-patel"), None, None);
        let bundle = book_appointment_transaction(&appt, &slot);
        let entries = bundle["entry"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["resource"]["status"], "busy");
        assert_eq!(entries[1]["resource"]["status"], "booked");
    }

    #[test]
    fn extracts_participant_refs_from_appointment() {
        let appt = json!({
            "participant": [
                { "actor": { "reference": "Patient/pat-1" }, "status": "accepted" },
                { "actor": { "reference": "Practitioner/dr-patel" }, "status": "accepted" },
                { "actor": { "reference": "Location/campus-1" }, "status": "accepted" }
            ]
        });
        assert_eq!(appointment_patient_id(&appt).as_deref(), Some("pat-1"));
        assert_eq!(appointment_practitioner_id(&appt).as_deref(), Some("dr-patel"));
        assert_eq!(appointment_location_id(&appt).as_deref(), Some("campus-1"));
    }

    #[test]
    fn extracts_slot_ids_from_appointment() {
        let appt = json!({
            "slot": [{ "reference": "Slot/s1" }, { "reference": "Slot/s2" }]
        });
        assert_eq!(appointment_slot_ids(&appt), vec!["s1", "s2"]);
    }
}
