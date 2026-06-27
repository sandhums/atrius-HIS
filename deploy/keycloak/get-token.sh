#!/usr/bin/env bash
# Obtain a client-credentials token from local Keycloak (https://localhost:8443 by default).
#
# Usage:
#   ./get-token.sh                          # HFS full-access client
#   ./get-token.sh his-backend-client       # HIS domain services client
#   ./get-token.sh hfs-readonly-client      # read-only Patient client
#
# Example: export HIS_FHIR_BEARER_TOKEN=$(./get-token.sh his-backend-client)

set -euo pipefail

KEYCLOAK_URL="${KEYCLOAK_URL:-https://localhost:8443}"
REALM="${REALM:-fhir}"
CLIENT_ID="${1:-hfs-backend-client}"

case "${CLIENT_ID}" in
  hfs-backend-client)  CLIENT_SECRET="${CLIENT_SECRET:-hfs-backend-secret}" ;;
  hfs-readonly-client) CLIENT_SECRET="${CLIENT_SECRET:-hfs-readonly-secret}" ;;
  his-backend-client)  CLIENT_SECRET="${CLIENT_SECRET:-his-backend-secret}" ;;
  *)                   CLIENT_SECRET="${CLIENT_SECRET:?CLIENT_SECRET must be set for custom client IDs}" ;;
esac

TOKEN_ENDPOINT="${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/token"

CURL_TLS=()
if [[ "${KEYCLOAK_URL}" == https://* ]]; then
  CURL_TLS=(-k)
fi

echo "Requesting token from ${TOKEN_ENDPOINT} (client: ${CLIENT_ID})" >&2

RESPONSE=$(curl -sf "${CURL_TLS[@]}" -X POST "${TOKEN_ENDPOINT}" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials" \
  --data-urlencode "client_id=${CLIENT_ID}" \
  --data-urlencode "client_secret=${CLIENT_SECRET}")

ACCESS_TOKEN=$(echo "${RESPONSE}" | python3 -c "import sys,json; print(json.load(sys.stdin)['access_token'])")

echo "Token obtained (expires in $(echo "${RESPONSE}" | python3 -c "import sys,json; print(json.load(sys.stdin).get('expires_in','?'))") seconds)" >&2
echo "${ACCESS_TOKEN}"
