#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${ROOT_DIR}/infra/neo4j-shadow/.env"
OUT_JSON="${ROOT_DIR}/docs/phase32_remediation_integrated_load_v1.json"
PRIMARY_LEDGER_JSON="${ROOT_DIR}/docs/phase32_remediation_primary_ledger_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
TOTAL_EVENTS="${1:-100}"

queued=0
written=0
skipped=0
errored=0

categories=(capability_runs stream_findings session_cookie_findings archive_search_findings device_service_metadata)
declare -A primary_counts
for c in "${categories[@]}"; do primary_counts["$c"]=0; done

if [[ ! -f "${ENV_FILE}" ]] || ! command -v cypher-shell >/dev/null 2>&1; then
  skipped="${TOTAL_EVENTS}"
  for ((i=0;i<TOTAL_EVENTS;i++)); do
    cat_idx=$((i % ${#categories[@]}))
    cat="${categories[$cat_idx]}"
    primary_counts["$cat"]=$((primary_counts["$cat"] + 1))
  done
  marker="KV_EXIT_REMEDIATION_V1|stage=integrated_load|status=blocked|reason=missing_env_or_cypher_shell|events=${TOTAL_EVENTS}"
else
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
  BOLT_URL="bolt://localhost:${NEO4J_BOLT_PORT:-7687}"

  for ((i=0;i<TOTAL_EVENTS;i++)); do
    cat_idx=$((i % ${#categories[@]}))
    cat="${categories[$cat_idx]}"
    primary_counts["$cat"]=$((primary_counts["$cat"] + 1))
    queued=$((queued + 1))

    run_id="rem_load_${NOW_UTC//[:TZ-]/}_${i}"
    finding_id="rem_finding_${cat}_${i}"

    query="MERGE (r:Run {run_id:'${run_id}'}) SET r.created_at=timestamp(), r.shadow_mode=true, r.projection_type='${cat}' MERGE (c:Capability {capability_key:'remediation_loader'}) MERGE (r)-[:USED_CAPABILITY]->(c) MERGE (f:Finding {finding_id:'${finding_id}'}) SET f.summary='remediation synthetic event', f.severity='info' MERGE (r)-[:PRODUCED_FINDING]->(f)"

    if cypher-shell -a "${BOLT_URL}" -u "${NEO4J_USER:-neo4j}" -p "${NEO4J_PASSWORD:-}" -d "${NEO4J_DATABASE:-neo4j}" "${query}" >/dev/null 2>&1; then
      written=$((written + 1))
    else
      errored=$((errored + 1))
    fi
  done

  status="pass_with_notes"
  if [[ "${written}" -eq 0 ]]; then
    status="blocked"
  fi
  marker="KV_EXIT_REMEDIATION_V1|stage=integrated_load|status=${status}|events=${TOTAL_EVENTS}|written=${written}|errored=${errored}"
fi

cat > "${PRIMARY_LEDGER_JSON}" <<JSON
{
  "version": "phase32_remediation_primary_ledger_v1",
  "generated_at": "${NOW_UTC}",
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
  "total_events": ${TOTAL_EVENTS},
  "queued": ${queued},
  "written": ${written},
  "skipped": ${skipped},
  "errored": ${errored},
  "marker": "${marker}"
}
JSON

echo "${marker}"
