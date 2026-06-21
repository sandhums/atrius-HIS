#!/usr/bin/env bash
# Smoke test for Phase 3 ADT (admit, transfer, discharge, bed board).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "${ROOT}/scripts/smoke-common.sh"
HIS_URL="${HIS_URL:-http://127.0.0.1:8096}"
HTS_URL="${HTS_URL:-http://127.0.0.1:9091}"
WARD_ID="${WARD_ID:-ward-med-a}"
BED_A="${BED_A:-bed-med-a-01}"
BED_B="${BED_B:-bed-med-a-02}"
PRACTITIONER_ID="${PRACTITIONER_ID:-dr-patel}"
HFS_URL="${HFS_URL:-http://127.0.0.1:8082}"

ensure_hfs_token "${ROOT}"

if ! curl -sf "${HIS_URL}/health" >/dev/null; then
  echo "his-server not reachable at ${HIS_URL}. Start it with HIS_FHIR_BEARER_TOKEN set." >&2
  exit 1
fi

echo "== Register inpatient =="
REGISTER=$(curl -sf -X POST "${HIS_URL}/api/v1/patients" \
  -H "Content-Type: application/json" \
  -d "{
        \"family_name\": \"AdtSmoke${RANDOM}\",
        \"given_names\": [\"Phase3\"],
        \"gender\": \"female\",
        \"birth_date\": \"1975-08-10\",
        \"telecom\": [{\"system\": \"phone\", \"value\": \"+91-9000000002\"}],
        \"address\": [{
          \"use\": \"home\",
          \"line\": [\"22 Ward Road\"],
          \"city\": \"Bengaluru\",
          \"state\": \"KA\",
          \"postal_code\": \"560001\",
          \"country\": \"IN\"
        }],
        \"birth_place\": {\"city\": \"Mysuru\", \"state\": \"KA\", \"country\": \"IN\"}
      }")
PATIENT_ID=$(echo "${REGISTER}" | python3 -c "import sys,json; print(json.load(sys.stdin)['patient_id'])")
echo "patient_id=${PATIENT_ID}"

echo ""
echo "== Bed board (expect vacant beds) =="
BED_BOARD=$(curl -sf "${HIS_URL}/api/v1/bed-board?ward_id=${WARD_ID}")
echo "${BED_BOARD}" | python3 -m json.tool | head -35
BED_COUNT=$(echo "${BED_BOARD}" | python3 -c "import sys,json; print(json.load(sys.stdin).get('count',0))")
if [[ "${BED_COUNT}" == "0" ]]; then
  diagnose_empty_bed_board "${WARD_ID}" || true
  echo "Aborting: seed foundation data before ADT smoke." >&2
  exit 1
fi

cleanup_stale_bed_encounter "${HIS_URL}" "${BED_A}" "${BED_BOARD}" || exit 1
cleanup_stale_bed_encounter "${HIS_URL}" "${BED_B}" "${BED_BOARD}" || exit 1
BED_BOARD=$(curl -sf "${HIS_URL}/api/v1/bed-board?ward_id=${WARD_ID}")
echo ""
echo "== Bed board after cleanup =="
echo "${BED_BOARD}" | python3 -m json.tool | head -35

OCCUPIED_A=$(echo "${BED_BOARD}" | python3 -c "import sys,json; b=json.load(sys.stdin); print(next((x['occupied'] for x in b['beds'] if x['bed_id']=='${BED_A}'), True))")
if [[ "${OCCUPIED_A}" == "True" ]]; then
  echo "Bed ${BED_A} still occupied after cleanup — manual discharge or docker compose down -v required." >&2
  exit 1
fi

echo ""
echo "== Admit to ${BED_A} =="
set +e
ADMIT_HTTP=$(curl -s -o /tmp/his-adt-admit.json -w "%{http_code}" -X POST "${HIS_URL}/api/v1/encounters/admit" \
  -H "Content-Type: application/json" \
  -d "{
        \"patient_id\": \"${PATIENT_ID}\",
        \"bed_id\": \"${BED_A}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"reason\": \"Planned admission for observation\"
      }")
set -e
if [[ "${ADMIT_HTTP}" != "200" ]]; then
  echo "Admit failed: HTTP ${ADMIT_HTTP}" >&2
  python3 -m json.tool /tmp/his-adt-admit.json >&2 || cat /tmp/his-adt-admit.json >&2
  exit 1
