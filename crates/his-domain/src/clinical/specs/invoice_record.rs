//! Invoice record Composition (NDHM InvoiceRecord shape).

use serde_json::Value;

use crate::clinical::specs::entry_sliced::{
    CompositionTypeCoding, EntrySlicedMeta, build_invoice_entry, entry_sliced_transaction,
    entry_sliced_update_transaction,
};
use crate::profiles::ATRIUS_IN_INVOICE_RECORD;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InvoiceRecordSections {
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_inr: Option<String>,
}

impl InvoiceRecordSections {
    pub fn has_content(&self) -> bool {
        !self.summary.trim().is_empty()
    }
}

const META: EntrySlicedMeta = EntrySlicedMeta {
    profile: ATRIUS_IN_INVOICE_RECORD,
    composition_type: CompositionTypeCoding::Text("Invoice Record"),
    title_narrative: "Invoice Record",
};

#[must_use]
pub fn invoice_record_transaction(
    composition_id: &str,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &InvoiceRecordSections,
) -> Value {
    let (resources, refs) = build_invoice_entry(
        composition_id,
        patient_id,
        &sections.summary,
        sections.amount_inr.as_deref(),
    );
    entry_sliced_transaction(
        &META,
        composition_id,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        &sections.summary,
        resources,
        refs,
    )
}

#[must_use]
pub fn invoice_record_update_transaction(
    composition: &Value,
    patient_id: &str,
    encounter_id: &str,
    practitioner_id: &str,
    title: &str,
    sections: &InvoiceRecordSections,
) -> Value {
    let composition_id = composition
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let (resources, refs) = build_invoice_entry(
        composition_id,
        patient_id,
        &sections.summary,
        sections.amount_inr.as_deref(),
    );
    entry_sliced_update_transaction(
        &META,
        composition,
        patient_id,
        encounter_id,
        practitioner_id,
        title,
        &sections.summary,
        resources,
        refs,
    )
}
