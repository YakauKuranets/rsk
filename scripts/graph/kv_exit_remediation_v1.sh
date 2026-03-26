#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_JSON="${ROOT_DIR}/docs/phase32_exit_remediation_report_v1.json"
REPORT_MD="${ROOT_DIR}/docs/phase32_exit_remediation_report_v1.md"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

LOAD_MARKER="$("${ROOT_DIR}/scripts/graph/kv_integrated_100_event_load.sh" 100)"
RECON_MARKER="$("${ROOT_DIR}/scripts/graph/kv_reconciliation_check_v1.sh")"
LAT_MARKER="$("${ROOT_DIR}/scripts/graph/kv_latency_benchmark_v1.sh" 100)"

load_status=$(python - <<PY
import json
print(json.load(open('${ROOT_DIR}/docs/phase32_remediation_integrated_load_v1.json'))['marker'].split('status=')[1].split('|')[0])
PY
)
recon_status=$(python - <<PY
import json
print(json.load(open('${ROOT_DIR}/docs/phase32_remediation_reconciliation_v1.json'))['status'])
PY
)
lat_status=$(python - <<PY
import json
print(json.load(open('${ROOT_DIR}/docs/phase32_remediation_latency_v1.json'))['status'])
PY
)

blockers=()
[[ "${load_status}" == "blocked" ]] && blockers+=("integrated_100_event_load_blocked")
[[ "${recon_status}" == "blocked" ]] && blockers+=("reconciliation_blocked")
[[ "${lat_status}" == "blocked" || "${lat_status}" == "problematic" ]] && blockers+=("latency_blocked_or_problematic")

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
  "integrated_100_event_load": "${load_status}",
  "reconciliation_check": "${recon_status}",
  "latency_benchmark": "${lat_status}",
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

- integrated_100_event_load: **${load_status}**
- reconciliation_check: **${recon_status}**
- latency_benchmark: **${lat_status}**
- blockers_resolved: **$([[ ${#blockers[@]} -eq 0 ]] && echo true || echo false)**
- recommendation: **${recommendation}**

## Remaining blockers
$(printf '%s
' "${blockers[@]}" | sed 's/^/- /')

## Stage markers
- ${LOAD_MARKER}
- ${RECON_MARKER}
- ${LAT_MARKER}
MD

echo "${marker}"
