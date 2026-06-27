#!/usr/bin/env bash
# Smoke test for Phase 2/3.5 scheduling (RRULE expansion, book/cancel/reschedule, $validate).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "${ROOT}/scripts/smoke-common.sh"
HIS_URL="${HIS_URL:-http://127.0.0.1:8096}"
SCHEDULE_ID="${SCHEDULE_ID:-opd-patel-schedule}"
PRACTITIONER_ID="${PRACTITIONER_ID:-dr-patel}"
LOCATION_ID="${LOCATION_ID:-atrius-demo-campus}"
START_DATE="${START_DATE:-$(date +%Y-%m-%d)}"
END_DATE="${END_DATE:-$(python3 -c "from datetime import date, timedelta; print((date.fromisoformat('${START_DATE}') + timedelta(days=14)).isoformat())")}"

if [[ -z "${HIS_FHIR_BEARER_TOKEN:-}" ]]; then
  export HIS_FHIR_BEARER_TOKEN="$("${ROOT}/deploy/keycloak/get-token.sh" his-backend-client)"
fi

echo "== Ensure HTS terminology =="
python3 "${ROOT}/scripts/seed-atrius-terminology.py" --hts-url "${HTS_URL:-http://127.0.0.1:9091}"

echo "== Expand schedule slots (${START_DATE} .. ${END_DATE}) =="
EXPAND=$(curl -sf -X POST \
  "${HIS_URL}/api/v1/schedules/${SCHEDULE_ID}/expand-slots?from=${START_DATE}&to=${END_DATE}")
echo "${EXPAND}" | python3 -m json.tool

echo ""
echo "== \$validate Schedule on HFS =="
SCHEDULE_JSON=$(hfs_curl "${HFS_URL}/Schedule/${SCHEDULE_ID}")
validate_fhir_resource "Schedule" "${SCHEDULE_JSON}"

echo ""
echo "== Register patient for booking =="
REGISTER=$(curl -sf -X POST "${HIS_URL}/api/v1/patients" \
  -H "Content-Type: application/json" \
  -d "{
        \"family_name\": \"ScheduleSmoke${RANDOM}\",
        \"given_names\": [\"Phase2\"],
        \"gender\": \"male\",
        \"birth_date\": \"1985-01-20\",
        \"telecom\": [{\"system\": \"phone\", \"value\": \"+91-9000000001\"}],
        \"address\": [{
          \"use\": \"home\",
          \"line\": [\"1 Clinic Road\"],
          \"city\": \"Bengaluru\",
          \"state\": \"KA\",
          \"postal_code\": \"560001\",
          \"country\": \"IN\"
        }]
      }")
PATIENT_ID=$(echo "${REGISTER}" | python3 -c "import sys,json; print(json.load(sys.stdin)['patient_id'])")
echo "patient_id=${PATIENT_ID}"

echo ""
echo "== Find free slots =="
SLOTS=$(curl -sf "${HIS_URL}/api/v1/slots?schedule_id=${SCHEDULE_ID}&start=${START_DATE}")
echo "${SLOTS}" | python3 -m json.tool | head -40
SLOT_COUNT=$(echo "${SLOTS}" | python3 -c "import sys,json; print(json.load(sys.stdin).get('count',0))")
if [[ "${SLOT_COUNT}" -lt 1 ]]; then
  diagnose_no_free_slots "${SCHEDULE_ID}" "${START_DATE}"
fi
SLOT_ID=$(echo "${SLOTS}" | python3 -c "import sys,json; print(json.load(sys.stdin)['slots'][0]['slot_id'])")
echo "Using slot_id=${SLOT_ID}"

echo ""
echo "== \$validate Slot on HFS =="
SLOT_JSON=$(hfs_curl "${HFS_URL}/Slot/${SLOT_ID}")
validate_fhir_resource "Slot" "${SLOT_JSON}"

echo ""
echo "== Book appointment =="
BOOK=$(curl -sf -X POST "${HIS_URL}/api/v1/appointments" \
  -H "Content-Type: application/json" \
  -d "{
        \"patient_id\": \"${PATIENT_ID}\",
        \"slot_id\": \"${SLOT_ID}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"location_id\": \"${LOCATION_ID}\",
        \"description\": \"OPD general visit\"
      }")
echo "${BOOK}" | python3 -m json.tool
APPT_ID=$(echo "${BOOK}" | python3 -c "import sys,json; print(json.load(sys.stdin)['appointment_id'])")

echo ""
echo "== \$validate Appointment on HFS =="
APPT_JSON=$(hfs_curl "${HFS_URL}/Appointment/${APPT_ID}")
validate_fhir_resource "Appointment" "${APPT_JSON}"

echo ""
echo "== Double-book same slot (expect HTTP 409) =="
set +e
HTTP=$(curl -s -o /tmp/his-slot-dup.json -w "%{http_code}" -X POST "${HIS_URL}/api/v1/appointments" \
  -H "Content-Type: application/json" \
  -d "{
        \"patient_id\": \"${PATIENT_ID}\",
        \"slot_id\": \"${SLOT_ID}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\"
      }")
set -e
echo "HTTP ${HTTP}"
python3 -m json.tool /tmp/his-slot-dup.json
[[ "${HTTP}" == "409" ]] || { echo "Expected 409 for double booking" >&2; exit 1; }

echo ""
echo "== Reschedule =="
ALT_SLOT_ID=$(echo "${SLOTS}" | python3 -c "import sys,json; s=json.load(sys.stdin); ids=[x['slot_id'] for x in s['slots']]; print(ids[1] if len(ids)>1 else ids[0])")
RESCHED=$(curl -sf -X POST "${HIS_URL}/api/v1/appointments/${APPT_ID}/reschedule" \
  -H "Content-Type: application/json" \
  -d "{\"new_slot_id\": \"${ALT_SLOT_ID}\"}")
echo "${RESCHED}" | python3 -c "import sys,json; a=json.load(sys.stdin); print(a['resourceType'], a['id'], a['status'], a.get('slot'))"

echo ""
echo "== Cancel appointment =="
CANCEL=$(curl -sf -X POST "${HIS_URL}/api/v1/appointments/${APPT_ID}/cancel" \
  -H "Content-Type: application/json" \
  -d '{"reason": "Patient requested cancellation"}')
echo "${CANCEL}" | python3 -c "import sys,json; a=json.load(sys.stdin); print(a['status'], a.get('cancelationReason',{}).get('text'))"

echo ""
echo "Scheduling smoke test complete."
