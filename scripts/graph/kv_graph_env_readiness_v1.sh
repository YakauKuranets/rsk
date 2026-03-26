#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"
OUT_JSON="${ROOT_DIR}/docs/phase32_graph_env_readiness_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

cypher_shell_present=false
env_present=false
env_complete=false
neo4j_reachable=false
shadow_write_enabled=false
status="blocked"
reason="unknown"

required_vars=(NEO4J_USER NEO4J_PASSWORD NEO4J_BOLT_PORT NEO4J_DATABASE KV_SHADOW_MODE KV_DUAL_WRITE_ENABLED)
missing_vars=()

if command -v cypher-shell >/dev/null 2>&1; then
  cypher_shell_present=true
fi

if [[ -f "${ENV_FILE}" ]]; then
  env_present=true
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
  for v in "${required_vars[@]}"; do
    if [[ -z "${!v:-}" ]]; then
      missing_vars+=("${v}")
    fi
  done
  if (( ${#missing_vars[@]} == 0 )); then
    env_complete=true
  fi
  if [[ "${KV_SHADOW_MODE:-false}" == "true" && "${KV_DUAL_WRITE_ENABLED:-false}" == "true" ]]; then
    shadow_write_enabled=true
  fi
fi

if [[ "${env_complete}" == true && "${cypher_shell_present}" == true ]]; then
  BOLT_URL="bolt://localhost:${NEO4J_BOLT_PORT}"
  if cypher-shell -a "${BOLT_URL}" -u "${NEO4J_USER}" -p "${NEO4J_PASSWORD}" -d "${NEO4J_DATABASE}" "RETURN 1" >/dev/null 2>&1; then
    neo4j_reachable=true
  fi
fi

if [[ "${env_present}" != true ]]; then
  reason="missing_env_file"
elif [[ "${env_complete}" != true ]]; then
  reason="env_incomplete"
elif [[ "${cypher_shell_present}" != true ]]; then
  reason="missing_cypher_shell"
elif [[ "${neo4j_reachable}" != true ]]; then
  reason="neo4j_unreachable"
elif [[ "${shadow_write_enabled}" != true ]]; then
  reason="shadow_write_disabled"
else
  reason="ready"
  status="pass"
fi

missing_vars_json="$(printf '%s\n' "${missing_vars[@]}" | python -c 'import json,sys; print(json.dumps([x.strip() for x in sys.stdin if x.strip()]))')"
marker="KV_GRAPH_ENV_READY_V1|status=${status}|reason=${reason}|neo4j_reachable=${neo4j_reachable}|cypher_shell_present=${cypher_shell_present}|env_complete=${env_complete}|shadow_write_enabled=${shadow_write_enabled}|ts=${NOW_UTC}"

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase32_graph_env_readiness_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${status}",
  "reason": "${reason}",
  "neo4j_reachable": ${neo4j_reachable},
  "cypher_shell_present": ${cypher_shell_present},
  "env_present": ${env_present},
  "env_complete": ${env_complete},
  "missing_env_keys": ${missing_vars_json},
  "shadow_write_enabled": ${shadow_write_enabled},
  "marker": "${marker}"
}
JSON

echo "${marker}"
