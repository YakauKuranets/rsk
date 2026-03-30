#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"
SCHEMA_FILE="${ROOT_DIR}/infra/neo4j-shadow/schema_v1.cypher"

if [[ ! -f "${SCHEMA_FILE}" ]]; then
  ALT_SCHEMA_FILE="${ROOT_DIR}/infra/neo4j-shadow/schema.cypher"
  if [[ -f "${ALT_SCHEMA_FILE}" ]]; then
    SCHEMA_FILE="${ALT_SCHEMA_FILE}"
  else
    echo "KV_SHADOW_SCHEMA_APPLY_V1|status=blocked|reason=missing_schema_file|path=${SCHEMA_FILE}"
    exit 1
  fi
fi

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "KV_SHADOW_SCHEMA_APPLY_V1|status=blocked|reason=missing_env_file|path=${ENV_FILE}"
  exit 1
fi

# shellcheck disable=SC1090
source "${ENV_FILE}"

if [[ -z "${NEO4J_USER:-}" || -z "${NEO4J_PASSWORD:-}" ]]; then
  echo "KV_SHADOW_SCHEMA_APPLY_V1|status=blocked|reason=missing_env_keys"
  exit 1
fi

BOLT_PORT="${NEO4J_BOLT_PORT:-7687}"
BOLT_URL="bolt://localhost:${BOLT_PORT}"

if ! command -v cypher-shell >/dev/null 2>&1; then
  echo "KV_SHADOW_SCHEMA_APPLY_V1|status=blocked|reason=missing_cypher_shell|hint=install_neo4j_client"
  exit 1
fi

if ! cypher-shell -a "${BOLT_URL}" -u "${NEO4J_USER}" -p "${NEO4J_PASSWORD}" -f "${SCHEMA_FILE}"; then
  echo "KV_SHADOW_SCHEMA_APPLY_V1|status=blocked|reason=schema_apply_failed|schema=${SCHEMA_FILE}"
  exit 1
fi

echo "KV_SHADOW_SCHEMA_APPLY_V1|status=ok|bolt=${BOLT_URL}|schema=schema_v1"
