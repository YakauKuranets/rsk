#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LEDGER_JSON="${ROOT_DIR}/docs/phase32_remediation_primary_ledger_v1.json"
INTEGRATED_LOAD_JSON="${ROOT_DIR}/docs/phase32_remediation_integrated_load_v1.json"
OUT_JSON="${ROOT_DIR}/docs/phase33_shadow_validation_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

# shellcheck source=scripts/graph/_kv_cypher.sh
source "${ROOT_DIR}/scripts/graph/_kv_cypher.sh"
kv_load_env
base_reason="$(kv_env_reason)"

if [[ ! -f "${LEDGER_JSON}" ]]; then
  status="blocked"
  reason="missing_primary_ledger"
  marker="KV_SHADOW_VALIDATION_V1|status=${status}|reason=${reason}|batch_id="
  cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_shadow_validation_v1",
  "generated_at": "${NOW_UTC}",
  "batch_id": "",
  "status": "${status}",
  "reason": "${reason}",
  "checks": {
    "run_count": 0,
    "capability_links": 0,
    "finding_links": 0,
    "orphan_runs": 0
  },
  "marker": "${marker}"
}
JSON
  echo "${marker}"
  exit 0
fi

batch_id="$(python - <<PY
import json
with open('${LEDGER_JSON}') as f:
    d=json.load(f)
print(d.get('batch_id',''))
PY
)"

if [[ -z "${batch_id}" ]]; then
  status="blocked"
  reason="missing_batch_id"
  marker="KV_SHADOW_VALIDATION_V1|status=${status}|reason=${reason}|batch_id="
  cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_shadow_validation_v1",
  "generated_at": "${NOW_UTC}",
  "batch_id": "",
  "status": "${status}",
  "reason": "${reason}",
  "checks": {
    "run_count": 0,
    "capability_links": 0,
    "finding_links": 0,
    "orphan_runs": 0
  },
  "marker": "${marker}"
}
JSON
  echo "${marker}"
  exit 0
fi

if [[ "${base_reason}" != "ready" ]]; then
  status="blocked"
  reason="${base_reason}"
  marker="KV_SHADOW_VALIDATION_V1|status=${status}|reason=${reason}|batch_id=${batch_id}"
  cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_shadow_validation_v1",
  "generated_at": "${NOW_UTC}",
  "batch_id": "${batch_id}",
  "status": "${status}",
  "reason": "${reason}",
  "checks": {
    "run_count": 0,
    "capability_links": 0,
    "finding_links": 0,
    "orphan_runs": 0
  },
  "marker": "${marker}"
}
JSON
  echo "${marker}"
  exit 0
fi

load_status=""
load_reason=""
load_batch_id=""
load_written="-1"
if [[ -f "${INTEGRATED_LOAD_JSON}" ]]; then
  read -r load_status load_reason load_batch_id load_written <<EOFINLOAD
$(python - <<PY
import json
with open('${INTEGRATED_LOAD_JSON}') as f:
    d=json.load(f)
print(d.get('status',''), d.get('reason',''), d.get('batch_id',''), d.get('written',-1))
PY
)
EOFINLOAD
fi

if [[ -n "${load_batch_id}" && "${load_batch_id}" != "${batch_id}" ]]; then
  status="blocked"
  reason="batch_id_mismatch_between_ledger_and_integrated_load"
  marker="KV_SHADOW_VALIDATION_V1|status=${status}|reason=${reason}|batch_id=${batch_id}"
  cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_shadow_validation_v1",
  "generated_at": "${NOW_UTC}",
  "batch_id": "${batch_id}",
  "status": "${status}",
  "reason": "${reason}",
  "checks": {
    "run_count": 0,
    "capability_links": 0,
    "finding_links": 0,
    "orphan_runs": 0
  },
  "details": ["ledger_and_integrated_load_batch_id_do_not_match"],
  "integrated_load": {
    "status": "${load_status}",
    "reason": "${load_reason}",
    "batch_id": "${load_batch_id}",
    "written": ${load_written}
  },
  "marker": "${marker}"
}
JSON
  echo "${marker}"
  exit 0
