#!/usr/bin/env bash
# Phase 5c/5d smoke: all clinical document kinds — create → finalize → $validate → export.
#
# Covers entry-sliced (OPD): prescription, immunization, invoice, wellness
# and SNOMED-sliced (IP): progress, procedure, operative, anesthesia.
#
# Prerequisites: platform-up, HFS, HTS, his-server, seed-hospital-foundation, bearer token.
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
WARD_ID="${WARD_ID:-ward-med-a}"
BED_IP="${BED_IP:-bed-med-a-02}"

ensure_hfs_token "${ROOT}"

if ! curl -sf "${HIS_URL}/health" >/dev/null; then
  echo "his-server not reachable at ${HIS_URL}" >&2
  exit 1
fi

echo "== Ensure HTS terminology =="
python3 "${ROOT}/scripts/seed-atrius-terminology.py" --hts-url "${HTS_URL}"

validate_document() {
  local label="$1"
  local api_path="$2"
  local comp_id="$3"

  echo ""
  echo "== ${label}: finalize → \$validate → export (${comp_id}) =="
  curl -sf -X POST "${HIS_URL}/api/v1/${api_path}/${comp_id}/finalize" \
    -H "Content-Type: application/json" \
    -d "{\"practitioner_id\": \"${PRACTITIONER_ID}\"}" >/dev/null

  local comp_json validate error_count warn_count
  comp_json=$(curl -sf "${HIS_URL}/api/v1/${api_path}/${comp_id}")
  validate=$(hfs_curl -X POST "${HFS_URL}/Composition/\$validate" \
    -H "Content-Type: application/fhir+json" \
    -H "Accept: application/fhir+json" \
    -d "${comp_json}")
  error_count=$(echo "${validate}" | python3 -c "import sys,json; o=json.load(sys.stdin); print(sum(1 for i in o.get('issue',[]) if i.get('severity')=='error'))")
  warn_count=$(echo "${validate}" | python3 -c "import sys,json; o=json.load(sys.stdin); print(sum(1 for i in o.get('issue',[]) if i.get('severity')=='warning'))")
  echo "\$validate errors=${error_count} warnings=${warn_count}"
  if [[ "${error_count}" != "0" ]]; then
    echo "${validate}" | python3 -m json.tool
    exit 1
  fi

  local export_json bundle_type entry_count
  export_json=$(curl -sf -X POST "${HIS_URL}/api/v1/${api_path}/${comp_id}/export")
  bundle_type=$(echo "${export_json}" | python3 -c "import sys,json; print(json.load(sys.stdin)['bundle']['type'])")
  entry_count=$(echo "${export_json}" | python3 -c "import sys,json; print(len(json.load(sys.stdin)['bundle']['entry']))")
  echo "export type=${bundle_type} entries=${entry_count}"
  [[ "${bundle_type}" == "document" ]] || exit 1
  [[ "${entry_count}" -ge 3 ]] || exit 1
}

create_document() {
  local label="$1"
  local api_path="$2"
  local body="$3"

  echo ""
  echo "== ${label}: create draft =="
  local create comp_id
  create=$(curl -sf -X POST "${HIS_URL}/api/v1/${api_path}" \
    -H "Content-Type: application/json" \
    -d "${body}")
  comp_id=$(echo "${create}" | python3 -c "import sys,json; print(json.load(sys.stdin)['composition_id'])")
  echo "composition_id=${comp_id}"
  validate_document "${label}" "${api_path}" "${comp_id}"
}

echo ""
echo "== Setup OPD encounter =="
OPD_PATIENT=$(curl -sf -X POST "${HIS_URL}/api/v1/patients" \
  -H "Content-Type: application/json" \
  -d "{
        \"family_name\": \"ClinicalDocsOPD${RANDOM}\",
        \"given_names\": [\"Smoke\"],
        \"gender\": \"male\",
        \"birth_date\": \"1992-04-01\",
        \"telecom\": [{\"system\": \"phone\", \"value\": \"+91-9000000101\"}]
      }")
OPD_PATIENT_ID=$(echo "${OPD_PATIENT}" | python3 -c "import sys,json; print(json.load(sys.stdin)['patient_id'])")
SLOT_ID=$(curl -sf "${HIS_URL}/api/v1/slots?schedule_id=${SCHEDULE_ID}&start=${START_DATE}" \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['slots'][0]['slot_id'])")
APPT_ID=$(curl -sf -X POST "${HIS_URL}/api/v1/appointments" \
  -H "Content-Type: application/json" \
  -d "{
        \"patient_id\": \"${OPD_PATIENT_ID}\",
        \"slot_id\": \"${SLOT_ID}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"location_id\": \"${LOCATION_ID}\",
        \"description\": \"Clinical documents OPD smoke\"
      }" \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['appointment_id'])")
ENC_OPD=$(curl -sf -X POST "${HIS_URL}/api/v1/encounters/start-visit" \
  -H "Content-Type: application/json" \
  -d "{
        \"appointment_id\": \"${APPT_ID}\",
        \"reason\": \"Clinical documents smoke\"
      }" \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['encounter_id'])")
echo "patient_id=${OPD_PATIENT_ID} encounter_id=${ENC_OPD}"

