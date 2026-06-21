//! Minimal FHIR narrative (`text.div`) generation for HIS-written resources.
//!
//! This is a placeholder until a shared narrative generator covers all resource types.

use crate::patient::{Address, BirthPlace};

const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";

/// Build a generated narrative for a Patient resource (dom-6 / `text.div` invariant).
#[must_use]
pub fn generate_patient_narrative(
    family: &str,
    given: &[&str],
    gender: &str,
    birth_date: Option<&str>,
    mrn: Option<&str>,
    birth_place: Option<&BirthPlace>,
    address: Option<&[Address]>,
) -> String {
    let name = if given.is_empty() {
        escape_html(family)
    } else {
        format!("{} {}", escape_html(&given.join(" ")), escape_html(family))
    };

    let mut parts = vec![format!("Patient {name}")];

    if let Some(mrn) = mrn.filter(|s| !s.is_empty()) {
        parts.push(format!("MRN {mrn}"));
    }

    parts.push(format!("Gender {}", escape_html(gender)));

    if let Some(bd) = birth_date.filter(|s| !s.is_empty()) {
        parts.push(format!("Born {}", escape_html(bd)));
    }

    if let Some(bp) = birth_place {
        if let Some(label) = format_place(bp) {
            parts.push(format!("Birth place {label}"));
        }
    }

    if let Some(items) = address {
        if let Some(first) = items.first() {
            if let Some(label) = format_address(first) {
                parts.push(format!("Address {label}"));
            }
        }
    }

    let body = parts.join(". ");
    format!(r#"<div xmlns="{XHTML_NS}"><p>{body}</p></div>"#)
}

fn format_place(place: &BirthPlace) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(city) = place.city.as_deref().filter(|s| !s.is_empty()) {
        parts.push(escape_html(city));
    }
    if let Some(state) = place.state.as_deref().filter(|s| !s.is_empty()) {
        parts.push(escape_html(state));
    }
    if let Some(country) = place.country.as_deref().filter(|s| !s.is_empty()) {
        parts.push(escape_html(country));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn format_address(address: &Address) -> Option<String> {
    let mut parts = Vec::new();
    for line in &address.line {
        if !line.is_empty() {
            parts.push(escape_html(line));
        }
    }
    if let Some(city) = address.city.as_deref().filter(|s| !s.is_empty()) {
        parts.push(escape_html(city));
    }
    if let Some(state) = address.state.as_deref().filter(|s| !s.is_empty()) {
        parts.push(escape_html(state));
    }
    if let Some(postal) = address.postal_code.as_deref().filter(|s| !s.is_empty()) {
        parts.push(escape_html(postal));
    }
    if let Some(country) = address.country.as_deref().filter(|s| !s.is_empty()) {
        parts.push(escape_html(country));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Build a generated narrative for an Encounter resource (dom-6 / `text.div` invariant).
#[must_use]
pub fn generate_encounter_narrative(
    class_display: &str,
    status: &str,
    patient_id: &str,
    period_start: &str,
    reason: Option<&str>,
) -> String {
    let mut parts = vec![
        format!("Encounter {}", escape_html(class_display)),
        format!("Status {}", escape_html(status)),
        format!("Patient {}", escape_html(patient_id)),
        format!("Started {}", escape_html(period_start)),
    ];

    if let Some(reason_text) = reason.filter(|s| !s.is_empty()) {
        parts.push(format!("Reason {}", escape_html(reason_text)));
    }

    let body = parts.join(". ");
    format!(r#"<div xmlns="{XHTML_NS}"><p>{body}</p></div>"#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encounter_narrative_includes_class_and_patient() {
        let div = generate_encounter_narrative(
            "ambulatory",
            "in-progress",
            "pat-1",
            "2026-07-03T16:30:00+05:30",
            Some("General consultation"),
        );
        assert!(div.contains("xmlns=\"http://www.w3.org/1999/xhtml\""));
        assert!(div.contains("Encounter ambulatory"));
        assert!(div.contains("Patient pat-1"));
        assert!(div.contains("General consultation"));
    }

    #[test]
    fn patient_narrative_includes_name_and_mrn() {
        let div = generate_patient_narrative(
            "Sharma",
            &["Priya"],
            "female",
            Some("1990-01-01"),
            Some("MRN-001"),
            None,
            None,
        );
        assert!(div.contains("xmlns=\"http://www.w3.org/1999/xhtml\""));
        assert!(div.contains("Patient Priya Sharma"));
        assert!(div.contains("MRN MRN-001"));
        assert!(div.contains("Born 1990-01-01"));
    }

    #[test]
    fn escapes_html_in_narrative() {
        let div = generate_patient_narrative(
            "O<Brien",
            &["Pat & Co"],
            "other",
            None,
            None,
            None,
            None,
        );
        assert!(div.contains("O&lt;Brien"));
        assert!(div.contains("Pat &amp; Co"));
    }
}
