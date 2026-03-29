#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/bin:$PATH"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"
KV_ENV_PRESENT=false
KV_ENV_COMPLETE=false
KV_SHADOW_WRITE_ENABLED=false
KV_CYPHER_SHELL_PRESENT=false
KV_ENV_REASON="ready"
KV_ENV_MISSING_VARS=()

if [[ -f "${ENV_FILE}" ]]; then
  KV_ENV_PRESENT=true
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
else
  KV_ENV_REASON="missing_env_file"
fi

if [[ "${KV_ENV_PRESENT}" == "true" ]]; then
  if [[ -z "${NEO4J_USER:-}" ]]; then
    KV_ENV_MISSING_VARS+=("NEO4J_USER")
  fi
  if [[ -z "${NEO4J_PASSWORD:-}" ]]; then
    KV_ENV_MISSING_VARS+=("NEO4J_PASSWORD")
  fi
  if [[ ${#KV_ENV_MISSING_VARS[@]} -gt 0 ]]; then
    KV_ENV_REASON="missing_env_keys"
  else
    KV_ENV_COMPLETE=true
  fi
fi

if command -v cypher-shell >/dev/null 2>&1; then
  KV_CYPHER_SHELL_PRESENT=true
elif [[ "${KV_ENV_REASON}" == "ready" ]]; then
  KV_ENV_REASON="missing_cypher_shell"
fi

if [[ "${KV_ENV_REASON}" == "ready" && "${KV_ENV_COMPLETE}" != "true" ]]; then
  KV_ENV_REASON="missing_env_keys"
fi

if [[ "${KV_ENV_REASON}" == "ready" && "${KV_CYPHER_SHELL_PRESENT}" != "true" ]]; then
  KV_ENV_REASON="missing_cypher_shell"
fi

if [[ "${KV_SHADOW_MODE:-false}" == "true" || "${KV_DUAL_WRITE_ENABLED:-false}" == "true" ]]; then
  KV_SHADOW_WRITE_ENABLED=true
fi

BOLT_URL="bolt://localhost:${NEO4J_BOLT_PORT:-7687}"
export KV_ENV_PRESENT KV_ENV_COMPLETE KV_SHADOW_WRITE_ENABLED KV_CYPHER_SHELL_PRESENT KV_ENV_REASON

kv_run_cypher() {
  local query="${1:-}"
  if [[ -z "${query}" ]]; then
    return 1
  fi
  if [[ "$(kv_env_reason)" != "ready" ]]; then
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
  echo "${KV_ENV_REASON}"
}

kv_probe_connection() {
  kv_run_cypher "RETURN 1" >/dev/null 2>&1
}

export -f kv_run_cypher