fi

if [[ "${load_written}" == "0" ]]; then
  status="blocked"
  reason="no_shadow_writes_recorded_for_batch"
  marker="KV_SHADOW_VALIDATION_V1|status=${status}|reason=${reason}|batch_id=${batch_id}"
  cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_shadow_validation_v1",
  "generated_at": "${NOW_UTC}",
  "batch_id": "${batch_id}",
  "status": "${status}",
  "reason": "${reason}",
  "checks": {
    "run_count": 0,
    "capability_links": 0,
    "finding_links": 0,
    "orphan_runs": 0
  },
  "details": ["integrated_load_reported_zero_written_records"],
  "integrated_load": {
    "status": "${load_status}",
    "reason": "${load_reason}",
    "batch_id": "${load_batch_id}",
    "written": ${load_written}
  },
  "marker": "${marker}"
}
JSON
  echo "${marker}"
  exit 0
fi

run_count_q="MATCH (r:Run {batch_id:'${batch_id}'}) RETURN count(r)"
cap_links_q="MATCH (r:Run {batch_id:'${batch_id}'})-[:USED_CAPABILITY]->(c:Capability) RETURN count(r)"
finding_links_q="MATCH (r:Run {batch_id:'${batch_id}'})-[:PRODUCED_FINDING]->(f:Finding) RETURN count(f)"
orphan_runs_q="MATCH (r:Run {batch_id:'${batch_id}'}) WHERE NOT (r)-[:PRODUCED_FINDING]->() RETURN count(r)"
global_runs_q="MATCH (r:Run) RETURN count(r)"

read_count() {
  local q="$1"
  local val
  val="$(kv_run_cypher "${q}" 2>/dev/null | tail -n1 | tr -d '\r')"
  if [[ -z "${val}" ]]; then
    echo 0
  else
    echo "${val}"
  fi
}

run_count="$(read_count "${run_count_q}")"
capability_links="$(read_count "${cap_links_q}")"
finding_links="$(read_count "${finding_links_q}")"
orphan_runs="$(read_count "${orphan_runs_q}")"
global_runs="$(read_count "${global_runs_q}")"

status="pass"
reason="graph_consistent"
details=()

if (( run_count <= 0 || capability_links <= 0 || finding_links <= 0 || orphan_runs > 0 )); then
  status="blocked"
  reason="graph_integrity_failure"
  (( run_count <= 0 )) && details+=("no_runs_for_batch_id")
  if (( run_count <= 0 && global_runs > 0 )); then
    details+=("global_runs_exist_but_not_for_requested_batch_id")
  fi
  (( capability_links <= 0 )) && details+=("missing_capability_links")
  (( finding_links <= 0 )) && details+=("missing_finding_links")
  (( orphan_runs > 0 )) && details+=("orphan_runs_detected")
elif (( capability_links != run_count || finding_links < run_count )); then
  status="pass_with_notes"
  reason="minor_graph_inconsistency"
  (( capability_links != run_count )) && details+=("capability_links_not_equal_run_count")
  (( finding_links < run_count )) && details+=("finding_links_less_than_run_count")
fi

if [[ "${status}" == "blocked" && ${#details[@]} -gt 0 ]]; then
  reason="${details[0]}"
fi

marker="KV_SHADOW_VALIDATION_V1|status=${status}|reason=${reason}|batch_id=${batch_id}"
if [[ ${#details[@]} -gt 0 ]]; then
  details_json="$(printf '%s\n' "${details[@]}" | python -c 'import json,sys; print(json.dumps([x.strip() for x in sys.stdin if x.strip()]))')"
else
  details_json="[]"
fi

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_shadow_validation_v1",
  "generated_at": "${NOW_UTC}",
  "batch_id": "${batch_id}",
  "status": "${status}",
  "reason": "${reason}",
  "checks": {
    "run_count": ${run_count},
    "capability_links": ${capability_links},
    "finding_links": ${finding_links},
    "orphan_runs": ${orphan_runs},
    "global_runs": ${global_runs}
  },
  "details": ${details_json},
  "marker": "${marker}"
}
JSON

echo "${marker}"
