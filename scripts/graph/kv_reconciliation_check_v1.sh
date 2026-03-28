#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}" )/../.." && pwd)"
LEDGER_JSON="${ROOT_DIR}/docs/phase32_remediation_primary_ledger_v1.json"
OUT_JSON="${ROOT_DIR}/docs/phase32_remediation_reconciliation_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
WINDOW_MINUTES="${KV_RECON_WINDOW_MINUTES:-10}"
WINDOW_MS=$(( WINDOW_MINUTES * 60 * 1000 ))

# shellcheck source=scripts/graph/_kv_cypher.sh
source "${ROOT_DIR}/scripts/graph/_kv_cypher.sh"
kv_load_env

reason="within_tolerance"

if [[ ! -f "${LEDGER_JSON}" ]]; then
  marker="KV_EXIT_REMEDIATION_V1|stage=reconciliation|status=blocked|reason=missing_primary_ledger"
  cat > "${OUT_JSON}" <<JSON
{
  "version": "phase32_remediation_reconciliation_v1",
  "generated_at": "${NOW_UTC}",
  "status": "blocked",
  "reason": "missing_primary_ledger",
  "marker": "${marker}",
  "counts": {}
}
JSON
  echo "${marker}"
  exit 0
fi

read_primary() { python - <<PY
import json
with open('${LEDGER_JSON}') as f:
    d=json.load(f)
print(d['counts'].get('${1}',0))
PY
}

declare -A primary
declare -A graph
declare -A diff
for c in capability_runs stream_findings session_cookie_findings archive_search_findings device_service_metadata; do
  primary[$c]="$(read_primary "$c")"
  graph[$c]=0
  diff[$c]=0
done

status="pass"
base_reason="$(kv_env_reason)"
used_unscoped_counts=false

if [[ "${base_reason}" == "ready" ]]; then
  for c in capability_runs stream_findings session_cookie_findings archive_search_findings device_service_metadata; do
    scoped_q="MATCH (r:Run {projection_type:'${c}'}) WHERE r.created_at >= timestamp() - ${WINDOW_MS} RETURN count(r)"
    scoped_v="$(kv_run_cypher --format plain "${scoped_q}" 2>/dev/null | tail -n1 | tr -d '\r' || echo 0)"
    [[ -z "${scoped_v}" ]] && scoped_v=0

    if [[ "${scoped_v}" == "0" ]]; then
      total_q="MATCH (r:Run {projection_type:'${c}'}) RETURN count(r)"
      total_v="$(kv_run_cypher --format plain "${total_q}" 2>/dev/null | tail -n1 | tr -d '\r' || echo 0)"
      [[ -z "${total_v}" ]] && total_v=0
      if (( total_v > 0 )); then
        graph[$c]="${total_v}"
        used_unscoped_counts=true
      else
        graph[$c]=0
      fi
    else
      graph[$c]="${scoped_v}"
    fi

    diff[$c]=$(( ${graph[$c]} - ${primary[$c]} ))
  done
else
  status="blocked"
  reason="${base_reason}"
fi

if [[ "${status}" != "blocked" ]]; then
  tolerance_exceeded=false
  exact_match=true

  for c in capability_runs stream_findings session_cookie_findings archive_search_findings device_service_metadata; do
    d=${diff[$c]}
    if (( d != 0 )); then
      exact_match=false
    fi
    if (( d < -5 || d > 5 )); then
      tolerance_exceeded=true
    fi
  done

  if [[ "${tolerance_exceeded}" == "true" ]]; then
    status="blocked"
    reason="count_drift_out_of_tolerance"
  elif [[ "${used_unscoped_counts}" == "true" ]]; then
    status="pass_with_notes"
    reason="unscoped_graph_counts"
  elif [[ "${exact_match}" == "true" ]]; then
    status="pass"
    reason="scoped_counts_match"
  else
    status="pass_with_notes"
    reason="minor_scoped_drift_within_tolerance"
  fi
fi

marker="KV_EXIT_REMEDIATION_V1|stage=reconciliation|status=${status}|reason=${reason}"
cat > "${OUT_JSON}" <<JSON
{
  "version": "phase32_remediation_reconciliation_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${status}",
  "reason": "${reason}",
  "marker": "${marker}",
  "counts": {
    "capability_runs": {"primary": ${primary[capability_runs]}, "graph": ${graph[capability_runs]}, "diff": ${diff[capability_runs]}},
    "stream_findings": {"primary": ${primary[stream_findings]}, "graph": ${graph[stream_findings]}, "diff": ${diff[stream_findings]}},
    "session_cookie_findings": {"primary": ${primary[session_cookie_findings]}, "graph": ${graph[session_cookie_findings]}, "diff": ${diff[session_cookie_findings]}},
    "archive_search_findings": {"primary": ${primary[archive_search_findings]}, "graph": ${graph[archive_search_findings]}, "diff": ${diff[archive_search_findings]}},
    "device_service_metadata": {"primary": ${primary[device_service_metadata]}, "graph": ${graph[device_service_metadata]}, "diff": ${diff[device_service_metadata]}}
  }
}
JSON

echo "${marker}"
