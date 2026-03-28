#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"
SCHEMA_FILE="${ROOT_DIR}/infra/neo4j-shadow/schema_v1.cypher"

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "KV_SHADOW_SCHEMA_APPLY_V1|status=error|reason=missing_env_file|path=${ENV_FILE}"
  exit 1
fi

# shellcheck disable=SC1090
source "${ENV_FILE}"

: "${NEO4J_USER:?NEO4J_USER is required}"
: "${NEO4J_PASSWORD:?NEO4J_PASSWORD is required}"

BOLT_PORT="${NEO4J_BOLT_PORT:-7687}"
BOLT_URL="bolt://localhost:${BOLT_PORT}"

if ! command -v cypher-shell >/dev/null 2>&1; then
  echo "KV_SHADOW_SCHEMA_APPLY_V1|status=error|reason=missing_cypher_shell|hint=install_neo4j_client"
  exit 1
fi

cypher-shell -a "${BOLT_URL}" -u "${NEO4J_USER}" -p "${NEO4J_PASSWORD}" -f "${SCHEMA_FILE}"

echo "KV_SHADOW_SCHEMA_APPLY_V1|status=ok|bolt=${BOLT_URL}|schema=schema_v1"
