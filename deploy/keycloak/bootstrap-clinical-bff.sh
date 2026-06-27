#!/usr/bin/env bash
# Add SMART scopes + atrius-clinical-bff to running local Keycloak without full realm re-import.
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

SCOPES=(
  "launch/patient|SMART EHR launch patient context"
  "launch/encounter|SMART EHR launch encounter context"
  "fhirUser|Practitioner identity for SMART apps"
  "user/Patient.rs|User read/search Patient"
  "user/Encounter.rs|User read/search Encounter"
  "user/Appointment.rs|User read/search Appointment"
  "user/Slot.rs|User read/search Slot"
  "user/Schedule.rs|User read/search Schedule"
  "user/QuestionnaireResponse.c|User create QuestionnaireResponse"
  "user/QuestionnaireResponse.rs|User read/search QuestionnaireResponse"
  "user/Condition.rs|User read/search Condition"
  "user/Observation.rs|User read/search Observation"
)

for entry in "${SCOPES[@]}"; do
  IFS='|' read -r name desc <<< "${entry}"
  ensure_scope "${name}" "${desc}"
done

if auth "${KEYCLOAK_URL}/admin/realms/${REALM}/clients?clientId=atrius-clinical-bff" | python3 -c "
import sys, json
print('yes' if json.load(sys.stdin) else 'no')
" | grep -q yes; then
  echo "client atrius-clinical-bff: ok"
  exit 0
fi

auth -X POST "${KEYCLOAK_URL}/admin/realms/${REALM}/clients" \
  -H "Content-Type: application/json" \
  -d '{
    "clientId": "atrius-clinical-bff",
    "name": "Atrius Clinical BFF",
    "enabled": true,
    "publicClient": false,
    "secret": "atrius-clinical-bff-secret",
    "standardFlowEnabled": true,
    "directAccessGrantsEnabled": true,
    "redirectUris": ["http://127.0.0.1:8084/callback", "http://localhost:8084/callback"],
    "webOrigins": ["http://127.0.0.1:5173", "http://localhost:5173", "http://127.0.0.1:8084", "http://localhost:8084"],
    "attributes": { "pkce.code.challenge.method": "S256" },
    "defaultClientScopes": [
      "openid", "profile", "email",
      "launch/patient", "launch/encounter", "fhirUser",
      "user/Patient.rs", "user/Encounter.rs", "user/Appointment.rs", "user/Slot.rs", "user/Schedule.rs",
      "user/QuestionnaireResponse.c", "user/QuestionnaireResponse.rs", "user/Condition.rs", "user/Observation.rs"
    ]
  }' >/dev/null

echo "client atrius-clinical-bff: created"
