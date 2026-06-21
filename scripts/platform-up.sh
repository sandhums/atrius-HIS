#!/usr/bin/env bash
# Start Docker infrastructure for the hardened HIS platform.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}/deploy"

echo "Starting postgres, elasticsearch, keycloak..."
docker compose up -d

echo ""
echo "Waiting for services..."
for i in $(seq 1 60); do
  pg_ok=false
  es_ok=false
  kc_ok=false

  docker compose exec -T postgres pg_isready -U atrius -d hfs_clinical >/dev/null 2>&1 && pg_ok=true
  curl -sf http://127.0.0.1:9200/_cluster/health >/dev/null 2>&1 && es_ok=true
  curl -sf http://127.0.0.1:8180/realms/fhir >/dev/null 2>&1 && kc_ok=true

  if [[ "${pg_ok}" == true && "${es_ok}" == true && "${kc_ok}" == true ]]; then
    echo "Platform infrastructure is up."
    echo "  PostgreSQL     postgresql://atrius:atrius@127.0.0.1:5432/hfs_clinical"
    echo "  Elasticsearch  http://127.0.0.1:9200"
    echo "  Keycloak       http://127.0.0.1:8180  (admin/admin)"
    exit 0
  fi
  sleep 2
done

echo "Timed out waiting for platform services. Check: docker compose -f deploy/docker-compose.yml ps" >&2
exit 1
