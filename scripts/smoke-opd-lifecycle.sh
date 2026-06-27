#!/usr/bin/env bash
# OPD lifecycle smoke: register → book → start-visit → note → lab order + task → LIS result → finish-visit.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "${ROOT}/scripts/smoke-common.sh"
HIS_URL="${HIS_URL:-http://127.0.0.1:8096}"
SCHEDULE_ID="${SCHEDULE_ID:-opd-patel-schedule}"
PRACTITIONER_ID="${PRACTITIONER_ID:-dr-patel}"
LOCATION_ID="${LOCATION_ID:-atrius-demo-campus}"
START_DATE="${START_DATE:-$(date +%Y-%m-%d)}"

if ! curl -sf "${HIS_URL}/health" >/dev/null; then
  echo "his-server not reachable at ${HIS_URL}" >&2
  exit 1
fi

echo "== Register patient =="
REGISTER=$(curl -sf -X POST "${HIS_URL}/api/v1/patients" \
  -H "Content-Type: application/json" \
  -d "{
        \"family_name\": \"OpdLife${RANDOM}\",
        \"given_names\": [\"Cycle\"],
        \"gender\": \"female\",
        \"birth_date\": \"1991-02-14\"
      }")
PATIENT_ID=$(echo "${REGISTER}" | python3 -c "import sys,json; print(json.load(sys.stdin)['patient_id'])")

echo ""
echo "== Book + start visit =="
SLOTS=$(curl -sf "${HIS_URL}/api/v1/slots?schedule_id=${SCHEDULE_ID}&start=${START_DATE}")
SLOT_ID=$(echo "${SLOTS}" | python3 -c "import sys,json; print(json.load(sys.stdin)['slots'][0]['slot_id'])")
BOOK=$(curl -sf -X POST "${HIS_URL}/api/v1/appointments" \
  -H "Content-Type: application/json" \
  -d "{
        \"patient_id\": \"${PATIENT_ID}\",
        \"slot_id\": \"${SLOT_ID}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"location_id\": \"${LOCATION_ID}\"
      }")
APPT_ID=$(echo "${BOOK}" | python3 -c "import sys,json; print(json.load(sys.stdin)['appointment_id'])")
START=$(curl -sf -X POST "${HIS_URL}/api/v1/encounters/start-visit" \
  -H "Content-Type: application/json" \
  -d "{\"appointment_id\": \"${APPT_ID}\", \"reason\": \"OPD lifecycle smoke\"}")
ENC_ID=$(echo "${START}" | python3 -c "import sys,json; print(json.load(sys.stdin)['encounter_id'])")
echo "encounter_id=${ENC_ID}"

echo ""
echo "== Consultation note =="
NOTE=$(curl -sf -X POST "${HIS_URL}/api/v1/consultation-notes" \
  -H "Content-Type: application/json" \
  -d "{
        \"encounter_id\": \"${ENC_ID}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"sections\": {
          \"chief_complaint\": \"Fatigue\",
          \"assessment\": \"Anemia workup\",
          \"plan\": \"CBC\"
        }
      }")
COMP_ID=$(echo "${NOTE}" | python3 -c "import sys,json; print(json.load(sys.stdin)['composition_id'])")
echo "composition_id=${COMP_ID}"

echo ""
echo "== Place lab order (with Task) =="
ORDER=$(curl -sf -X POST "${HIS_URL}/api/v1/lab-orders" \
  -H "Content-Type: application/json" \
  -d "{
        \"encounter_id\": \"${ENC_ID}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"loinc_code\": \"58410-2\"
      }")
ORDER_ID=$(echo "${ORDER}" | python3 -c "import sys,json; print(json.load(sys.stdin)['order_id'])")
TASK_ID=$(echo "${ORDER}" | python3 -c "import sys,json; print(json.load(sys.stdin)['task_id'])")
echo "order_id=${ORDER_ID} task_id=${TASK_ID}"

TASKS=$(curl -sf "${HIS_URL}/api/v1/encounters/${ENC_ID}/lab-tasks")
[[ "$(echo "${TASKS}" | python3 -c "import sys,json; print(json.load(sys.stdin)['count'])")" -ge 1 ]] || exit 1

echo ""
echo "== LIS stub result =="
RESULT=$(curl -sf -X POST "${HIS_URL}/api/v1/lab-orders/${ORDER_ID}/result" \
  -H "Content-Type: application/json" \
  -d '{"value": "4.5", "unit": "10*3/uL"}')
echo "${RESULT}" | python3 -c "import sys,json; d=json.load(sys.stdin); print('report', d['report_id'], 'task', d['task_id'])"

RESULTS=$(curl -sf "${HIS_URL}/api/v1/encounters/${ENC_ID}/lab-results")
[[ "$(echo "${RESULTS}" | python3 -c "import sys,json; print(json.load(sys.stdin)['count'])")" -ge 1 ]] || exit 1

echo ""
echo "== Finish visit =="
FINISH=$(curl -sf -X POST "${HIS_URL}/api/v1/encounters/finish-visit" \
  -H "Content-Type: application/json" \
  -d "{\"encounter_id\": \"${ENC_ID}\"}")
echo "${FINISH}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['encounter']['status'], d['appointment']['status'])"
[[ "$(echo "${FINISH}" | python3 -c "import sys,json; print(json.load(sys.stdin)['encounter']['status'])")" == "finished" ]] || exit 1
[[ "$(echo "${FINISH}" | python3 -c "import sys,json; print(json.load(sys.stdin)['appointment']['status'])")" == "fulfilled" ]] || exit 1

echo ""
echo "== Block new lab order after checkout =="
HTTP=$(curl -s -o /tmp/opd-life-block.json -w "%{http_code}" -X POST "${HIS_URL}/api/v1/lab-orders" \
  -H "Content-Type: application/json" \
  -d "{
        \"encounter_id\": \"${ENC_ID}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"loinc_code\": \"58410-2\"
      }")
echo "post-checkout lab order HTTP ${HTTP}"
[[ "${HTTP}" == "409" ]] || { cat /tmp/opd-life-block.json; exit 1; }

echo ""
echo "OPD lifecycle smoke passed."
