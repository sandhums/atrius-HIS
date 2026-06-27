//! Expand Atrius Schedule recurrence (RFC 5545 RRULE) into profile-conformant Slots.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, TimeZone, Timelike, Weekday};
use serde_json::{Value, json};

use crate::profiles::{ATRIUS_IN_SCHEDULE_RECURRENCE, ATRIUS_IN_SLOT};

const DEFAULT_SLOT_MINUTES: i64 = 30;
const DEFAULT_START_HOUR: u32 = 9;
const DEFAULT_END_HOUR: u32 = 17;

/// Parsed recurrence extension on an Atrius Schedule.
#[derive(Debug, Clone)]
pub struct ScheduleRecurrence {
    pub rrule: String,
    pub tzid: String,
    pub exdates: Vec<DateTime<FixedOffset>>,
    pub rdates: Vec<DateTime<FixedOffset>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExpandError {
    #[error("schedule missing planningHorizon")]
    MissingPlanningHorizon,
    #[error("schedule missing recurrence extension")]
    MissingRecurrence,
    #[error("invalid planningHorizon: {0}")]
    InvalidPlanningHorizon(String),
    #[error("invalid recurrence: {0}")]
    InvalidRecurrence(String),
    #[error("invalid date range: {0}")]
    InvalidDateRange(String),
}

/// Build an Atrius-in-Schedule resource with optional recurrence extension.
#[must_use]
pub fn build_schedule(
    id: &str,
    profile_url: &str,
    actors: &[Value],
    planning_horizon_start: &str,
    planning_horizon_end: &str,
    recurrence: Option<(&str, &str)>,
) -> Value {
    let mut schedule = json!({
        "resourceType": "Schedule",
        "id": id,
        "meta": { "profile": [profile_url] },
        "active": true,
        "actor": actors,
        "planningHorizon": {
            "start": planning_horizon_start,
            "end": planning_horizon_end
        }
    });

    if let Some((rrule, tzid)) = recurrence {
        schedule["extension"] = json!([schedule_recurrence_extension(rrule, tzid)]);
    }

    schedule
}

#[must_use]
pub fn schedule_recurrence_extension(rrule: &str, tzid: &str) -> Value {
    json!({
        "url": ATRIUS_IN_SCHEDULE_RECURRENCE,
        "extension": [
            { "url": "RRULE", "valueString": rrule },
            { "url": "TZID", "valueString": tzid }
        ]
    })
}

/// Build a free Slot linked to a Schedule.
#[must_use]
pub fn build_slot(id: &str, schedule_id: &str, start: &str, end: &str, status: &str) -> Value {
    json!({
        "resourceType": "Slot",
        "id": id,
        "meta": { "profile": [ATRIUS_IN_SLOT] },
        "schedule": { "reference": format!("Schedule/{schedule_id}") },
        "status": status,
        "start": start,
        "end": end
    })
}

/// Transaction bundle that PUTs materialized Slots (idempotent expansion).
#[must_use]
pub fn expand_slots_transaction(slots: &[Value]) -> Value {
    let entries: Vec<Value> = slots
        .iter()
        .map(|slot| {
            let slot_id = slot
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            json!({
                "resource": slot,
                "request": { "method": "PUT", "url": format!("Slot/{slot_id}") }
            })
        })
        .collect();

    json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": entries
    })
}

/// Deterministic Slot id for idempotent expansion.
#[must_use]
pub fn slot_id_for_instant(schedule_id: &str, start: DateTime<FixedOffset>) -> String {
    format!(
        "slot-{}-{}-{:02}{:02}",
        schedule_id,
        start.format("%Y%m%d"),
        start.hour(),
        start.minute()
    )
}

