//! Validate bed Location occupancy updates against the Atrius bed profile manifest.

use fhir_validation::profile::profile_registry::ProfileRegistry;
use fhir_validation::{load_profile_registry_from_manifest_file, LocalTerminologyService, Severity, Validator};
use helios_fhir::FhirResource;
use helios_fhir::FhirVersion;
use helios_fhir::r4::Resource;
use his_domain::{bed_with_occupancy, build_inpatient_episode_of_care, ATRIUS_IN_EPISODE_OF_CARE};
use serde_json::json;
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

fn assert_resource_valid(resource_json: serde_json::Value) {
    let (validator, registry) = validator_and_registry();
    let term = LocalTerminologyService::new(FhirVersion::R4);
    let resource: Resource = serde_json::from_value(resource_json).expect("parse resource");
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
        "expected profile-valid resource, got {} errors: {:?}",
        errors.len(),
        errors
            .iter()
            .map(|e| format!("{}: {}", e.fhir_path, e.diagnostics))
            .collect::<Vec<_>>()
    );
}

#[test]
fn occupied_bed_location_passes_atrius_bed_profile() {
    let bed = json!({
        "resourceType": "Location",
        "id": "bed-med-a-01",
        "status": "active",
        "name": "Medical Ward A — Bed 1",
        "mode": "instance",
        "physicalType": {
            "coding": [{
                "system": "http://terminology.hl7.org/CodeSystem/location-physical-type",
                "code": "bd",
                "display": "Bed"
            }]
        },
        "partOf": { "reference": "Location/ward-med-a" },
        "managingOrganization": { "reference": "Organization/atrius-demo-hospital" }
    });
    assert_resource_valid(bed_with_occupancy(&bed, true));
}

#[test]
fn inpatient_episode_of_care_builder_has_atrius_profile() {
    let episode = build_inpatient_episode_of_care(
        "ep-test",
        "pat-1",
        "atrius-demo-hospital",
    );
    assert_eq!(episode["resourceType"], "EpisodeOfCare");
    assert_eq!(
        episode["meta"]["profile"][0].as_str().unwrap(),
        ATRIUS_IN_EPISODE_OF_CARE
    );
    assert_eq!(episode["status"], "active");
    assert_eq!(episode["type"][0]["coding"][0]["code"], "inp");
    assert_eq!(episode["patient"]["reference"], "Patient/pat-1");
    assert!(episode.get("period").and_then(|p| p.get("start")).is_some());
}
