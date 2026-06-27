#!/usr/bin/env python3
"""Seed hospital foundation FHIR resources into Clinical HFS (Phase 0.4 + Phase 2 slots).

Posts a transaction Bundle with Organization, Location hierarchy, Practitioners,
HealthcareService, Schedule, and OPD Slot availability. Requires Clinical HFS running.

Usage:
  export HIS_FHIR_BEARER_TOKEN=$(./deploy/keycloak/get-token.sh his-backend-client)
  python3 scripts/seed-hospital-foundation.py \\
    --base-url http://127.0.0.1:8082 \\
    --tenant atrius-hospital \\
    --token "$HIS_FHIR_BEARER_TOKEN"

Tenant must match his-server (`HIS_DEFAULT_TENANT`, default `atrius-hospital`).
"""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request
from datetime import date, datetime, timedelta, timezone

ATRIUS_IG = "https://atrius.in/fhir/r4/atrius-in"
ATRIUS_SD = f"{ATRIUS_IG}/StructureDefinition"
ATRIUS_ORG = f"{ATRIUS_IG}/Organization/atrius-demo-hospital"
ATRIUS_LOCATION = f"{ATRIUS_IG}/Location/atrius-demo-campus"


ATRIUS_LOCATION = f"{ATRIUS_IG}/Location/atrius-demo-campus"
BOOKING_SCHEDULES = [
    {
        "id": "opd-patel-schedule",
        "practitioner_id": "dr-patel",
        "practitioner_name": {"family": "Patel", "given": ["Anita"], "prefix": ["Dr."]},
        "rrule": "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR;BYHOUR=9;BYMINUTE=0;BYSECOND=0",
    },
    {
        "id": "opd-sharma-schedule",
        "practitioner_id": "dr-sharma",
        "practitioner_name": {"family": "Sharma", "given": ["Raj"], "prefix": ["Dr."]},
        "rrule": "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR;BYHOUR=14;BYMINUTE=0;BYSECOND=0",
    },
]
WARD_ID = "ward-med-a"
IST = timezone(timedelta(hours=5, minutes=30))
V2_BED_STATUS = "http://terminology.hl7.org/CodeSystem/v2-0116"
LOC_PHYSICAL = "http://terminology.hl7.org/CodeSystem/location-physical-type"


def iso_instant(dt: datetime) -> str:
    return dt.astimezone(IST).isoformat(timespec="seconds")


def schedule_recurrence_extension(rrule: str) -> dict:
    return {
        "url": f"{ATRIUS_SD}/atrius-in-schedule-recurrence",
        "extension": [
            {"url": "RRULE", "valueString": rrule},
            {"url": "TZID", "valueString": "Asia/Kolkata"},
        ],
    }


def schedule_entry(schedule_id: str, practitioner_id: str, rrule: str) -> dict:
    return {
        "fullUrl": f"urn:uuid:{schedule_id}",
        "resource": {
            "resourceType": "Schedule",
            "id": schedule_id,
            "meta": {"profile": [f"{ATRIUS_SD}/atrius-in-schedule"]},
            "active": True,
            "serviceCategory": [
                {
                    "coding": [
                        {
                            "system": "http://terminology.hl7.org/CodeSystem/service-category",
                            "code": "17",
                            "display": "General Practice",
                        }
                    ]
                }
            ],
            "serviceType": [
                {
                    "coding": [
                        {
                            "system": "http://snomed.info/sct",
                            "code": "394802001",
                            "display": "General medicine",
                        }
                    ]
                }
            ],
            "actor": [
                {"reference": f"Practitioner/{practitioner_id}"},
                {"reference": "Location/atrius-demo-campus"},
            ],
            "planningHorizon": {
                "start": iso_instant(datetime.now(tz=IST).replace(hour=0, minute=0, second=0)),
                "end": iso_instant(
                    datetime.now(tz=IST).replace(hour=0, minute=0, second=0)
                    + timedelta(days=14)
                ),
            },
            "extension": [schedule_recurrence_extension(rrule)],
        },
        "request": {"method": "PUT", "url": f"Schedule/{schedule_id}"},
    }


