#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_JSON="${ROOT_DIR}/docs/phase32_remediation_latency_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
N="${1:-100}"

# shellcheck source=scripts/graph/_kv_cypher.sh
source "${ROOT_DIR}/scripts/graph/_kv_cypher.sh"
kv_load_env

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
status="blocked"
reason="$(kv_env_reason)"
overhead=-1

if [[ "${reason}" == "ready" ]]; then
  start_shadow=$(python - <<PY
import time; print(time.time())
PY
)
  success=0
  for ((i=0;i<N;i++)); do
    if kv_run_cypher "RETURN 1" >/dev/null 2>&1; then
      success=$((success + 1))
    fi
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

  if [[ "${success}" -eq 0 ]]; then
    status="blocked"
    reason="shadow_query_unreachable"
  elif python - <<PY
import sys
sys.exit(0 if ${overhead} < 5000 else 1)
PY
  then
    status="acceptable"
    reason="overhead_within_threshold"
  else
    status="borderline"
    reason="overhead_exceeds_threshold"
  fi
fi

marker="KV_EXIT_REMEDIATION_V1|stage=latency|status=${status}|reason=${reason}|baselineMs=${baseline_ms}|shadowMs=${shadow_ms}"
cat > "${OUT_JSON}" <<JSON
{
  "version": "phase32_remediation_latency_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${status}",
  "reason": "${reason}",
  "baseline_ms": ${baseline_ms},
  "shadow_ms": ${shadow_ms},
  "overhead_ms": ${overhead},
  "marker": "${marker}"
}
JSON

echo "${marker}"
