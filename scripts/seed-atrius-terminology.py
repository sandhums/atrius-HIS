#!/usr/bin/env python3
"""Seed minimal Atrius + HL7 ValueSets into HTS for HIS Encounter profile validation.

Imports CodeSystems and ValueSets required by atrius-in-encounter bindings when HFS
$validate uses remote terminology (HFS_TERMINOLOGY_SERVER → HTS).

Usage:
  python3 scripts/seed-atrius-terminology.py \\
    --hts-url http://127.0.0.1:9091
"""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request

V3_ACT_CODE = "http://terminology.hl7.org/CodeSystem/v3-ActCode"
V3_PARTICIPATION = "http://terminology.hl7.org/CodeSystem/v3-ParticipationType"
EPISODE_OF_CARE_TYPE = "http://terminology.hl7.org/CodeSystem/episodeofcare-type"
V2_BED_STATUS = "http://terminology.hl7.org/CodeSystem/v2-0116"
LOINC = "http://loinc.org"
DOC_CLASS = "http://terminology.hl7.org/CodeSystem/document-classcodes"
ATRIUS_ENCOUNTER_CLASS_VS = (
    "https://atrius.in/fhir/r4/atrius-in/ValueSet/atrius-in-encounter-class"
)
HL7_ENCOUNTER_PARTICIPANT_TYPE_VS = "http://hl7.org/fhir/ValueSet/encounter-participant-type"

ENCOUNTER_CLASS_CODES = [
    ("AMB", "ambulatory"),
    ("IMP", "inpatient encounter"),
    ("EMER", "emergency"),
    ("VR", "virtual"),
    ("HH", "home health"),
]

PARTICIPANT_TYPE_CODES = [
    ("ATND", "attender"),
    ("PPRF", "primary performer"),
    ("SPRF", "secondary performer"),
    ("PART", "Participation"),
    ("translator", "Translator"),
    ("emergency", "Emergency"),
]

EPISODE_OF_CARE_TYPE_CODES = [
    ("inp", "Inpatient"),
    ("amb", "Ambulatory"),
    ("prenc", "Pre-admission"),
    ("hacc", "Home and Community Care"),
]

BED_STATUS_CODES = [
    ("O", "Occupied"),
    ("U", "Unoccupied"),
    ("C", "Closed"),
    ("H", "Housekeeping"),
    ("I", "Isolated"),
    ("K", "Contaminated"),
    ("L", "Blocked"),
]

LOINC_COMPOSITION_CODES = [
    ("11488-4", "Consult note"),
    ("10154-3", "Chief complaint Narrative"),
    ("10164-2", "History of Present illness Narrative"),
    ("29545-1", "Physical findings Narrative"),
    ("51848-0", "Evaluation note"),
    ("18776-5", "Plan of care note"),
]

DOC_CLASS_CODES = [
    ("clinical-note", "Clinical Note"),
]


def build_bundle() -> dict:
    act_code = {
        "resourceType": "CodeSystem",
        "id": "v3-ActCode",
        "url": V3_ACT_CODE,
        "version": "3.0.0",
        "name": "ActCode",
        "title": "ActCode",
        "status": "active",
        "content": "complete",
        "concept": [{"code": code, "display": display} for code, display in ENCOUNTER_CLASS_CODES],
    }

    participation_type = {
        "resourceType": "CodeSystem",
        "id": "v3-ParticipationType",
        "url": V3_PARTICIPATION,
        "version": "3.0.0",
        "name": "ParticipationType",
        "title": "ParticipationType",
        "status": "active",
        "content": "complete",
        "concept": [{"code": code, "display": display} for code, display in PARTICIPANT_TYPE_CODES],
    }

    episode_of_care_type = {
        "resourceType": "CodeSystem",
        "id": "episodeofcare-type",
        "url": EPISODE_OF_CARE_TYPE,
        "version": "4.0.1",
        "name": "EpisodeOfCareType",
        "title": "Episode of care type",
        "status": "active",
        "content": "complete",
        "concept": [
            {"code": code, "display": display} for code, display in EPISODE_OF_CARE_TYPE_CODES
        ],
    }

    v2_bed_status = {
        "resourceType": "CodeSystem",
        "id": "v2-0116",
        "url": V2_BED_STATUS,
        "version": "2.9",
        "name": "BedStatus",
        "title": "Bed Status",
        "status": "active",
        "content": "complete",
        "concept": [{"code": code, "display": display} for code, display in BED_STATUS_CODES],
    }

    loinc = {
        "resourceType": "CodeSystem",
        "id": "loinc-consult-minimal",
        "url": LOINC,
        "version": "2.77",
        "name": "LOINCConsultMinimal",
        "title": "LOINC (consultation note subset)",
        "status": "active",
        "content": "complete",
        "concept": [
            {"code": code, "display": display} for code, display in LOINC_COMPOSITION_CODES
        ],
    }

    document_class = {
        "resourceType": "CodeSystem",
        "id": "document-classcodes",
        "url": DOC_CLASS,
        "version": "4.0.1",
        "name": "DocumentClassValueSet",
        "title": "Document Class Value Set",
        "status": "active",
        "content": "complete",
        "concept": [{"code": code, "display": display} for code, display in DOC_CLASS_CODES],
    }

    atrius_encounter_class_vs = {
        "resourceType": "ValueSet",
        "id": "atrius-in-encounter-class",
        "url": ATRIUS_ENCOUNTER_CLASS_VS,
        "version": "0.1.0",
        "name": "AtriusInEncounterClass",
        "title": "Atrius India Core Encounter Class",
        "status": "active",
        "compose": {"include": [{"system": V3_ACT_CODE}]},
    }

    encounter_participant_type_vs = {
        "resourceType": "ValueSet",
        "id": "encounter-participant-type",
        "url": HL7_ENCOUNTER_PARTICIPANT_TYPE_VS,
        "version": "4.0.1",
        "name": "EncounterParticipantType",
        "title": "Encounter Participant Type",
        "status": "active",
        "compose": {
            "include": [
                {
                    "system": V3_PARTICIPATION,
                    "concept": [{"code": code} for code, _ in PARTICIPANT_TYPE_CODES],
                }
            ]
        },
    }

    return {
        "resourceType": "Bundle",
        "type": "collection",
        "entry": [
            {"resource": act_code},
            {"resource": participation_type},
            {"resource": episode_of_care_type},
            {"resource": v2_bed_status},
            {"resource": loinc},
            {"resource": document_class},
            {"resource": atrius_encounter_class_vs},
            {"resource": encounter_participant_type_vs},
        ],
    }


def post_import(hts_url: str, bundle: dict) -> dict:
    url = hts_url.rstrip("/") + "/import"
    data = json.dumps(bundle).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=data,
        headers={
            "Content-Type": "application/fhir+json",
            "Accept": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        print(f"HTS import failed: HTTP {exc.code}\n{body}", file=sys.stderr)
        raise SystemExit(1) from exc


def main() -> None:
    parser = argparse.ArgumentParser(description="Seed Atrius HIS terminology into HTS")
    parser.add_argument("--hts-url", default="http://127.0.0.1:9091")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    bundle = build_bundle()
    if args.dry_run:
        print(json.dumps(bundle, indent=2))
        return

    response = post_import(args.hts_url, bundle)
    print(json.dumps(response, indent=2))
    if response.get("errors"):
        print("Import completed with errors:", response["errors"], file=sys.stderr)
        raise SystemExit(1)
    print("Atrius terminology seed complete.", file=sys.stderr)


if __name__ == "__main__":
    main()
