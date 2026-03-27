#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_JSON="${ROOT_DIR}/docs/phase32_graph_env_readiness_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

# shellcheck source=scripts/graph/_kv_cypher.sh
source "${ROOT_DIR}/scripts/graph/_kv_cypher.sh"

kv_load_env

neo4j_reachable=false
status="blocked"
reason="$(kv_env_reason)"

if [[ "${reason}" == "ready" ]]; then
  if kv_probe_connection; then
    neo4j_reachable=true
    reason="ready"
    status="pass"
  else
    reason="neo4j_unreachable"
  fi
fi

missing_vars_json="$(printf '%s\n' "${KV_ENV_MISSING_VARS[@]:-}" | python -c 'import json,sys; print(json.dumps([x.strip() for x in sys.stdin if x.strip()]))')"
marker="KV_GRAPH_ENV_READY_V1|status=${status}|reason=${reason}|neo4j_reachable=${neo4j_reachable}|cypher_shell_present=${KV_CYPHER_SHELL_PRESENT}|env_complete=${KV_ENV_COMPLETE}|shadow_write_enabled=${KV_SHADOW_WRITE_ENABLED}|ts=${NOW_UTC}"

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase32_graph_env_readiness_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${status}",
  "reason": "${reason}",
  "neo4j_reachable": ${neo4j_reachable},
  "cypher_shell_present": ${KV_CYPHER_SHELL_PRESENT},
  "env_present": ${KV_ENV_PRESENT},
  "env_complete": ${KV_ENV_COMPLETE},
  "missing_env_keys": ${missing_vars_json},
  "shadow_write_enabled": ${KV_SHADOW_WRITE_ENABLED},
  "marker": "${marker}"
}
JSON

echo "${marker}"
