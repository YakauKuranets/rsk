#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "KV_SHADOW_HEALTH_V1|status=error|reason=missing_env_file|path=${ENV_FILE}"
  exit 1
fi

# shellcheck disable=SC1090
source "${ENV_FILE}"

: "${NEO4J_USER:?NEO4J_USER is required}"
: "${NEO4J_PASSWORD:?NEO4J_PASSWORD is required}"

BOLT_PORT="${NEO4J_BOLT_PORT:-7687}"
BOLT_URL="bolt://localhost:${BOLT_PORT}"

if ! command -v cypher-shell >/dev/null 2>&1; then
  echo "KV_SHADOW_HEALTH_V1|status=error|reason=missing_cypher_shell"
  exit 1
fi

QUERY_RESULT="$(cypher-shell -a "${BOLT_URL}" -u "${NEO4J_USER}" -p "${NEO4J_PASSWORD}" 'RETURN 1 AS ok' --format plain | tail -n 1 | tr -d '\r')"

if [[ "${QUERY_RESULT}" == "1" ]]; then
  echo "KV_SHADOW_HEALTH_V1|status=ok|bolt=${BOLT_URL}|shadow_mode=${KV_SHADOW_MODE:-unknown}"
else
  echo "KV_SHADOW_HEALTH_V1|status=error|reason=unexpected_query_result|value=${QUERY_RESULT}"
  exit 1
fi
