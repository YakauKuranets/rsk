#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_JSON="${ROOT_DIR}/docs/phase32_exit_remediation_report_v1.json"
REPORT_MD="${ROOT_DIR}/docs/phase32_exit_remediation_report_v1.md"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

"${ROOT_DIR}/scripts/graph/kv_graph_env_readiness_v1.sh" >/dev/null
"${ROOT_DIR}/scripts/graph/kv_integrated_100_event_load.sh" 100 >/dev/null
"${ROOT_DIR}/scripts/graph/kv_reconciliation_check_v1.sh" >/dev/null
"${ROOT_DIR}/scripts/graph/kv_latency_benchmark_v1.sh" 100 >/dev/null

read_json_field() {
  local file="$1"
  local key="$2"
  python - <<PY
import json
print(json.load(open('${file}'))${key})
PY
}

readiness_status="$(read_json_field "${ROOT_DIR}/docs/phase32_graph_env_readiness_v1.json" "['status']")"
readiness_reason="$(read_json_field "${ROOT_DIR}/docs/phase32_graph_env_readiness_v1.json" "['reason']")"
readiness_marker="$(read_json_field "${ROOT_DIR}/docs/phase32_graph_env_readiness_v1.json" "['marker']")"

load_status="$(read_json_field "${ROOT_DIR}/docs/phase32_remediation_integrated_load_v1.json" "['status']")"
load_reason="$(read_json_field "${ROOT_DIR}/docs/phase32_remediation_integrated_load_v1.json" "['reason']")"
load_marker="$(read_json_field "${ROOT_DIR}/docs/phase32_remediation_integrated_load_v1.json" "['marker']")"

recon_status="$(read_json_field "${ROOT_DIR}/docs/phase32_remediation_reconciliation_v1.json" "['status']")"
recon_reason="$(read_json_field "${ROOT_DIR}/docs/phase32_remediation_reconciliation_v1.json" "['reason']")"
recon_marker="$(read_json_field "${ROOT_DIR}/docs/phase32_remediation_reconciliation_v1.json" "['marker']")"

lat_status="$(read_json_field "${ROOT_DIR}/docs/phase32_remediation_latency_v1.json" "['status']")"
lat_reason="$(read_json_field "${ROOT_DIR}/docs/phase32_remediation_latency_v1.json" "['reason']")"
lat_marker="$(read_json_field "${ROOT_DIR}/docs/phase32_remediation_latency_v1.json" "['marker']")"

blockers=()
[[ "${readiness_status}" == "blocked" ]] && blockers+=("graph_env_not_ready:${readiness_reason}")
[[ "${load_status}" == "blocked" ]] && blockers+=("integrated_100_event_load_blocked:${load_reason}")
[[ "${recon_status}" == "blocked" ]] && blockers+=("reconciliation_blocked:${recon_reason}")
[[ "${lat_status}" == "blocked" || "${lat_status}" == "problematic" ]] && blockers+=("latency_blocked_or_problematic:${lat_reason}")

overall="pass_with_notes"
recommendation="go_to_phase33"
if (( ${#blockers[@]} > 0 )); then
  overall="blocked"
  recommendation="stay_in_phase32"
fi

blockers_json="$(printf '%s\n' "${blockers[@]}" | python -c 'import json,sys; print(json.dumps([x.strip() for x in sys.stdin if x.strip()]))')"
marker="KV_EXIT_REMEDIATION_V1|status=${overall}|blockers=${#blockers[@]}|ts=${NOW_UTC}"

cat > "${REPORT_JSON}" <<JSON
{
  "version": "phase32_exit_remediation_report_v1",
  "generated_at": "${NOW_UTC}",
  "overall_status": "${overall}",
  "marker": "${marker}",
  "graph_env_readiness": {
    "status": "${readiness_status}",
    "reason": "${readiness_reason}",
    "marker": "${readiness_marker}"
  },
  "integrated_100_event_load": {
    "status": "${load_status}",
    "reason": "${load_reason}",
    "marker": "${load_marker}"
  },
  "reconciliation_check": {
    "status": "${recon_status}",
    "reason": "${recon_reason}",
    "marker": "${recon_marker}"
  },
  "latency_benchmark": {
    "status": "${lat_status}",
    "reason": "${lat_reason}",
    "marker": "${lat_marker}"
  },
  "blockers_resolved": $([[ ${#blockers[@]} -eq 0 ]] && echo true || echo false),
  "remaining_blockers": ${blockers_json},
  "recommendation": "${recommendation}"
}
JSON

cat > "${REPORT_MD}" <<MD
# Phase 32 Remediation Report v1

Generated at: ${NOW_UTC}

Marker: \
\`${marker}\`

- graph_env_readiness: **${readiness_status}** (${readiness_reason})
- integrated_100_event_load: **${load_status}** (${load_reason})
- reconciliation_check: **${recon_status}** (${recon_reason})
- latency_benchmark: **${lat_status}** (${lat_reason})
- blockers_resolved: **$([[ ${#blockers[@]} -eq 0 ]] && echo true || echo false)**
- recommendation: **${recommendation}**

## Remaining blockers
$(printf '%s
' "${blockers[@]}" | sed 's/^/- /')

## Stage markers
- ${readiness_marker}
- ${load_marker}
- ${recon_marker}
- ${lat_marker}
MD

echo "${marker}"
