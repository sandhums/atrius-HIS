//! Validate OP consult Composition builder shape against NDHM-equivalent Atrius profile.

use his_domain::{
    ATRIUS_IN_OP_CONSULT_RECORD, ConsultNoteSections, build_consultation_composition,
    finalize_consultation_composition, op_consult_transaction,
};

#[test]
fn preliminary_op_consult_composition_has_ndhm_shape() {
    let sections = ConsultNoteSections {
        chief_complaint: Some("Headache".into()),
        hpi: Some("3 days, non-focal".into()),
        exam: Some("Normal neuro exam".into()),
        assessment: Some("Tension headache".into()),
        plan: Some("Analgesia and follow up".into()),
        ..Default::default()
    };
    let composition = build_consultation_composition(
        "comp-test",
        "pat-1",
        "enc-1",
        "dr-patel",
        "OPD Consultation Note",
        &sections,
    );

    assert_eq!(
        composition["meta"]["profile"][0],
        ATRIUS_IN_OP_CONSULT_RECORD
    );
    assert_eq!(composition["type"]["coding"][0]["code"], "371530004");
    assert_eq!(composition["section"].as_array().unwrap().len(), 5);

    let chief = &composition["section"][0];
    assert_eq!(chief["code"]["coding"][0]["code"], "422843007");
    assert_eq!(
        chief["entry"][0]["reference"],
        "Condition/comp-test-cc"
    );
}

#[test]
fn op_consult_transaction_bundle_includes_composition_and_entry_resources() {
    let sections = ConsultNoteSections {
        chief_complaint: Some("Headache".into()),
        assessment: Some("Tension headache".into()),
        ..Default::default()
    };
    let bundle = op_consult_transaction(
        "comp-test",
        "pat-1",
        "enc-1",
        "dr-patel",
        "OPD Consultation Note",
        &sections,
    );
    assert_eq!(bundle["type"], "transaction");
    assert_eq!(bundle["entry"].as_array().unwrap().len(), 3);

    let types: Vec<_> = bundle["entry"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["resource"]["resourceType"].as_str())
        .collect();
    assert!(types.contains(&"Condition"));
    assert!(types.contains(&"Observation"));
    assert!(types.contains(&"Composition"));
}

#[test]
fn finalized_consultation_composition_has_attester() {
    let sections = ConsultNoteSections {
        assessment: Some("Stable".into()),
        ..Default::default()
    };
    let composition = build_consultation_composition(
        "comp-final",
        "pat-1",
        "enc-1",
        "dr-patel",
        "Note",
        &sections,
    );
    let final_comp = finalize_consultation_composition(&composition, "dr-patel");
    assert_eq!(final_comp["status"], "final");
    assert_eq!(
        final_comp["attester"][0]["party"]["reference"],
        "Practitioner/dr-patel"
    );
}