create_document "Prescription record" "prescription-records" "{
  \"encounter_id\": \"${ENC_OPD}\",
  \"practitioner_id\": \"${PRACTITIONER_ID}\",
  \"title\": \"Outpatient Prescription\",
  \"sections\": {
    \"medications\": [\"Metformin 500mg BD\", \"Atorvastatin 10mg OD\"]
  }
}"

create_document "Immunization record" "immunization-records" "{
  \"encounter_id\": \"${ENC_OPD}\",
  \"practitioner_id\": \"${PRACTITIONER_ID}\",
  \"title\": \"Vaccination visit\",
  \"sections\": {
    \"immunizations\": [\"Hep B booster\", \"Influenza 2026\"]
  }
}"

create_document "Invoice record" "invoice-records" "{
  \"encounter_id\": \"${ENC_OPD}\",
  \"practitioner_id\": \"${PRACTITIONER_ID}\",
  \"title\": \"OPD Bill\",
  \"sections\": {
    \"summary\": \"Consultation + vaccines\",
    \"amount_inr\": \"1500.00\"
  }
}"

create_document "Wellness record" "wellness-records" "{
  \"encounter_id\": \"${ENC_OPD}\",
  \"practitioner_id\": \"${PRACTITIONER_ID}\",
  \"title\": \"Annual Wellness\",
  \"sections\": {
    \"vital_signs\": \"BP 118/76, HR 72\",
    \"body_measurement\": \"BMI 22.8\"
  }
}"

echo ""
echo "== Setup inpatient encounter =="
IP_PATIENT=$(curl -sf -X POST "${HIS_URL}/api/v1/patients" \
  -H "Content-Type: application/json" \
  -d "{
        \"family_name\": \"ClinicalDocsIP${RANDOM}\",
        \"given_names\": [\"Smoke\"],
        \"gender\": \"female\",
        \"birth_date\": \"1970-05-05\",
        \"telecom\": [{\"system\": \"phone\", \"value\": \"+91-9000000102\"}]
      }")
IP_PATIENT_ID=$(echo "${IP_PATIENT}" | python3 -c "import sys,json; print(json.load(sys.stdin)['patient_id'])")
cleanup_stale_bed_encounter "${HIS_URL}" "${BED_IP}" \
  "$(curl -sf "${HIS_URL}/api/v1/bed-board?ward_id=${WARD_ID}")" || true
ENC_IP=$(curl -sf -X POST "${HIS_URL}/api/v1/encounters/admit" \
  -H "Content-Type: application/json" \
  -d "{
        \"patient_id\": \"${IP_PATIENT_ID}\",
        \"bed_id\": \"${BED_IP}\",
        \"practitioner_id\": \"${PRACTITIONER_ID}\",
        \"reason\": \"Clinical documents IP smoke\"
      }" \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['encounter_id'])")
echo "patient_id=${IP_PATIENT_ID} encounter_id=${ENC_IP}"

create_document "Progress note" "progress-notes" "{
  \"encounter_id\": \"${ENC_IP}\",
  \"practitioner_id\": \"${PRACTITIONER_ID}\",
  \"title\": \"Day 1 Progress Note\",
  \"sections\": {
    \"subjective\": \"Post-op pain controlled\",
    \"objective\": \"Incision clean, vitals stable\",
    \"assessment\": \"Recovering as expected\",
    \"plan\": \"Mobilize today, continue antibiotics\"
  }
}"

create_document "Procedure note" "procedure-notes" "{
  \"encounter_id\": \"${ENC_IP}\",
  \"practitioner_id\": \"${PRACTITIONER_ID}\",
  \"title\": \"Central line placement\",
  \"sections\": {
    \"indication\": \"IV access required\",
    \"procedure\": \"Ultrasound-guided IJ central line\",
    \"findings\": \"Good flow, no pneumothorax\"
  }
}"

create_document "Operative note" "operative-notes" "{
  \"encounter_id\": \"${ENC_IP}\",
  \"practitioner_id\": \"${PRACTITIONER_ID}\",
  \"title\": \"Appendectomy\",
  \"sections\": {
    \"pre_op_diagnosis\": \"Acute appendicitis\",
    \"procedure_performed\": \"Laparoscopic appendectomy\",
    \"intraoperative_findings\": \"Inflamed appendix, no perforation\",
    \"specimens\": \"Appendix sent to pathology\",
    \"post_op_plan\": \"Advance diet POD1, pain control\"
  }
}"

create_document "Anesthesia record" "anesthesia-records" "{
  \"encounter_id\": \"${ENC_IP}\",
  \"practitioner_id\": \"${PRACTITIONER_ID}\",
  \"title\": \"General anesthesia\",
  \"sections\": {
    \"pre_anesthesia_eval\": \"ASA II, NPO confirmed\",
    \"airway_assessment\": \"Mallampati II, adequate mouth opening\",
    \"anesthetic_agents\": \"Propofol induction, sevoflurane maintenance\",
    \"intraoperative_monitoring\": \"Stable vitals throughout\",
    \"pacu_handoff\": \"Awake, pain controlled, SpO2 98% on room air\"
  }
}"

echo ""
echo "Clinical documents smoke test complete (8 document kinds)."
