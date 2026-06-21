#!/usr/bin/env bash
# Phase 5a smoke: book → start-visit → draft consult note → update → finalize → read back.
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

ensure_hfs_token "${ROOT}"

if ! curl -sf "${HIS_URL}/health" >/dev/null; then
  echo "his-server not reachable at ${HIS_URL}" >&2
  exit 1
fi

echo "== Register patient =="
REGISTER=$(curl -sf -X POST "${HIS_URL}/api/v1/patients" \
  -H "Content-Type: application/json" \
  -d "{
        \"family_name\": \"ConsultSmoke${RANDOM}\",
        \"given_names\": [\"Note\"],
        \"gender\": \"male\",
        \"birth_date\": \"1988-11-03\",
        \"telecom\": [{\"system\": \"phone\", \"value\": \"+91-9000000004\"}],
        \"address\": [{
          \"use\": \"home\",
          \"line\": [\"4 Clinic Street\"],
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
        \"description\": \"Consult note smoke\"
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
ENC_ID=$(echo "${START}" | python3 -c "import sys,json; print(json.load(sys.stdin)['encounter_id'])")
echo "encounter_id=${ENC_ID}"

echo ""
echo "== Create draft consultation note =="
CREATE=$(curl -sf -X POST "${HIS_URL}/api/v1/consultation-notes" \
  -H "Content-Type: application/json" \
  -d "{
        \"encounter_id\": \"${ENC_ID}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"sections\": {
          \"chief_complaint\": \"Intermittent headache for 3 days\",
          \"hpi\": \"Non-focal, worse in evenings, no trauma\",
          \"exam\": \"Alert, oriented, normal neurological exam\",
          \"assessment\": \"Tension-type headache\",
          \"plan\": \"Hydration, analgesia PRN, follow up in 1 week\"
        }
      }")
echo "${CREATE}" | python3 -m json.tool | head -45
COMP_ID=$(echo "${CREATE}" | python3 -c "import sys,json; print(json.load(sys.stdin)['composition_id'])")
echo "composition_id=${COMP_ID}"

echo ""
echo "== Update draft note =="
UPDATE=$(curl -sf -X PUT "${HIS_URL}/api/v1/consultation-notes/${COMP_ID}" \
  -H "Content-Type: application/json" \
  -d '{
        "sections": {
          "chief_complaint": "Intermittent headache for 3 days",
          "hpi": "Non-focal, worse in evenings, no trauma",
          "exam": "Alert, oriented, normal neurological exam",
          "assessment": "Tension-type headache — low concern for secondary cause",
          "plan": "Hydration, paracetamol PRN, follow up in 1 week"
        }
      }')
echo "${UPDATE}" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['status'], len(d['composition'].get('section',[])), 'sections')"

echo ""
echo "== Finalize note =="
FINAL=$(curl -sf -X POST "${HIS_URL}/api/v1/consultation-notes/${COMP_ID}/finalize" \
  -H "Content-Type: application/json" \
  -d "{\"practitioner_id\": \"${PRACTITIONER_ID}\"}")
echo "${FINAL}" | python3 -c "import sys,json; d=json.load(sys.stdin); c=d['composition']; print(d['status'], c.get('attester',[{}])[0].get('mode')); print('sections', len(c.get('section',[])))"

echo ""
echo "== List notes by encounter =="
LIST=$(curl -sf "${HIS_URL}/api/v1/encounters/${ENC_ID}/consultation-notes")
echo "${LIST}" | python3 -c "import sys,json; b=json.load(sys.stdin); print('count', b['count']); print('statuses', [n.get('status') for n in b.get('notes',[])])"
[[ "$(echo "${LIST}" | python3 -c "import sys,json; print(json.load(sys.stdin)['count'])")" -ge 1 ]] || exit 1

echo ""
echo "== Ensure HTS terminology =="
python3 "${ROOT}/scripts/seed-atrius-terminology.py" --hts-url "${HTS_URL}"

echo ""
echo "== \$validate Composition on HFS =="
COMP_JSON=$(curl -sf "${HIS_URL}/api/v1/consultation-notes/${COMP_ID}")
VALIDATE=$(hfs_curl -X POST "${HFS_URL}/Composition/\$validate" \
  -H "Content-Type: application/fhir+json" \
  -H "Accept: application/fhir+json" \
  -d "${COMP_JSON}")
ERROR_COUNT=$(echo "${VALIDATE}" | python3 -c "import sys,json; o=json.load(sys.stdin); print(sum(1 for i in o.get('issue',[]) if i.get('severity')=='error'))")
echo "Composition \$validate errors: ${ERROR_COUNT}"
[[ "${ERROR_COUNT}" == "0" ]] || { echo "${VALIDATE}" | python3 -m json.tool; exit 1; }

echo ""
echo "Consultation note smoke test complete."
