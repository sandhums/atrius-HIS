#!/usr/bin/env bash
# Smoke test for Phase 1 patient registration.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HIS_URL="${HIS_URL:-http://127.0.0.1:8096}"
HFS_URL="${HFS_URL:-http://127.0.0.1:8082}"
TENANT="${HIS_DEFAULT_TENANT:-atrius-hospital}"
RUN_ID="${RUN_ID:-$(date +%s)}"

if [[ -z "${HIS_FHIR_BEARER_TOKEN:-}" ]]; then
  export HIS_FHIR_BEARER_TOKEN="$("${ROOT}/deploy/keycloak/get-token.sh" his-backend-client)"
fi

FAMILY="SmokeTest${RUN_ID}"
BIRTH_DATE="1990-03-15"

PATIENT_PAYLOAD=$(cat <<EOF
{
  "family_name": "${FAMILY}",
  "given_names": ["Phase1"],
  "gender": "female",
  "birth_date": "${BIRTH_DATE}",
  "telecom": [{"system": "phone", "value": "+91-9876543210"}],
  "address": [{
    "use": "home",
    "line": ["12 MG Road"],
    "city": "Bengaluru",
    "state": "KA",
    "postal_code": "560001",
    "country": "IN"
  }],
  "birth_place": {
    "city": "Mysuru",
    "state": "KA",
    "country": "IN"
  }
}
EOF
)

DUPLICATE_CHECK_PAYLOAD=$(cat <<EOF
{
  "family_name": "${FAMILY}",
  "given_names": ["Phase1"],
  "gender": "female",
  "birth_date": "${BIRTH_DATE}"
}
EOF
)

echo "== Duplicate check before register (expect no match) =="
BEFORE=$(curl -sf -X POST "${HIS_URL}/api/v1/patients/check-duplicates" \
  -H "Content-Type: application/json" \
  -d "${DUPLICATE_CHECK_PAYLOAD}")
echo "${BEFORE}" | python3 -m json.tool
BEFORE_COUNT=$(echo "${BEFORE}" | python3 -c "import sys,json; print(json.load(sys.stdin)['count'])")
if [[ "${BEFORE_COUNT}" != "0" ]]; then
  echo "Expected 0 duplicates before first registration (got ${BEFORE_COUNT})" >&2
  exit 1
fi

echo ""
echo "== Register patient =="
REGISTER=$(curl -sf -X POST "${HIS_URL}/api/v1/patients" \
  -H "Content-Type: application/json" \
  -d "${PATIENT_PAYLOAD}")

echo "${REGISTER}" | python3 -m json.tool

PATIENT_ID=$(echo "${REGISTER}" | python3 -c "import sys,json; print(json.load(sys.stdin)['patient_id'])")
MRN=$(echo "${REGISTER}" | python3 -c "import sys,json; print(json.load(sys.stdin)['mrn'])")

echo ""
echo "Registered patient_id=${PATIENT_ID} mrn=${MRN}"

echo ""
echo "== Read via his-server =="
curl -sf "${HIS_URL}/api/v1/patients/${PATIENT_ID}" | python3 -c "import sys,json; p=json.load(sys.stdin); print(p['resourceType'], p['id'], p['identifier'][0]['value'])"

echo ""
echo "== Duplicate check after register (expect match) =="
AFTER=$(curl -sf -X POST "${HIS_URL}/api/v1/patients/check-duplicates" \
  -H "Content-Type: application/json" \
  -d "${DUPLICATE_CHECK_PAYLOAD}")
echo "${AFTER}" | python3 -m json.tool
AFTER_COUNT=$(echo "${AFTER}" | python3 -c "import sys,json; print(json.load(sys.stdin)['count'])")
if [[ "${AFTER_COUNT}" -lt 1 ]]; then
  echo "Expected at least 1 duplicate after registration (got ${AFTER_COUNT})" >&2
  exit 1
fi

echo ""
echo "== Duplicate register (expect HTTP 409) =="
set +e
HTTP=$(curl -s -o /tmp/his-dup.json -w "%{http_code}" -X POST "${HIS_URL}/api/v1/patients" \
  -H "Content-Type: application/json" \
  -d "${DUPLICATE_CHECK_PAYLOAD}")
set -e
echo "HTTP ${HTTP}"
python3 -m json.tool /tmp/his-dup.json

if [[ "${HTTP}" != "409" ]]; then
  echo "Expected 409 Conflict for duplicate registration" >&2
  exit 1
fi

echo ""
echo "Registration smoke test complete."