fi
ADMIT=$(cat /tmp/his-adt-admit.json)
echo "${ADMIT}" | python3 -m json.tool
ENC_ID=$(echo "${ADMIT}" | python3 -c "import sys,json; print(json.load(sys.stdin)['encounter_id'])")
EPISODE_ID=$(echo "${ADMIT}" | python3 -c "import sys,json; d=json.load(sys.stdin); e=d.get('episode_id'); assert e, 'expected episode_id on admit'; print(e)")
echo "episode_id=${EPISODE_ID}"

echo ""
echo "== Ensure HTS terminology for \$validate =="
python3 "${ROOT}/scripts/seed-atrius-terminology.py" --hts-url "${HTS_URL}"

echo ""
echo "== \$validate occupied bed Location on HFS =="
ensure_hfs_token "${ROOT}"
BED_JSON=$(hfs_curl "${HFS_URL}/Location/${BED_A}")
BED_VALIDATE=$(hfs_curl -X POST "${HFS_URL}/Location/\$validate" \
  -H "Content-Type: application/fhir+json" \
  -H "Accept: application/fhir+json" \
  -d "${BED_JSON}")
BED_ERRORS=$(echo "${BED_VALIDATE}" | python3 -c "import sys,json; o=json.load(sys.stdin); print(sum(1 for i in o.get('issue',[]) if i.get('severity')=='error'))")
echo "Bed Location \$validate errors: ${BED_ERRORS}"
[[ "${BED_ERRORS}" == "0" ]] || { echo "${BED_VALIDATE}" | python3 -m json.tool; exit 1; }

echo ""
echo "== \$validate inpatient Encounter on HFS =="
ENC_JSON=$(curl -sf "${HIS_URL}/api/v1/encounters/${ENC_ID}")
VALIDATE=$(hfs_curl -X POST "${HFS_URL}/Encounter/\$validate" \
  -H "Content-Type: application/fhir+json" \
  -H "Accept: application/fhir+json" \
  -d "${ENC_JSON}")
ERROR_COUNT=$(echo "${VALIDATE}" | python3 -c "import sys,json; o=json.load(sys.stdin); print(sum(1 for i in o.get('issue',[]) if i.get('severity')=='error'))")
echo "Encounter \$validate errors: ${ERROR_COUNT}"
[[ "${ERROR_COUNT}" == "0" ]] || { echo "${VALIDATE}" | python3 -m json.tool; exit 1; }

echo ""
echo "== Double admit same bed (expect HTTP 409) =="
set +e
HTTP=$(curl -s -o /tmp/his-adt-dup.json -w "%{http_code}" -X POST "${HIS_URL}/api/v1/encounters/admit" \
  -H "Content-Type: application/json" \
  -d "{\"patient_id\": \"${PATIENT_ID}\", \"bed_id\": \"${BED_A}\"}")
set -e
echo "HTTP ${HTTP}"
python3 -m json.tool /tmp/his-adt-dup.json
[[ "${HTTP}" == "409" ]] || { echo "Expected 409 for occupied bed" >&2; exit 1; }

echo ""
echo "== Transfer to ${BED_B} =="
TRANSFER=$(curl -sf -X POST "${HIS_URL}/api/v1/encounters/${ENC_ID}/transfer" \
  -H "Content-Type: application/json" \
  -d "{\"new_bed_id\": \"${BED_B}\", \"reason\": \"Patient moved for isolation\"}")
echo "${TRANSFER}" | python3 -c "import sys,json; e=json.load(sys.stdin); print(e['resourceType'], e['id'], e['status']); print(e['location'][-1]['location']['reference'])"

echo ""
echo "== Bed board (expect ${BED_B} occupied) =="
curl -sf "${HIS_URL}/api/v1/bed-board?ward_id=${WARD_ID}" | python3 -c "import sys,json; b=json.load(sys.stdin); print('count', b['count']); [print(x['bed_id'], 'occupied='+str(x['occupied']), 'enc='+str(x.get('encounter_id'))) for x in b['beds']]"

echo ""
echo "== Discharge =="
DISCHARGE=$(curl -sf -X POST "${HIS_URL}/api/v1/encounters/${ENC_ID}/discharge" \
  -H "Content-Type: application/json" \
  -d '{"discharge_disposition": "home"}')
echo "${DISCHARGE}" | python3 -c "import sys,json; e=json.load(sys.stdin); print(e['status'], e.get('period',{}).get('end'))"

echo ""
echo "ADT smoke test complete."
