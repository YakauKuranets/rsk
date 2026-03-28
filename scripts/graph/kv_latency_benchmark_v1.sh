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
import time
n=${N}
x=0
s=time.time()
for i in range(n):
    x += i
print(s)
PY
)
end_base=$(python - <<PY
import time
print(time.time())
PY
)

baseline_ms=$(python - <<PY
print(round((${end_base}-${start_base})*1000,2))
PY
)

shadow_ms=-1
status="blocked"
reason="$(kv_env_reason)"
overhead_ms=-1

if [[ "${reason}" == "ready" ]]; then
  query="UNWIND range(1,${N}) AS i RETURN 1"

  start_shadow=$(python - <<PY
import time; print(time.time())
PY
)
  if kv_run_cypher "${query}" >/dev/null 2>&1; then
    end_shadow=$(python - <<PY
import time; print(time.time())
PY
)
    shadow_ms=$(python - <<PY
print(round((${end_shadow}-${start_shadow})*1000,2))
PY
)
    overhead_ms=$(python - <<PY
print(round(${shadow_ms}-${baseline_ms},2))
PY
)

    if python - <<PY
import sys
sys.exit(0 if ${overhead_ms} < 5000 else 1)
PY
    then
      status="acceptable"
      reason="overhead_within_threshold"
    else
      status="borderline"
      reason="overhead_exceeds_threshold"
    fi
  else
    status="blocked"
    reason="shadow_query_unreachable"
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
  "overhead_ms": ${overhead_ms},
  "marker": "${marker}"
}
JSON

echo "${marker}"