def practitioner_entry(practitioner_id: str, name: dict, gender: str) -> dict:
    return {
        "fullUrl": f"urn:uuid:practitioner-{practitioner_id}",
        "resource": {
            "resourceType": "Practitioner",
            "id": practitioner_id,
            "meta": {"profile": [f"{ATRIUS_SD}/atrius-in-practitioner"]},
            "active": True,
            "name": [name],
            "gender": gender,
        },
        "request": {"method": "PUT", "url": f"Practitioner/{practitioner_id}"},
    }


def ward_and_bed_entries() -> list[dict]:
    ward = {
        "fullUrl": f"urn:uuid:{WARD_ID}",
        "resource": {
            "resourceType": "Location",
            "id": WARD_ID,
            "meta": {"profile": [f"{ATRIUS_SD}/atrius-in-location"]},
            "status": "active",
            "name": "Medical Ward A",
            "mode": "instance",
            "physicalType": {
                "coding": [{"system": LOC_PHYSICAL, "code": "wa", "display": "Ward"}]
            },
            "partOf": {"reference": "Location/atrius-demo-campus"},
            "managingOrganization": {"reference": "Organization/atrius-demo-hospital"},
        },
        "request": {"method": "PUT", "url": f"Location/{WARD_ID}"},
    }

    beds = []
    for n in (1, 2):
        bed_id = f"bed-med-a-{n:02d}"
        beds.append(
            {
                "fullUrl": f"urn:uuid:{bed_id}",
                "resource": {
                    "resourceType": "Location",
                    "id": bed_id,
                    "meta": {"profile": [f"{ATRIUS_SD}/atrius-in-location-bed"]},
                    "status": "active",
                    "name": f"Medical Ward A — Bed {n}",
                    "mode": "instance",
                    "physicalType": {
                        "coding": [{"system": LOC_PHYSICAL, "code": "bd", "display": "Bed"}]
                    },
                    "partOf": {"reference": f"Location/{WARD_ID}"},
                    "managingOrganization": {"reference": "Organization/atrius-demo-hospital"},
                    "operationalStatus": {
                        "system": V2_BED_STATUS,
                        "code": "U",
                        "display": "Unoccupied",
                    },
                },
                "request": {"method": "PUT", "url": f"Location/{bed_id}"},
            }
        )
    return [ward, *beds]