pub fn parse_schedule_recurrence(schedule: &Value) -> Result<ScheduleRecurrence, ExpandError> {
    let extension = schedule
        .get("extension")
        .and_then(|v| v.as_array())
        .and_then(|exts| {
            exts.iter().find(|ext| {
                ext.get("url")
                    .and_then(|u| u.as_str())
                    .is_some_and(|u| u == ATRIUS_IN_SCHEDULE_RECURRENCE)
            })
        })
        .ok_or(ExpandError::MissingRecurrence)?;

    let nested = extension
        .get("extension")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ExpandError::InvalidRecurrence("missing nested extensions".into()))?;

    let mut rrule = None;
    let mut tzid = None;
    let mut exdates = Vec::new();
    let mut rdates = Vec::new();

    for part in nested {
        let url = part.get("url").and_then(|v| v.as_str()).unwrap_or("");
        match url {
            "RRULE" => {
                rrule = part
                    .get("valueString")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            "TZID" => {
                tzid = part
                    .get("valueString")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
            "EXDATE" => {
                if let Some(dt) = part.get("valueDateTime").and_then(|v| v.as_str()) {
                    exdates.push(parse_fhir_datetime(dt)?);
                }
            }
            "RDATE" => {
                if let Some(dt) = part.get("valueDateTime").and_then(|v| v.as_str()) {
                    rdates.push(parse_fhir_datetime(dt)?);
                }
            }
            _ => {}
        }
    }

    let rrule = rrule.ok_or_else(|| ExpandError::InvalidRecurrence("RRULE required".into()))?;
    let tzid = tzid.ok_or_else(|| ExpandError::InvalidRecurrence("TZID required".into()))?;

    Ok(ScheduleRecurrence {
        rrule,
        tzid,
        exdates,
        rdates,
    })
}

pub fn expand_schedule_slots(
    schedule: &Value,
    from: DateTime<FixedOffset>,
    to: DateTime<FixedOffset>,
) -> Result<Vec<Value>, ExpandError> {
    if from > to {
        return Err(ExpandError::InvalidDateRange(
            "from must be before or equal to to".into(),
        ));
    }

    let schedule_id = schedule
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ExpandError::InvalidRecurrence("schedule missing id".into()))?;

    let horizon = schedule
        .get("planningHorizon")
        .ok_or(ExpandError::MissingPlanningHorizon)?;
    let horizon_start = parse_fhir_datetime(
        horizon
            .get("start")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExpandError::InvalidPlanningHorizon("start required".into()))?,
    )?;
    let horizon_end = parse_fhir_datetime(
        horizon
            .get("end")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExpandError::InvalidPlanningHorizon("end required".into()))?,
    )?;

    let range_start = from.max(horizon_start);
    let range_end = to.min(horizon_end);
    if range_start > range_end {
        return Ok(Vec::new());
    }

    let recurrence = parse_schedule_recurrence(schedule)?;
    let rrule_parts = parse_rrule(&recurrence.rrule)?;
    let weekdays = rrule_weekdays(&rrule_parts)?;
    let start_hour = rrule_parts
        .get("BYHOUR")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(DEFAULT_START_HOUR);
    let end_hour = DEFAULT_END_HOUR;
    let offset = ist_offset();

    let mut slots = Vec::new();
    let mut seen_ids = HashSet::new();

    let mut day = range_start.date_naive();
    let last_day = range_end.date_naive();

    while day <= last_day {
        if weekdays.contains(&day.weekday()) && !is_excluded_day(day, &recurrence.exdates, offset) {
            for minute in [0u32, 30] {
                for hour in start_hour..end_hour {
                    if let Some(start) = offset
                        .from_local_datetime(
                            &day.and_hms_opt(hour, minute, 0)
                                .ok_or_else(|| ExpandError::InvalidDateRange("bad local time".into()))?,
                        )
                        .single()
                    {
                        let end = start + Duration::minutes(DEFAULT_SLOT_MINUTES);
                        if start < range_start || start > range_end {
                            continue;
                        }
                        push_slot(
                            &mut slots,
                            &mut seen_ids,
                            schedule_id,
                            start,
                            end,
                            "free",
                        );
                    }
                }
            }
        }
        day += Duration::days(1);
    }

    for rdate in &recurrence.rdates {
        if *rdate < range_start || *rdate > range_end {
            continue;
        }
        let end = *rdate + Duration::minutes(DEFAULT_SLOT_MINUTES);
        push_slot(
            &mut slots,
            &mut seen_ids,
            schedule_id,
            *rdate,
            end,
            "free",
        );
    }

    slots.sort_by(|a, b| {
        a.get("start")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("start").and_then(|v| v.as_str()).unwrap_or(""))
    });

    Ok(slots)
}

fn push_slot(
    slots: &mut Vec<Value>,
    seen_ids: &mut HashSet<String>,
    schedule_id: &str,
    start: DateTime<FixedOffset>,
    end: DateTime<FixedOffset>,
    status: &str,
) {
    let id = slot_id_for_instant(schedule_id, start);
    if !seen_ids.insert(id.clone()) {
        return;
    }
    slots.push(build_slot(
        &id,
        schedule_id,
        &format_fhir_datetime(start),
        &format_fhir_datetime(end),
        status,
    ));
}

fn is_excluded_day(day: NaiveDate, exdates: &[DateTime<FixedOffset>], offset: FixedOffset) -> bool {
    exdates.iter().any(|ex| {
        ex.date_naive() == day
            || ex
                .with_timezone(&offset)
                .date_naive()
                == day
    })
}

