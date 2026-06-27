#!/usr/bin/env bash
# Shared defaults and helpers for HIS smoke scripts.
# Seed data and his-server must use the same X-Tenant-ID (default: atrius-hospital).

: "${HIS_DEFAULT_TENANT:=atrius-hospital}"
: "${HFS_URL:=http://127.0.0.1:8082}"

# Obtain/refreshes SMART token for direct HFS calls (his-server uses its own env token).
ensure_hfs_token() {
  local root="${1:?scripts root}"
  if [[ -z "${HIS_FHIR_BEARER_TOKEN:-}" ]]; then
    export HIS_FHIR_BEARER_TOKEN="$("${root}/deploy/keycloak/get-token.sh" his-backend-client)"
  fi
}

hfs_curl() {
  ensure_hfs_token "${ROOT:-.}"
  curl -sf \
    -H "Authorization: Bearer ${HIS_FHIR_BEARER_TOKEN}" \
    -H "X-Tenant-ID: ${HIS_DEFAULT_TENANT}" \
    "$@"
}

# POST resource JSON to HFS $validate; fail when OperationOutcome reports errors.
validate_fhir_resource() {
  local resource_type="$1"
  local resource_json="$2"
  local validate error_count

  validate=$(hfs_curl -X POST "${HFS_URL}/${resource_type}/\$validate" \
    -H "Content-Type: application/fhir+json" \
    -H "Accept: application/fhir+json" \
    -d "${resource_json}")
  error_count=$(echo "${validate}" | python3 -c "import sys,json; o=json.load(sys.stdin); print(sum(1 for i in o.get('issue',[]) if i.get('severity')=='error'))")
  echo "${resource_type} \$validate errors: ${error_count}"
  if [[ "${error_count}" != "0" ]]; then
    echo "${validate}" | python3 -m json.tool
    return 1
  fi
}

# When his-server returns no slots, probe HFS directly to distinguish seed vs tenant issues.
diagnose_no_free_slots() {
  local schedule_id="${1:-opd-patel-schedule}"
  local start_date="${2:-$(date +%Y-%m-%d)}"

  echo "no free slots via his-server (schedule=${schedule_id}, start=${start_date})" >&2
  echo "Tenant for seed + his-server must match (default: ${HIS_DEFAULT_TENANT})." >&2
  echo "  seed:  python3 scripts/seed-hospital-foundation.py --tenant ${HIS_DEFAULT_TENANT} ..." >&2
  echo "  server: HIS_DEFAULT_TENANT=${HIS_DEFAULT_TENANT} (deploy/env/his-server.env) then restart his-server" >&2

  if [[ -z "${HIS_FHIR_BEARER_TOKEN:-}" ]]; then
    echo "Re-run seed if needed: scripts/seed-hospital-foundation.py" >&2
    return 1
  fi

  local hfs_count
  hfs_count=$(curl -sf \
    -H "Authorization: Bearer ${HIS_FHIR_BEARER_TOKEN}" \
    -H "X-Tenant-ID: ${HIS_DEFAULT_TENANT}" \
    "${HFS_URL}/Slot?status=free&schedule=Schedule/${schedule_id}&start=ge${start_date}&_count=1" \
    | python3 -c "import sys,json; b=json.load(sys.stdin); print(len(b.get('entry',[])))" 2>/dev/null \
    || echo "0")

  if [[ "${hfs_count}" != "0" ]]; then
    echo "HFS has free slots under tenant '${HIS_DEFAULT_TENANT}' but his-server returned none." >&2
    echo "Restart his-server with HIS_DEFAULT_TENANT=${HIS_DEFAULT_TENANT} and retry." >&2
  else
    echo "HFS also has no free slots under tenant '${HIS_DEFAULT_TENANT}' — re-run seed-hospital-foundation.py" >&2
  fi
  return 1
}

# When bed-board is empty, probe HFS for ward/bed Locations under the expected tenant.
diagnose_empty_bed_board() {
  local ward_id="${1:-ward-med-a}"

  echo "bed-board returned no beds (ward_id=${ward_id})" >&2
  echo "Tenant for seed + his-server must match (default: ${HIS_DEFAULT_TENANT})." >&2
  echo "  seed:  python3 scripts/seed-hospital-foundation.py --tenant ${HIS_DEFAULT_TENANT} --token \"\$HIS_FHIR_BEARER_TOKEN\"" >&2
  echo "  server: HIS_DEFAULT_TENANT=${HIS_DEFAULT_TENANT} in deploy/env/his-server.env, then restart his-server" >&2

  if [[ -z "${HIS_FHIR_BEARER_TOKEN:-}" ]]; then
    echo "Set HIS_FHIR_BEARER_TOKEN and re-run seed." >&2
    return 1
  fi

  local bed_count ward_count
  bed_count=$(curl -sf \
    -H "Authorization: Bearer ${HIS_FHIR_BEARER_TOKEN}" \
    -H "X-Tenant-ID: ${HIS_DEFAULT_TENANT}" \
    "${HFS_URL}/Location?partof=Location/${ward_id}&status=active&_count=10" \
    | python3 -c "import sys,json; b=json.load(sys.stdin); print(len(b.get('entry',[])))" 2>/dev/null \
    || echo "0")
  ward_count=$(curl -sf \
    -H "Authorization: Bearer ${HIS_FHIR_BEARER_TOKEN}" \
    -H "X-Tenant-ID: ${HIS_DEFAULT_TENANT}" \
    "${HFS_URL}/Location/${ward_id}" \
    | python3 -c "import sys,json; r=json.load(sys.stdin); print(1 if r.get('resourceType')=='Location' else 0)" 2>/dev/null \
    || echo "0")

  if [[ "${ward_count}" == "0" ]]; then
    echo "HFS has no Location/${ward_id} under tenant '${HIS_DEFAULT_TENANT}' — run seed-hospital-foundation.py" >&2
  elif [[ "${bed_count}" == "0" ]]; then
    echo "Ward exists but no active child Locations — re-run seed (beds may have failed profile validation)" >&2
  else
    echo "HFS has ${bed_count} active Location(s) under ${ward_id} but bed-board still empty — check physicalType=bd" >&2
  fi
  return 1
}

# Discharge in-progress encounter left on a bed from a prior failed smoke run.
cleanup_stale_bed_encounter() {
  local his_url="$1"
  local bed_id="$2"
  local board_json="$3"

  local enc_id
  enc_id=$(echo "${board_json}" | python3 -c "
import json, sys
bed = sys.argv[1]
data = json.load(sys.stdin)
for entry in data.get('beds', []):
    if entry.get('bed_id') == bed and entry.get('encounter_id'):
        print(entry['encounter_id'])
        break
" "${bed_id}")

  if [[ -z "${enc_id}" ]]; then
    return 0
  fi

  echo "== Cleanup: discharge stale encounter ${enc_id} on ${bed_id} ==" >&2
  local http
  http=$(curl -s -o /tmp/his-adt-cleanup.json -w "%{http_code}" -X POST \
    "${his_url}/api/v1/encounters/${enc_id}/discharge" \
    -H "Content-Type: application/json" \
    -d '{"discharge_disposition": "home"}')
  if [[ "${http}" != "200" ]]; then
    echo "Cleanup discharge failed: HTTP ${http}" >&2
    python3 -m json.tool /tmp/his-adt-cleanup.json >&2 || cat /tmp/his-adt-cleanup.json >&2
    return 1
  fi
  python3 -c "import json; e=json.load(open('/tmp/his-adt-cleanup.json')); print('discharged', e.get('id'), e.get('status'))"
}
