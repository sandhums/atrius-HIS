#!/usr/bin/env bash
# Start optional Docker infrastructure (postgres, elasticsearch).
# Keycloak is local: https://localhost:8443 (not started by this script).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}/deploy"

KEYCLOAK_URL="${KEYCLOAK_URL:-https://localhost:8443}"

echo "Starting postgres, elasticsearch (optional docker stack)..."
docker compose up -d postgres elasticsearch

echo ""
echo "Waiting for docker services..."
for i in $(seq 1 60); do
  pg_ok=false
  es_ok=false
  kc_ok=false

  docker compose exec -T postgres pg_isready -U sandhu -d hfs_clinical >/dev/null 2>&1 && pg_ok=true
  curl -sf http://127.0.0.1:9200/_cluster/health >/dev/null 2>&1 && es_ok=true
  if [[ "${KEYCLOAK_URL}" == https://* ]]; then
    curl -sfk "${KEYCLOAK_URL}/realms/fhir" >/dev/null 2>&1 && kc_ok=true
  else
    curl -sf "${KEYCLOAK_URL}/realms/fhir" >/dev/null 2>&1 && kc_ok=true
  fi

  if [[ "${pg_ok}" == true && "${es_ok}" == true && "${kc_ok}" == true ]]; then
    echo "Platform infrastructure is up."
    echo "  PostgreSQL (docker)  postgresql://sandhu:parsons02@127.0.0.1:5433/hfs_clinical  (optional)"
    echo "  Elasticsearch        http://127.0.0.1:9200  (optional — skip if HFS uses postgres-only)"
    echo "  Keycloak (local)     ${KEYCLOAK_URL}  (Postgres keycloak_db on :5432)"
    echo ""
    echo "Typical local Postgres on :5432: fhir_server (HFS), auth_db (BFF), keycloak_db (Keycloak)"
    exit 0
  fi
  sleep 2
done

echo "Timed out waiting for platform services." >&2
echo "  Docker: docker compose -f deploy/docker-compose.yml ps" >&2
echo "  Keycloak: ensure local server is running at ${KEYCLOAK_URL}" >&2
exit 1