fn parse_rrule(rrule: &str) -> Result<HashMap<String, String>, ExpandError> {
    let mut parts = HashMap::new();
    for segment in rrule.split(';') {
        let Some((key, value)) = segment.split_once('=') else {
            continue;
        };
        parts.insert(key.trim().to_uppercase(), value.trim().to_string());
    }
    if !parts.contains_key("FREQ") {
        return Err(ExpandError::InvalidRecurrence(
            "RRULE must include FREQ".into(),
        ));
    }
    Ok(parts)
}

fn rrule_weekdays(parts: &HashMap<String, String>) -> Result<HashSet<Weekday>, ExpandError> {
    let byday = parts.get("BYDAY").cloned().unwrap_or_else(|| {
        "MO,TU,WE,TH,FR".to_string()
    });
    let mut days = HashSet::new();
    for token in byday.split(',') {
        days.insert(parse_weekday(token.trim())?);
    }
    Ok(days)
}

fn parse_weekday(token: &str) -> Result<Weekday, ExpandError> {
    match token.to_ascii_uppercase().as_str() {
        "MO" => Ok(Weekday::Mon),
        "TU" => Ok(Weekday::Tue),
        "WE" => Ok(Weekday::Wed),
        "TH" => Ok(Weekday::Thu),
        "FR" => Ok(Weekday::Fri),
        "SA" => Ok(Weekday::Sat),
        "SU" => Ok(Weekday::Sun),
        other => Err(ExpandError::InvalidRecurrence(format!(
            "unsupported BYDAY token: {other}"
        ))),
    }
}

pub fn parse_fhir_datetime(value: &str) -> Result<DateTime<FixedOffset>, ExpandError> {
    DateTime::parse_from_rfc3339(value).map_err(|err| {
        ExpandError::InvalidPlanningHorizon(format!("cannot parse '{value}': {err}"))
    })
}

pub fn parse_date_or_datetime(value: &str, end_of_day: bool) -> Result<DateTime<FixedOffset>, ExpandError> {
    if value.contains('T') {
        return parse_fhir_datetime(value);
    }
    let day = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|err| {
        ExpandError::InvalidDateRange(format!("cannot parse date '{value}': {err}"))
    })?;
    let offset = ist_offset();
    let time = if end_of_day {
        day.and_hms_opt(23, 59, 59)
    } else {
        day.and_hms_opt(0, 0, 0)
    }
    .ok_or_else(|| ExpandError::InvalidDateRange("invalid date boundary".into()))?;
    offset
        .from_local_datetime(&time)
        .single()
        .ok_or_else(|| ExpandError::InvalidDateRange("ambiguous local datetime".into()))
}

fn format_fhir_datetime(dt: DateTime<FixedOffset>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

fn ist_offset() -> FixedOffset {
    FixedOffset::east_opt(5 * 3600 + 30 * 60).expect("valid IST offset")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_schedule() -> Value {
        build_schedule(
            "opd-patel-schedule",
            crate::profiles::ATRIUS_IN_SCHEDULE,
            &[json!({"reference": "Practitioner/dr-patel"})],
            "2026-06-15T00:00:00+05:30",
            "2026-06-29T23:59:59+05:30",
            Some((
                "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR;BYHOUR=9;BYMINUTE=0;BYSECOND=0",
                "Asia/Kolkata",
            )),
        )
    }

    #[test]
    fn expands_weekday_slots_with_deterministic_ids() {
        let schedule = sample_schedule();
        let from = parse_date_or_datetime("2026-06-16", false).unwrap();
        let to = parse_date_or_datetime("2026-06-16", true).unwrap();
        let slots = expand_schedule_slots(&schedule, from, to).unwrap();
        assert_eq!(slots.len(), 16);
        assert_eq!(
            slots[0]["id"],
            "slot-opd-patel-schedule-20260616-0900"
        );
        assert_eq!(slots[0]["meta"]["profile"][0], ATRIUS_IN_SLOT);
    }

    #[test]
    fn skips_weekends() {
        let schedule = sample_schedule();
        let from = parse_date_or_datetime("2026-06-20", false).unwrap();
        let to = parse_date_or_datetime("2026-06-21", true).unwrap();
        let slots = expand_schedule_slots(&schedule, from, to).unwrap();
        assert!(slots.is_empty());
    }

    #[test]
    fn expand_slots_transaction_uses_put() {
        let schedule = sample_schedule();
        let from = parse_date_or_datetime("2026-06-16", false).unwrap();
        let to = parse_date_or_datetime("2026-06-16", true).unwrap();
        let slots = expand_schedule_slots(&schedule, from, to).unwrap();
        let bundle = expand_slots_transaction(&slots);
        assert_eq!(bundle["type"], "transaction");
        assert_eq!(bundle["entry"][0]["request"]["method"], "PUT");
    }
}
