#!/usr/bin/env bash
# Smoke test: register → book appointment → start OPD visit → read Encounter.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "${ROOT}/scripts/smoke-common.sh"
HIS_URL="${HIS_URL:-http://127.0.0.1:8096}"
HTS_URL="${HTS_URL:-http://127.0.0.1:9091}"
HFS_URL="${HFS_URL:-http://127.0.0.1:8082}"
SCHEDULE_ID="${SCHEDULE_ID:-opd-patel-schedule}"
PRACTITIONER_ID="${PRACTITIONER_ID:-dr-patel}"
LOCATION_ID="${LOCATION_ID:-atrius-demo-campus}"
START_DATE="${START_DATE:-$(date +%Y-%m-%d)}"

if [[ -z "${HIS_FHIR_BEARER_TOKEN:-}" ]]; then
  export HIS_FHIR_BEARER_TOKEN="$("${ROOT}/deploy/keycloak/get-token.sh" his-backend-client)"
fi

AUTH=(-H "Authorization: Bearer ${HIS_FHIR_BEARER_TOKEN}")

if ! curl -sf "${HIS_URL}/health" >/dev/null; then
  echo "his-server not reachable at ${HIS_URL}" >&2
  exit 1
fi

echo "== Register patient =="
REGISTER=$(curl -sf -X POST "${HIS_URL}/api/v1/patients" \
  -H "Content-Type: application/json" \
  -d "{
        \"family_name\": \"VisitSmoke${RANDOM}\",
        \"given_names\": [\"OPD\"],
        \"gender\": \"female\",
        \"birth_date\": \"1990-04-12\",
        \"telecom\": [{\"system\": \"phone\", \"value\": \"+91-9000000003\"}],
        \"address\": [{
          \"use\": \"home\",
          \"line\": [\"3 OPD Lane\"],
          \"city\": \"Bengaluru\",
          \"state\": \"KA\",
          \"postal_code\": \"560001\",
          \"country\": \"IN\"
        }]
      }")
PATIENT_ID=$(echo "${REGISTER}" | python3 -c "import sys,json; print(json.load(sys.stdin)['patient_id'])")
echo "patient_id=${PATIENT_ID}"

echo ""
echo "== Find free slot =="
SLOTS=$(curl -sf "${HIS_URL}/api/v1/slots?schedule_id=${SCHEDULE_ID}&start=${START_DATE}")
SLOT_COUNT=$(echo "${SLOTS}" | python3 -c "import sys,json; print(json.load(sys.stdin).get('count',0))")
if [[ "${SLOT_COUNT}" -lt 1 ]]; then
  diagnose_no_free_slots "${SCHEDULE_ID}" "${START_DATE}"
fi
SLOT_ID=$(echo "${SLOTS}" | python3 -c "import sys,json; print(json.load(sys.stdin)['slots'][0]['slot_id'])")
echo "slot_id=${SLOT_ID}"

echo ""
echo "== Book appointment =="
BOOK=$(curl -sf -X POST "${HIS_URL}/api/v1/appointments" \
  -H "Content-Type: application/json" \
  -d "{
        \"patient_id\": \"${PATIENT_ID}\",
        \"slot_id\": \"${SLOT_ID}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"location_id\": \"${LOCATION_ID}\",
        \"description\": \"OPD visit smoke\"
      }")
APPT_ID=$(echo "${BOOK}" | python3 -c "import sys,json; print(json.load(sys.stdin)['appointment_id'])")
echo "appointment_id=${APPT_ID}"

echo ""
echo "== Start visit =="
START=$(curl -sf -X POST "${HIS_URL}/api/v1/encounters/start-visit" \
  -H "Content-Type: application/json" \
  -d "{
        \"appointment_id\": \"${APPT_ID}\",
        \"reason\": \"General consultation\"
      }")
echo "${START}" | python3 -m json.tool
ENC_ID=$(echo "${START}" | python3 -c "import sys,json; print(json.load(sys.stdin)['encounter_id'])")

echo ""
echo "== Read encounter via his-server =="
curl -sf "${HIS_URL}/api/v1/encounters/${ENC_ID}" | python3 -c "import sys,json; e=json.load(sys.stdin); print(e['resourceType'], e['id'], e['class']['code'], e['status']); print('appointment', e.get('appointment')); print('profile', e.get('meta',{}).get('profile'))"

echo ""
echo "== Ensure HTS terminology for Encounter \$validate =="
python3 "${ROOT}/scripts/seed-atrius-terminology.py" --hts-url "${HTS_URL}"

echo ""
echo "== \$validate Encounter on HFS =="
VALIDATE=$(curl -sf -X POST "${HFS_URL}/Encounter/\$validate" \
  "${AUTH[@]}" \
  -H "Content-Type: application/fhir+json" \
  -H "Accept: application/fhir+json" \
  -d "$(curl -sf "${HIS_URL}/api/v1/encounters/${ENC_ID}")")
ISSUE_COUNT=$(echo "${VALIDATE}" | python3 -c "import sys,json; o=json.load(sys.stdin); print(o.get('issue',[]).__len__())")
echo "OperationOutcome issues: ${ISSUE_COUNT}"
echo "${VALIDATE}" | python3 -m json.tool | head -40
ERROR_COUNT=$(echo "${VALIDATE}" | python3 -c "import sys,json; o=json.load(sys.stdin); print(sum(1 for i in o.get('issue',[]) if i.get('severity')=='error'))")
[[ "${ERROR_COUNT}" == "0" ]] || { echo "Encounter \$validate returned errors" >&2; exit 1; }

echo ""
echo "== Double start-visit (expect HTTP 409) =="
set +e
HTTP=$(curl -s -o /tmp/his-start-visit-dup.json -w "%{http_code}" -X POST "${HIS_URL}/api/v1/encounters/start-visit" \
  -H "Content-Type: application/json" \
  -d "{\"appointment_id\": \"${APPT_ID}\"}")
set -e
echo "HTTP ${HTTP}"
python3 -m json.tool /tmp/his-start-visit-dup.json
[[ "${HTTP}" == "409" ]] || { echo "Expected 409 for duplicate start-visit" >&2; exit 1; }

echo ""
echo "Start-visit smoke test complete."
