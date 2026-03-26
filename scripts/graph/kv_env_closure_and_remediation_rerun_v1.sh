#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"
ENV_EXAMPLE="${ROOT_DIR}/infra/neo4j-shadow/.env.example"
COMPOSE_FILE="${ROOT_DIR}/infra/neo4j-shadow/docker-compose.yml"
OUT_MD="${ROOT_DIR}/docs/phase32_graph_env_closure_v1.md"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

notes=()
health_marker="KV_SHADOW_HEALTH_V1|status=skipped|reason=healthcheck_not_run"

if [[ ! -f "${ENV_FILE}" && -f "${ENV_EXAMPLE}" ]]; then
  cp "${ENV_EXAMPLE}" "${ENV_FILE}"
  notes+=("env_bootstrapped_from_example")
fi

if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
  if [[ -f "${COMPOSE_FILE}" ]]; then
    docker compose -f "${COMPOSE_FILE}" --env-file "${ENV_FILE}" up -d >/dev/null 2>&1 || notes+=("docker_compose_up_failed")
    notes+=("docker_compose_attempted")
  fi
else
  notes+=("docker_or_compose_unavailable")
fi

if health_out="$(${ROOT_DIR}/scripts/graph/neo4j_shadow_healthcheck.sh 2>&1)"; then
  health_marker="${health_out}"
else
  health_marker="${health_out}"
  notes+=("neo4j_healthcheck_failed")
fi

READINESS_MARKER="$(${ROOT_DIR}/scripts/graph/kv_graph_env_readiness_v1.sh)"
EXIT_MARKER="$(${ROOT_DIR}/scripts/graph/kv_exit_remediation_v1.sh)"

cat > "${OUT_MD}" <<MD
# Phase 32.5r.1 Graph Environment Closure

Generated at: ${NOW_UTC}

## Notes
$(printf '%s\n' "${notes[@]}" | sed 's/^/- /')

## Markers
- ${health_marker}
- ${READINESS_MARKER}
- ${EXIT_MARKER}
MD

echo "${EXIT_MARKER}"
