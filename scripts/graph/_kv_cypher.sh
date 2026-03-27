#!/usr/bin/env bash

# Shared Neo4j/cypher-shell bootstrap for Phase 32 remediation scripts.
# Intended to be sourced by sibling scripts.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"

KV_ENV_LOADED="${KV_ENV_LOADED:-false}"
KV_ENV_PRESENT="${KV_ENV_PRESENT:-false}"
KV_ENV_COMPLETE="${KV_ENV_COMPLETE:-false}"
KV_CYPHER_SHELL_PRESENT="${KV_CYPHER_SHELL_PRESENT:-false}"
KV_SHADOW_WRITE_ENABLED="${KV_SHADOW_WRITE_ENABLED:-false}"
KV_BOLT_URL="${KV_BOLT_URL:-}"

kv_load_env() {
  if [[ "${KV_ENV_LOADED}" == "true" ]]; then
    return 0
  fi

  KV_ENV_LOADED=true
  KV_ENV_PRESENT=false
  KV_ENV_COMPLETE=false
  KV_CYPHER_SHELL_PRESENT=false
  KV_SHADOW_WRITE_ENABLED=false
  KV_ENV_MISSING_VARS=()

  if command -v cypher-shell >/dev/null 2>&1; then
    KV_CYPHER_SHELL_PRESENT=true
  fi

  if [[ -f "${ENV_FILE}" ]]; then
    KV_ENV_PRESENT=true
    # shellcheck disable=SC1090
    source "${ENV_FILE}"

    local required_vars=(NEO4J_USER NEO4J_PASSWORD NEO4J_BOLT_PORT NEO4J_DATABASE KV_SHADOW_MODE KV_DUAL_WRITE_ENABLED)
    for v in "${required_vars[@]}"; do
      if [[ -z "${!v:-}" ]]; then
        KV_ENV_MISSING_VARS+=("${v}")
      fi
    done

    if (( ${#KV_ENV_MISSING_VARS[@]} == 0 )); then
      KV_ENV_COMPLETE=true
      KV_BOLT_URL="bolt://localhost:${NEO4J_BOLT_PORT}"
    fi

    if [[ "${KV_SHADOW_MODE:-false}" == "true" && "${KV_DUAL_WRITE_ENABLED:-false}" == "true" ]]; then
      KV_SHADOW_WRITE_ENABLED=true
    fi
  fi
}

kv_env_reason() {
  kv_load_env
  if [[ "${KV_ENV_PRESENT}" != "true" ]]; then
    echo "missing_env_file"
  elif [[ "${KV_ENV_COMPLETE}" != "true" ]]; then
    echo "env_incomplete"
  elif [[ "${KV_CYPHER_SHELL_PRESENT}" != "true" ]]; then
    echo "missing_cypher_shell"
  else
    echo "ready"
  fi
}

kv_run_cypher() {
  kv_load_env
  if [[ "${KV_ENV_COMPLETE}" != "true" || "${KV_CYPHER_SHELL_PRESENT}" != "true" ]]; then
    return 127
  fi
  cypher-shell -a "${KV_BOLT_URL}" -u "${NEO4J_USER}" -p "${NEO4J_PASSWORD}" -d "${NEO4J_DATABASE}" "$@"
}

kv_probe_connection() {
  kv_run_cypher "RETURN 1" >/dev/null 2>&1
}
