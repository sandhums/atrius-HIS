#!/usr/bin/env bash
# Phase 5c smoke: admit → draft discharge summary → finalize → $validate → export.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "${ROOT}/scripts/smoke-common.sh"
HIS_URL="${HIS_URL:-http://127.0.0.1:8096}"
HTS_URL="${HTS_URL:-http://127.0.0.1:9091}"
HFS_URL="${HFS_URL:-http://127.0.0.1:8082}"
WARD_ID="${WARD_ID:-ward-med-a}"
BED_A="${BED_A:-bed-med-a-01}"
PRACTITIONER_ID="${PRACTITIONER_ID:-dr-patel}"

ensure_hfs_token "${ROOT}"

if ! curl -sf "${HIS_URL}/health" >/dev/null; then
  echo "his-server not reachable at ${HIS_URL}" >&2
  exit 1
fi

echo "== Register inpatient =="
REGISTER=$(curl -sf -X POST "${HIS_URL}/api/v1/patients" \
  -H "Content-Type: application/json" \
  -d "{
        \"family_name\": \"DischargeSmoke${RANDOM}\",
        \"given_names\": [\"Summary\"],
        \"gender\": \"female\",
        \"birth_date\": \"1975-08-10\",
        \"telecom\": [{\"system\": \"phone\", \"value\": \"+91-9000000005\"}]
      }")
PATIENT_ID=$(echo "${REGISTER}" | python3 -c "import sys,json; print(json.load(sys.stdin)['patient_id'])")
echo "patient_id=${PATIENT_ID}"

echo ""
echo "== Admit to ${BED_A} =="
cleanup_stale_bed_encounter "${HIS_URL}" "${BED_A}" "$(curl -sf "${HIS_URL}/api/v1/bed-board?ward_id=${WARD_ID}")" || true
ADMIT=$(curl -sf -X POST "${HIS_URL}/api/v1/encounters/admit" \
  -H "Content-Type: application/json" \
  -d "{
        \"patient_id\": \"${PATIENT_ID}\",
        \"bed_id\": \"${BED_A}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"reason\": \"Discharge summary smoke admission\"
      }")
ENC_ID=$(echo "${ADMIT}" | python3 -c "import sys,json; print(json.load(sys.stdin)['encounter_id'])")
echo "encounter_id=${ENC_ID}"

echo ""
echo "== Create draft discharge summary =="
CREATE=$(curl -sf -X POST "${HIS_URL}/api/v1/discharge-summaries" \
  -H "Content-Type: application/json" \
  -d "{
        \"encounter_id\": \"${ENC_ID}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"title\": \"Inpatient Discharge Summary\",
        \"sections\": {
          \"chief_complaint\": \"Chest pain\",
          \"exam\": \"Stable on discharge\",
          \"hospital_course\": \"Rule out ACS, monitored 48h\",
          \"investigations\": \"Troponin negative x2\",
          \"discharge_medications\": \"Aspirin 75mg daily\",
          \"procedures\": \"Coronary angiography - no significant stenosis\",
          \"care_plan\": \"Cardiology OPD follow up in 2 weeks\"
        }
      }")
COMP_ID=$(echo "${CREATE}" | python3 -c "import sys,json; print(json.load(sys.stdin)['composition_id'])")
echo "composition_id=${COMP_ID}"

echo ""
echo "== Finalize discharge summary =="
FINAL=$(curl -sf -X POST "${HIS_URL}/api/v1/discharge-summaries/${COMP_ID}/finalize" \
  -H "Content-Type: application/json" \
  -d "{\"practitioner_id\": \"${PRACTITIONER_ID}\"}")
echo "status $(echo "${FINAL}" | python3 -c "import sys,json; print(json.load(sys.stdin)['status'])")"

echo ""
echo "== Ensure HTS terminology =="
python3 "${ROOT}/scripts/seed-atrius-terminology.py" --hts-url "${HTS_URL}"

echo ""
echo "== \$validate Composition on HFS =="
COMP_JSON=$(curl -sf "${HIS_URL}/api/v1/discharge-summaries/${COMP_ID}")
VALIDATE=$(hfs_curl -X POST "${HFS_URL}/Composition/\$validate" \
  -H "Content-Type: application/fhir+json" \
  -H "Accept: application/fhir+json" \
  -d "${COMP_JSON}")
ERROR_COUNT=$(echo "${VALIDATE}" | python3 -c "import sys,json; o=json.load(sys.stdin); print(sum(1 for i in o.get('issue',[]) if i.get('severity')=='error'))")
echo "Composition \$validate errors: ${ERROR_COUNT}"
[[ "${ERROR_COUNT}" == "0" ]] || { echo "${VALIDATE}" | python3 -m json.tool; exit 1; }

echo ""
echo "== Export DocumentBundle =="
EXPORT=$(curl -sf -X POST "${HIS_URL}/api/v1/discharge-summaries/${COMP_ID}/export")
BUNDLE_TYPE=$(echo "${EXPORT}" | python3 -c "import sys,json; print(json.load(sys.stdin)['bundle']['type'])")
ENTRY_COUNT=$(echo "${EXPORT}" | python3 -c "import sys,json; print(len(json.load(sys.stdin)['bundle']['entry']))")
echo "bundle type=${BUNDLE_TYPE} entries=${ENTRY_COUNT}"
[[ "${BUNDLE_TYPE}" == "document" ]] || exit 1
[[ "${ENTRY_COUNT}" -ge 3 ]] || exit 1

echo ""
echo "Discharge summary smoke test complete."
