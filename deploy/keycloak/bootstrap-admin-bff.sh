#!/usr/bin/env bash
# Enable staff OIDC login (standard flow + PKCE) on atrius-admin-bff without full realm re-import.
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

CLIENT_JSON=$(auth "${KEYCLOAK_URL}/admin/realms/${REALM}/clients?clientId=atrius-admin-bff")
CLIENT_ID=$(echo "${CLIENT_JSON}" | python3 -c "import sys,json; c=json.load(sys.stdin); print(c[0]['id'] if c else '')")

if [[ -z "${CLIENT_ID}" ]]; then
  echo "atrius-admin-bff client not found in realm ${REALM}" >&2
  exit 1
fi

CURRENT=$(auth "${KEYCLOAK_URL}/admin/realms/${REALM}/clients/${CLIENT_ID}")

echo "${CURRENT}" | python3 -c "
import json, sys
c = json.load(sys.stdin)
c['standardFlowEnabled'] = True
c['directAccessGrantsEnabled'] = True
attrs = c.get('attributes') or {}
attrs['pkce.code.challenge.method'] = 'S256'
c['attributes'] = attrs
uris = set(c.get('redirectUris') or [])
uris.update(['http://127.0.0.1:8084/callback', 'http://localhost:8084/callback'])
c['redirectUris'] = sorted(uris)
origins = set(c.get('webOrigins') or [])
origins.update([
    'http://localhost:5174', 'http://127.0.0.1:5174', 'http://localhost:8084'
])
c['webOrigins'] = sorted(origins)
defaults = set(c.get('defaultClientScopes') or [])
defaults.update(['openid', 'profile', 'email', 'fhirUser',
    'user/Organization.rs', 'user/Organization.u', 'user/HealthcareService.rs',
    'user/Location.rs', 'user/Practitioner.rs',
    'user/Patient.cruds', 'user/Appointment.cruds', 'user/Slot.rs', 'user/Schedule.rs',
    'user/Encounter.cruds'])
c['defaultClientScopes'] = sorted(defaults)
attrs['post.logout.redirect.uris'] = 'http://localhost:5174/login##http://127.0.0.1:5174/login##http://localhost:5173/login##http://127.0.0.1:5173/login'
print(json.dumps(c))
" | auth -X PUT "${KEYCLOAK_URL}/admin/realms/${REALM}/clients/${CLIENT_ID}" \
  -H "Content-Type: application/json" \
  -d @- >/dev/null

echo "atrius-admin-bff: standardFlowEnabled=true, PKCE S256, callback URIs set"
