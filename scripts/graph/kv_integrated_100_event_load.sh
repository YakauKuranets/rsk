#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_JSON="${ROOT_DIR}/docs/phase32_remediation_integrated_load_v1.json"
PRIMARY_LEDGER_JSON="${ROOT_DIR}/docs/phase32_remediation_primary_ledger_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
TOTAL_EVENTS="${1:-100}"
BATCH_ID="rem_batch_${NOW_UTC//[:TZ-]/}"

# shellcheck source=scripts/graph/_kv_cypher.sh
source "${ROOT_DIR}/scripts/graph/_kv_cypher.sh"

kv_load_env

queued=0
written=0
skipped=0
errored=0
status="blocked"
reason="not_executed"

categories=(capability_runs stream_findings session_cookie_findings archive_search_findings device_service_metadata)
declare -A primary_counts
for c in "${categories[@]}"; do primary_counts["$c"]=0; done

base_reason="$(kv_env_reason)"
if [[ "${base_reason}" != "ready" ]]; then
  skipped="${TOTAL_EVENTS}"
  reason="${base_reason}"
  for ((i=0;i<TOTAL_EVENTS;i++)); do
    cat_idx=$((i % ${#categories[@]}))
    cat="${categories[$cat_idx]}"
    primary_counts["$cat"]=$((primary_counts["$cat"] + 1))
  done
else
  for ((i=0;i<TOTAL_EVENTS;i++)); do
    cat_idx=$((i % ${#categories[@]}))
    cat="${categories[$cat_idx]}"
    primary_counts["$cat"]=$((primary_counts["$cat"] + 1))
    queued=$((queued + 1))

    run_id="${BATCH_ID}_${i}"
    finding_id="rem_finding_${cat}_${i}"
    query="MERGE (r:Run {run_id:'${run_id}'}) SET r.created_at=timestamp(), r.shadow_mode=true, r.projection_type='${cat}', r.run_batch_id='${BATCH_ID}' MERGE (c:Capability {capability_key:'remediation_loader'}) MERGE (r)-[:USED_CAPABILITY]->(c) MERGE (f:Finding {finding_id:'${finding_id}'}) SET f.summary='remediation synthetic event', f.severity='info' MERGE (r)-[:PRODUCED_FINDING]->(f)"

    if kv_run_cypher "${query}" >/dev/null 2>&1; then
      written=$((written + 1))
    else
      errored=$((errored + 1))
    fi
  done

  status="pass_with_notes"
  reason="writes_completed"
  if [[ "${written}" -eq 0 ]]; then
    status="blocked"
    reason="shadow_writes_failed"
  elif [[ "${errored}" -gt 0 ]]; then
    reason="partial_shadow_write_errors"
  fi
fi

marker="KV_EXIT_REMEDIATION_V1|stage=integrated_load|status=${status}|reason=${reason}|batchId=${BATCH_ID}|events=${TOTAL_EVENTS}|written=${written}|errored=${errored}"

cat > "${PRIMARY_LEDGER_JSON}" <<JSON
{
  "version": "phase32_remediation_primary_ledger_v1",
  "generated_at": "${NOW_UTC}",
  "batch_id": "${BATCH_ID}",
  "total_events": ${TOTAL_EVENTS},
  "counts": {
    "capability_runs": ${primary_counts[capability_runs]},
    "stream_findings": ${primary_counts[stream_findings]},
    "session_cookie_findings": ${primary_counts[session_cookie_findings]},
    "archive_search_findings": ${primary_counts[archive_search_findings]},
    "device_service_metadata": ${primary_counts[device_service_metadata]}
  }
}
JSON

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase32_remediation_integrated_load_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${status}",
  "reason": "${reason}",
  "batch_id": "${BATCH_ID}",
  "total_events": ${TOTAL_EVENTS},
  "queued": ${queued},
  "written": ${written},
  "skipped": ${skipped},
  "errored": ${errored},
  "marker": "${marker}"
}
JSON

echo "${marker}"