def build_bundle() -> dict:
    base_entries = [
            {
                "fullUrl": ATRIUS_ORG,
                "resource": {
                    "resourceType": "Organization",
                    "id": "atrius-demo-hospital",
                    "meta": {
                        "profile": [f"{ATRIUS_SD}/atrius-in-organization"]
                    },
                    "active": True,
                    "name": "Atrius Demo Hospital",
                    "type": [
                        {
                            "coding": [
                                {
                                    "system": "http://terminology.hl7.org/CodeSystem/organization-type",
                                    "code": "prov",
                                    "display": "Healthcare Provider",
                                }
                            ]
                        }
                    ],
                },
                "request": {"method": "PUT", "url": "Organization/atrius-demo-hospital"},
            },
            {
                "fullUrl": ATRIUS_LOCATION,
                "resource": {
                    "resourceType": "Location",
                    "id": "atrius-demo-campus",
                    "meta": {
                        "profile": [f"{ATRIUS_SD}/atrius-in-location"]
                    },
                    "status": "active",
                    "name": "Main Campus",
                    "mode": "instance",
                    "type": [
                        {
                            "coding": [
                                {
                                    "system": "http://terminology.hl7.org/CodeSystem/v3-RoleCode",
                                    "code": "HOSP",
                                    "display": "Hospital",
                                }
                            ]
                        }
                    ],
                    "physicalType": {
                        "coding": [
                            {
                                "system": "http://terminology.hl7.org/CodeSystem/location-physical-type",
                                "code": "si",
                                "display": "Site",
                            }
                        ]
                    },
                    "managingOrganization": {"reference": "Organization/atrius-demo-hospital"},
                },
                "request": {"method": "PUT", "url": "Location/atrius-demo-campus"},
            },
        ]
    for schedule in BOOKING_SCHEDULES:
        gender = "female" if schedule["practitioner_id"] == "dr-patel" else "male"
        base_entries.append(
            practitioner_entry(schedule["practitioner_id"], schedule["practitioner_name"], gender)
        )
    base_entries.append(
            {
                "fullUrl": "urn:uuid:healthcare-service-opd",
                "resource": {
                    "resourceType": "HealthcareService",
                    "id": "opd-general",
                    "meta": {
                        "profile": [f"{ATRIUS_SD}/atrius-in-healthcareservice"]
                    },
                    "active": True,
                    "providedBy": {"reference": "Organization/atrius-demo-hospital"},
                    "category": [
                        {
                            "coding": [
                                {
                                    "system": "http://terminology.hl7.org/CodeSystem/service-category",
                                    "code": "17",
                                    "display": "General Practice",
                                }
                            ]
                        }
                    ],
                    "name": "General OPD",
                    "location": [{"reference": "Location/atrius-demo-campus"}],
                },
                "request": {"method": "PUT", "url": "HealthcareService/opd-general"},
            },
        )
    base_entries.extend(ward_and_bed_entries())
    for schedule in BOOKING_SCHEDULES:
        base_entries.append(
            schedule_entry(schedule["id"], schedule["practitioner_id"], schedule["rrule"])
        )

    return {
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": base_entries,
    }


def post_bundle(base_url: str, tenant: str, token: str | None, bundle: dict) -> dict:
    url = base_url.rstrip("/") + "/"
    data = json.dumps(bundle).encode("utf-8")
    headers = {
        "Content-Type": "application/fhir+json",
        "Accept": "application/fhir+json",
        "X-Tenant-ID": tenant,
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"

    req = urllib.request.Request(url, data=data, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        print(f"Transaction failed: HTTP {exc.code}\n{body}", file=sys.stderr)
        raise SystemExit(1) from exc


def expand_schedule_slots(his_url: str, schedule_id: str, from_date: str, to_date: str) -> dict:
    url = (
        f"{his_url.rstrip('/')}/api/v1/schedules/{schedule_id}/expand-slots"
        f"?from={from_date}&to={to_date}"
    )
    req = urllib.request.Request(url, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        print(f"Slot expansion failed: HTTP {exc.code}\n{body}", file=sys.stderr)
        raise SystemExit(1) from exc


def main() -> None:
    parser = argparse.ArgumentParser(description="Seed hospital foundation data")
    parser.add_argument("--base-url", default="http://127.0.0.1:8082")
    parser.add_argument("--his-url", default="http://127.0.0.1:8096")
    parser.add_argument("--tenant", default="atrius-hospital")
    parser.add_argument("--token", default=None)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--skip-expand", action="store_true")
    args = parser.parse_args()

    bundle = build_bundle()
    if args.dry_run:
        print(json.dumps(bundle, indent=2))
        return

    response = post_bundle(args.base_url, args.tenant, args.token, bundle)
    print(json.dumps(response, indent=2))

    if not args.skip_expand:
        today = date.today()
        end = today + timedelta(days=14)
        for schedule in BOOKING_SCHEDULES:
            schedule_id = schedule["id"]
            expanded = expand_schedule_slots(
                args.his_url,
                schedule_id,
                today.isoformat(),
                end.isoformat(),
            )
            print(json.dumps(expanded, indent=2), file=sys.stderr)
            print(
                f"Expanded {expanded.get('slots_created', 0)} slots for {schedule_id}.",
                file=sys.stderr,
            )

    print("Seed complete.", file=sys.stderr)


if __name__ == "__main__":
    main()
