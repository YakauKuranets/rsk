#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"
LEDGER_JSON="${ROOT_DIR}/docs/phase32_remediation_primary_ledger_v1.json"
OUT_JSON="${ROOT_DIR}/docs/phase32_remediation_reconciliation_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

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
for c in capability_runs stream_findings session_cookie_findings archive_search_findings device_service_metadata; do
  primary[$c]="$(read_primary "$c")"
done

declare -A graph
declare -A diff
status="pass_with_notes"

if [[ -f "${ENV_FILE}" ]] && command -v cypher-shell >/dev/null 2>&1; then
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
  BOLT_URL="bolt://localhost:${NEO4J_BOLT_PORT:-7687}"

  for c in capability_runs stream_findings session_cookie_findings archive_search_findings device_service_metadata; do
    q="MATCH (r:Run {projection_type:'${c}'}) RETURN count(r)"
    v="$(cypher-shell -a "${BOLT_URL}" -u "${NEO4J_USER:-neo4j}" -p "${NEO4J_PASSWORD:-}" -d "${NEO4J_DATABASE:-neo4j}" --format plain "${q}" 2>/dev/null | tail -n1 | tr -d '\r' || echo 0)"
    [[ -z "${v}" ]] && v=0
    graph[$c]="${v}"
    diff[$c]=$(( ${graph[$c]} - ${primary[$c]} ))
  done
else
  status="blocked"
  reason="missing_env_or_cypher_shell"
  for c in capability_runs stream_findings session_cookie_findings archive_search_findings device_service_metadata; do
    graph[$c]=0
    diff[$c]=$((0 - ${primary[$c]}))
  done
fi

if [[ "${reason}" == "within_tolerance" ]]; then
  for c in capability_runs stream_findings session_cookie_findings archive_search_findings device_service_metadata; do
    d=${diff[$c]}
    if (( d < -5 || d > 5 )); then
      status="blocked"
      reason="count_drift_out_of_tolerance"
    fi
  done
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
