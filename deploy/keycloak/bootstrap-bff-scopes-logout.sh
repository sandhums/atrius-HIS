#!/usr/bin/env bash
# Patch running Keycloak: full BFF FHIR scopes + post-logout redirect URIs.
# Run after pulling realm.json changes when you cannot re-import the full realm.
set -euo pipefail

KEYCLOAK_URL="${KEYCLOAK_URL:-https://localhost:8443}"
REALM="${REALM:-fhir}"
ADMIN_USER="${ADMIN_USER:-admin}"
ADMIN_PASS="${ADMIN_PASS:-admin}"

CURL_TLS=()
if [[ "${KEYCLOAK_URL}" == https://* ]]; then
  CURL_TLS=(-k)
fi

TOKEN=$(curl -sf "${CURL_TLS[@]}" -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
  -d "client_id=admin-cli" \
  -d "username=${ADMIN_USER}" \
  -d "password=${ADMIN_PASS}" \
  -d "grant_type=password" | python3 -c "import sys,json; print(json.load(sys.stdin)['access_token'])")

auth() {
  curl -sf "${CURL_TLS[@]}" -H "Authorization: Bearer ${TOKEN}" "$@"
}

ensure_scope() {
  local name="$1"
  local desc="$2"
  if auth "${KEYCLOAK_URL}/admin/realms/${REALM}/client-scopes?search=${name}" | python3 -c "
import sys, json
scopes = json.load(sys.stdin)
print('yes' if any(s.get('name') == '${name}' for s in scopes) else 'no')
" | grep -q yes; then
    echo "scope ${name}: ok"
    return
  fi
  auth -X POST "${KEYCLOAK_URL}/admin/realms/${REALM}/client-scopes" \
    -H "Content-Type: application/json" \
    -d "{\"name\":\"${name}\",\"description\":\"${desc}\",\"protocol\":\"openid-connect\",\"attributes\":{\"include.in.token.scope\":\"true\",\"display.on.consent.screen\":\"false\"}}" >/dev/null
  echo "scope ${name}: created"
}

POST_LOGOUT="http://localhost:5174/login##http://127.0.0.1:5174/login##http://localhost:5173/login##http://127.0.0.1:5173/login"

ALL_SCOPES=(
  "launch/patient|SMART EHR launch patient context"
  "launch/encounter|SMART EHR launch encounter context"
  "fhirUser|Practitioner identity for SMART apps"
  "user/Patient.cruds|User CRUD+search Patient"
  "user/Encounter.cruds|User CRUD+search Encounter"
  "user/Appointment.cruds|User CRUD+search Appointment"
  "user/Composition.cruds|User CRUD+search Composition"
  "user/ServiceRequest.cruds|User CRUD+search ServiceRequest"
  "user/EpisodeOfCare.cruds|User CRUD+search EpisodeOfCare"
  "user/Task.cruds|User CRUD+search Task"
  "user/DiagnosticReport.cruds|User CRUD+search DiagnosticReport"
  "user/Slot.rs|User read/search Slot"
  "user/Schedule.rs|User read/search Schedule"
  "user/Location.rs|User read/search Location"
  "user/Organization.rs|User read/search Organization"
  "user/Organization.u|User update Organization"
  "user/HealthcareService.rs|User read/search HealthcareService"
  "user/Practitioner.rs|User read/search Practitioner"
  "user/QuestionnaireResponse.c|User create QuestionnaireResponse"
  "user/QuestionnaireResponse.rs|User read/search QuestionnaireResponse"
  "user/Condition.rs|User read/search Condition"
  "user/Observation.cruds|User CRUD+search Observation"
)

for entry in "${ALL_SCOPES[@]}"; do
  IFS='|' read -r name desc <<< "${entry}"
  ensure_scope "${name}" "${desc}"
done

patch_client() {
  local client_id_name="$1"
  shift
  local -a extra_defaults=("$@")

  local internal_id
  internal_id=$(auth "${KEYCLOAK_URL}/admin/realms/${REALM}/clients?clientId=${client_id_name}" \
    | python3 -c "import sys,json; c=json.load(sys.stdin); print(c[0]['id'] if c else '')")
  if [[ -z "${internal_id}" ]]; then
    echo "${client_id_name}: not found, skipping" >&2
    return
  fi

  auth "${KEYCLOAK_URL}/admin/realms/${REALM}/clients/${internal_id}" \
    | POST_LOGOUT="${POST_LOGOUT}" EXTRA="${extra_defaults[*]}" python3 -c "
import json, os, sys
c = json.load(sys.stdin)
attrs = c.get('attributes') or {}
attrs['pkce.code.challenge.method'] = 'S256'
attrs['post.logout.redirect.uris'] = os.environ['POST_LOGOUT']
c['attributes'] = attrs
defaults = set(c.get('defaultClientScopes') or [])
defaults.update(os.environ['EXTRA'].split())
c['defaultClientScopes'] = sorted(defaults)
print(json.dumps(c))
" | auth -X PUT "${KEYCLOAK_URL}/admin/realms/${REALM}/clients/${internal_id}" \
    -H "Content-Type: application/json" \
    -d @- >/dev/null

  echo "${client_id_name}: scopes + post-logout URIs updated"
}

patch_client "atrius-admin-bff" \
  openid profile email fhirUser \
  user/Organization.rs user/Organization.u user/HealthcareService.rs \
  user/Location.rs user/Practitioner.rs \
  user/Patient.cruds user/Appointment.cruds user/Slot.rs user/Schedule.rs user/Encounter.cruds

patch_client "atrius-clinical-bff" \
  openid profile email launch/patient launch/encounter fhirUser \
  user/Patient.cruds user/Encounter.cruds user/Appointment.cruds \
  user/Composition.cruds user/ServiceRequest.cruds user/EpisodeOfCare.cruds \
  user/Task.cruds user/DiagnosticReport.cruds \
  user/Slot.rs user/Schedule.rs user/Location.rs \
  user/Organization.rs user/HealthcareService.rs user/Practitioner.rs \
  user/QuestionnaireResponse.c user/QuestionnaireResponse.rs \
  user/Condition.rs user/Observation.cruds

echo "Done. Log out of both SPAs and sign in again so new scopes appear in access tokens."
