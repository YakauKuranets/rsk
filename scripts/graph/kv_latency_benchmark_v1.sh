#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_JSON="${ROOT_DIR}/docs/phase32_remediation_latency_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
N="${1:-100}"
BATCH_FILE="${ROOT_DIR}/docs/phase32_latency_batch_benchmark.cypher"

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

baseline_total_ms=$(python - <<PY
print(round((${end_base}-${start_base})*1000,2))
PY
)
baseline_ms=$(python - <<PY
print(round(${baseline_total_ms}/${N},2))
PY
)

shadow_ms=-1
shadow_total_ms=-1
status="blocked"
reason="$(kv_env_reason)"
overhead_ms=-1

if [[ "${reason}" == "ready" ]]; then
  : > "${BATCH_FILE}"
  for ((i=0;i<N;i++)); do
    echo "RETURN 1;" >> "${BATCH_FILE}"
  done

  start_shadow=$(python - <<PY
import time; print(time.time())
PY
)

  if kv_run_cypher -f "${BATCH_FILE}" >/dev/null 2>&1; then
    end_shadow=$(python - <<PY
import time; print(time.time())
PY
)
    shadow_total_ms=$(python - <<PY
print(round((${end_shadow}-${start_shadow})*1000,2))
PY
)
    shadow_ms=$(python - <<PY
print(round(${shadow_total_ms}/${N},2))
PY
)
    overhead_ms=$(python - <<PY
print(round(${shadow_ms}-${baseline_ms},2))
PY
)

    if python - <<PY
import sys
sys.exit(0 if ${shadow_ms} <= 100 else 1)
PY
    then
      status="acceptable"
      reason="average_latency_within_threshold"
    else
      status="borderline"
      reason="average_latency_above_target"
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
  "baseline_total_ms": ${baseline_total_ms},
  "shadow_ms": ${shadow_ms},
  "shadow_total_ms": ${shadow_total_ms},
  "overhead_ms": ${overhead_ms},
  "batch_size": ${N},
  "batch_file": "docs/phase32_latency_batch_benchmark.cypher",
  "marker": "${marker}"
}
JSON

echo "${marker}"
