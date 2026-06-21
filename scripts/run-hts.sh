#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ATRIUS_HFS_PATH="${ATRIUS_HFS_PATH:-${ROOT}/../atrius-hfs}"
ENV_FILE="${ROOT}/deploy/env/hts.env"

if [[ ! -d "${ATRIUS_HFS_PATH}" ]]; then
  echo "atrius-hfs not found at ${ATRIUS_HFS_PATH}. Set ATRIUS_HFS_PATH." >&2
  exit 1
fi

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "Missing ${ENV_FILE}. Copy from hts.env.example first." >&2
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "${ENV_FILE}"
set +a

mkdir -p "${ROOT}/data"

cd "${ATRIUS_HFS_PATH}"
echo "Starting HTS on port ${HTS_SERVER_PORT:-9091}..."
exec cargo run --release --bin hts
