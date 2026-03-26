#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"
OUT_JSON="${ROOT_DIR}/docs/phase32_remediation_latency_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
N="${1:-100}"

start_base=$(python - <<PY
import time; print(time.time())
PY
)
for ((i=0;i<N;i++)); do :; done
end_base=$(python - <<PY
import time; print(time.time())
PY
)

baseline_ms=$(python - <<PY
print(round((${end_base}-${start_base})*1000,2))
PY
)

shadow_ms=-1
status="pass_with_notes"
if [[ -f "${ENV_FILE}" ]] && command -v cypher-shell >/dev/null 2>&1; then
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
  BOLT_URL="bolt://localhost:${NEO4J_BOLT_PORT:-7687}"
  start_shadow=$(python - <<PY
import time; print(time.time())
PY
)
  for ((i=0;i<N;i++)); do
    cypher-shell -a "${BOLT_URL}" -u "${NEO4J_USER:-neo4j}" -p "${NEO4J_PASSWORD:-}" -d "${NEO4J_DATABASE:-neo4j}" "RETURN 1" >/dev/null 2>&1 || true
  done
  end_shadow=$(python - <<PY
import time; print(time.time())
PY
)
  shadow_ms=$(python - <<PY
print(round((${end_shadow}-${start_shadow})*1000,2))
PY
)
  overhead=$(python - <<PY
print(round(${shadow_ms}-${baseline_ms},2))
PY
)
  if python - <<PY
import sys
sys.exit(0 if ${overhead} < 5000 else 1)
PY
  then
    status="acceptable"
  else
    status="borderline"
  fi
else
  status="blocked"
  overhead=-1
fi

marker="KV_EXIT_REMEDIATION_V1|stage=latency|status=${status}|baselineMs=${baseline_ms}|shadowMs=${shadow_ms}"
cat > "${OUT_JSON}" <<JSON
{
  "version": "phase32_remediation_latency_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${status}",
  "baseline_ms": ${baseline_ms},
  "shadow_ms": ${shadow_ms},
  "overhead_ms": ${overhead:- -1},
  "marker": "${marker}"
}
JSON

echo "${marker}"
