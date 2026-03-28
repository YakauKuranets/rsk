#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/bin:$PATH"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "missing_env_file"
  exit 1
fi

# shellcheck disable=SC1090
source "${ENV_FILE}"

if ! command -v cypher-shell >/dev/null 2>&1; then
  echo "missing_cypher_shell"
  exit 1
fi

BOLT_URL="bolt://localhost:${NEO4J_BOLT_PORT:-7687}"

kv_run_cypher() {
  local query="${1:-}"
  if [[ -z "${query}" ]]; then
    return 1
  fi

  cypher-shell \
    -a "${BOLT_URL}" \
    -u "${NEO4J_USER:-neo4j}" \
    -p "${NEO4J_PASSWORD:-}" \
    -d "${NEO4J_DATABASE:-neo4j}" \
    --format plain \
    "${query}"
}

# Compatibility wrappers for existing scripts (no behavior change intended).
kv_load_env() {
  :
}

kv_env_reason() {
  echo "ready"
}

kv_probe_connection() {
  kv_run_cypher "RETURN 1" >/dev/null 2>&1
}

export -f kv_run_cypher
