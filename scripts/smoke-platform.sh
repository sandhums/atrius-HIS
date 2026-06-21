#!/usr/bin/env bash
# Smoke test for Phase 0 platform hardening.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HFS_URL="${HFS_URL:-http://127.0.0.1:8082}"
HTS_URL="${HTS_URL:-http://127.0.0.1:9091}"
HIS_URL="${HIS_URL:-http://127.0.0.1:8096}"
TENANT="${HIS_DEFAULT_TENANT:-atrius-hospital}"
TOKEN="${HIS_FHIR_BEARER_TOKEN:-}"

auth_args=()
if [[ -n "${TOKEN}" ]]; then
  auth_args=(-H "Authorization: Bearer ${TOKEN}")
fi

echo "== Infrastructure =="
curl -sf http://127.0.0.1:9200/_cluster/health | python3 -c "import sys,json; h=json.load(sys.stdin); print(f'Elasticsearch: {h[\"status\"]}')"
curl -sf http://127.0.0.1:8180/realms/fhir >/dev/null && echo "Keycloak realm: ok"

echo ""
echo "== HTS =="
curl -sf "${HTS_URL}/health" >/dev/null && echo "HTS /health: ok"

echo ""
echo "== Clinical HFS =="
curl -sf "${auth_args[@]}" -H "X-Tenant-ID: ${TENANT}" "${HFS_URL}/metadata" \
  | python3 -c "import sys,json; m=json.load(sys.stdin); print(f'HFS metadata: {m.get(\"software\",{}).get(\"name\",\"?\")} fhirVersion={m.get(\"fhirVersion\")}')"

echo ""
echo "== his-server =="
curl -sf "${HIS_URL}/health" | python3 -c "import sys,json; print('his-server /health:', json.load(sys.stdin))"
curl -sf "${HIS_URL}/ready" | python3 -c "import sys,json; r=json.load(sys.stdin); print('his-server /ready:', r)"

echo ""
echo "Platform smoke test complete."
