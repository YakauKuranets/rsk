#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "KV_SHADOW_HEALTH_V1|status=blocked|reason=missing_env_file|path=${ENV_FILE}"
  exit 1
fi

# shellcheck disable=SC1090
source "${ENV_FILE}"

if [[ -z "${NEO4J_USER:-}" || -z "${NEO4J_PASSWORD:-}" ]]; then
  echo "KV_SHADOW_HEALTH_V1|status=blocked|reason=missing_env_keys"
  exit 1
fi

BOLT_PORT="${NEO4J_BOLT_PORT:-7687}"
BOLT_URL="bolt://localhost:${BOLT_PORT}"

if ! command -v cypher-shell >/dev/null 2>&1; then
  echo "KV_SHADOW_HEALTH_V1|status=blocked|reason=missing_cypher_shell"
  exit 1
fi

if ! QUERY_RESULT="$(cypher-shell -a "${BOLT_URL}" -u "${NEO4J_USER}" -p "${NEO4J_PASSWORD}" 'RETURN 1 AS ok' --format plain 2>/dev/null | tail -n 1 | tr -d '\r')"; then
  echo "KV_SHADOW_HEALTH_V1|status=blocked|reason=neo4j_unreachable|bolt=${BOLT_URL}"
  exit 1
fi

if [[ "${QUERY_RESULT}" == "1" ]]; then
  echo "KV_SHADOW_HEALTH_V1|status=ok|bolt=${BOLT_URL}|shadow_mode=${KV_SHADOW_MODE:-unknown}"
else
  echo "KV_SHADOW_HEALTH_V1|status=blocked|reason=unexpected_query_result|value=${QUERY_RESULT}"
  exit 1
fi
