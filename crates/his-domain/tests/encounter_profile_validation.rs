//! Validate HIS encounter builders against the Atrius core profile manifest.

use fhir_validation::profile::profile_registry::ProfileRegistry;
use fhir_validation::{load_profile_registry_from_manifest_file, LocalTerminologyService, Severity, Validator};
use helios_fhir::FhirResource;
use helios_fhir::FhirVersion;
use helios_fhir::r4::Resource;
use his_domain::{build_ambulatory_encounter, build_inpatient_encounter};
use std::path::PathBuf;

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../atrius-hfs/manifests/atrius-r4-profile-manifest-core.json")
}

fn validator_and_registry() -> (Validator, ProfileRegistry) {
    let mut validator = Validator::default();
    validator.config.recurse_on_base_definition = false;
    validator.config.enable_base_definition_url_lookup = false;
    validator.config.strict_extensible_bindings = false;

    let registry =
        load_profile_registry_from_manifest_file(&manifest_path()).expect("load Atrius manifest");
    (validator, registry)
}

fn assert_encounter_valid(encounter_json: serde_json::Value) {
    let (validator, registry) = validator_and_registry();
    let term = LocalTerminologyService::new(FhirVersion::R4);
    let resource: Resource = serde_json::from_value(encounter_json).expect("parse Encounter");
    let evaluator = fhir_validation::R4FhirPathEvaluator::new(resource.clone());
    let issues = validator.validate_resource_with_profiles(
        &FhirResource::R4(Box::new(resource)),
        Some(&term),
        &evaluator,
        &registry,
    );

    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();

    assert!(
        errors.is_empty(),
        "expected profile-valid Encounter, got {} errors: {:?}",
        errors.len(),
        errors
            .iter()
            .map(|e| format!("{}: {}", e.fhir_path, e.diagnostics))
            .collect::<Vec<_>>()
    );
}

#[test]
fn inpatient_encounter_builder_passes_atrius_profile() {
    let encounter = build_inpatient_encounter(
        "enc-ip-test",
        "pat-1",
        "bed-med-a-01",
        "atrius-demo-hospital",
        Some("dr-patel"),
        None,
        Some("other"),
        Some("Planned admission"),
        None,
    );
    assert_encounter_valid(encounter);
}

#[test]
fn ambulatory_encounter_builder_passes_atrius_profile() {
    let encounter = build_ambulatory_encounter(
        "enc-opd-test",
        "pat-1",
        "atrius-demo-hospital",
        "dr-patel",
        "appt-1",
        "2026-06-20T09:00:00+05:30",
        Some("2026-06-20T09:30:00+05:30"),
        Some("atrius-demo-campus"),
        Some("Follow-up visit"),
    );
    assert_encounter_valid(encounter);
}
