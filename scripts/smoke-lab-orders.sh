#!/usr/bin/env bash
# Phase 5b smoke: book → start-visit → place lab order → $validate → list → revoke.
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
LOINC_CODE="${LOINC_CODE:-58410-2}"

ensure_hfs_token "${ROOT}"

if ! curl -sf "${HIS_URL}/health" >/dev/null; then
  echo "his-server not reachable at ${HIS_URL}" >&2
  exit 1
fi

echo "== Lab catalog =="
CATALOG=$(curl -sf "${HIS_URL}/api/v1/lab-catalog")
echo "${CATALOG}" | python3 -c "import sys,json; d=json.load(sys.stdin); print('count', d['count']); print('codes', [t['loinc_code'] for t in d['tests']])"
[[ "$(echo "${CATALOG}" | python3 -c "import sys,json; print(json.load(sys.stdin)['count'])")" -ge 1 ]] || exit 1

echo ""
echo "== Register patient =="
REGISTER=$(curl -sf -X POST "${HIS_URL}/api/v1/patients" \
  -H "Content-Type: application/json" \
  -d "{
        \"family_name\": \"LabOrderSmoke${RANDOM}\",
        \"given_names\": [\"CBC\"],
        \"gender\": \"female\",
        \"birth_date\": \"1990-04-12\",
        \"telecom\": [{\"system\": \"phone\", \"value\": \"+91-9000000005\"}],
        \"address\": [{
          \"use\": \"home\",
          \"line\": [\"5 Lab Lane\"],
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
        \"description\": \"Lab order smoke\"
      }")
APPT_ID=$(echo "${BOOK}" | python3 -c "import sys,json; print(json.load(sys.stdin)['appointment_id'])")
echo "appointment_id=${APPT_ID}"

echo ""
echo "== Start visit =="
START=$(curl -sf -X POST "${HIS_URL}/api/v1/encounters/start-visit" \
  -H "Content-Type: application/json" \
  -d "{
        \"appointment_id\": \"${APPT_ID}\",
        \"reason\": \"Routine blood work\"
      }")
ENC_ID=$(echo "${START}" | python3 -c "import sys,json; print(json.load(sys.stdin)['encounter_id'])")
echo "encounter_id=${ENC_ID}"

echo ""
echo "== Place lab order (LOINC ${LOINC_CODE}) =="
ORDER=$(curl -sf -X POST "${HIS_URL}/api/v1/lab-orders" \
  -H "Content-Type: application/json" \
  -d "{
        \"encounter_id\": \"${ENC_ID}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"loinc_code\": \"${LOINC_CODE}\",
        \"note\": \"CBC for fatigue workup\"
      }")
echo "${ORDER}" | python3 -m json.tool | head -35
ORDER_ID=$(echo "${ORDER}" | python3 -c "import sys,json; print(json.load(sys.stdin)['order_id'])")
TASK_ID=$(echo "${ORDER}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('task_id',''))")
echo "order_id=${ORDER_ID}"
if [[ -n "${TASK_ID}" ]]; then echo "task_id=${TASK_ID}"; fi

echo ""
echo "== List lab orders by encounter =="
LIST=$(curl -sf "${HIS_URL}/api/v1/encounters/${ENC_ID}/lab-orders")
echo "${LIST}" | python3 -c "import sys,json; b=json.load(sys.stdin); print('count', b['count']); print('statuses', [o.get('status') for o in b.get('orders',[])])"
[[ "$(echo "${LIST}" | python3 -c "import sys,json; print(json.load(sys.stdin)['count'])")" -ge 1 ]] || exit 1

echo ""
echo "== Ensure HTS terminology =="
python3 "${ROOT}/scripts/seed-atrius-terminology.py" --hts-url "${HTS_URL}"

echo ""
echo "== \$validate ServiceRequest on HFS =="
SR_JSON=$(curl -sf "${HIS_URL}/api/v1/lab-orders/${ORDER_ID}")
VALIDATE=$(hfs_curl -X POST "${HFS_URL}/ServiceRequest/\$validate" \
  -H "Content-Type: application/fhir+json" \
  -H "Accept: application/fhir+json" \
  -d "${SR_JSON}")
ERROR_COUNT=$(echo "${VALIDATE}" | python3 -c "import sys,json; o=json.load(sys.stdin); print(sum(1 for i in o.get('issue',[]) if i.get('severity')=='error'))")
WARN_COUNT=$(echo "${VALIDATE}" | python3 -c "import sys,json; o=json.load(sys.stdin); print(sum(1 for i in o.get('issue',[]) if i.get('severity')=='warning'))")
echo "ServiceRequest \$validate errors=${ERROR_COUNT} warnings=${WARN_COUNT}"
[[ "${ERROR_COUNT}" == "0" ]] || { echo "${VALIDATE}" | python3 -m json.tool; exit 1; }

echo ""
echo "== Revoke lab order =="
REVOKE=$(curl -sf -X POST "${HIS_URL}/api/v1/lab-orders/${ORDER_ID}/revoke")
echo "${REVOKE}" | python3 -c "import sys,json; d=json.load(sys.stdin); print('status', d.get('status'))"
[[ "$(echo "${REVOKE}" | python3 -c "import sys,json; print(json.load(sys.stdin)['status'])")" == "revoked" ]] || exit 1

echo ""
echo "Phase 5b lab order smoke passed."
