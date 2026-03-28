#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LEDGER_JSON="${ROOT_DIR}/docs/phase32_remediation_primary_ledger_v1.json"
OUT_JSON="${ROOT_DIR}/docs/phase33_shadow_validation_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

# shellcheck source=scripts/graph/_kv_cypher.sh
source "${ROOT_DIR}/scripts/graph/_kv_cypher.sh"
kv_load_env

emit_report() {
  local status="$1"
  local reason="$2"
  local batch_id="$3"
  local run_count="$4"
  local capability_links="$5"
  local finding_links="$6"
  local orphan_runs="$7"

  local marker="KV_SHADOW_VALIDATION_V1|status=${status}|reason=${reason}|batch_id=${batch_id}"

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
    "orphan_runs": ${orphan_runs}
  },
  "marker": "${marker}"
}
JSON

  echo "${marker}"
}

if [[ ! -f "${LEDGER_JSON}" ]]; then
  emit_report "blocked" "missing_primary_ledger" "" 0 0 0 0
  exit 0
fi

batch_id="$(python - <<PY
import json
with open('${LEDGER_JSON}') as f:
    d = json.load(f)
print(d.get('batch_id', ''))
PY
)"

if [[ -z "${batch_id}" ]]; then
  emit_report "blocked" "missing_batch_id" "" 0 0 0 0
  exit 0
fi

read_count() {
  local query="$1"
  local value
  value="$(kv_run_cypher "${query}" 2>/dev/null | tail -n1 | tr -d '\r')"
  if [[ -z "${value}" ]]; then
    echo 0
  else
    echo "${value}"
  fi
}

run_count="$(read_count "MATCH (r:Run {batch_id:'${batch_id}'}) RETURN count(r)")"
capability_links="$(read_count "MATCH (r:Run {batch_id:'${batch_id}'})-[:USED_CAPABILITY]->(c:Capability) RETURN count(r)")"
finding_links="$(read_count "MATCH (r:Run {batch_id:'${batch_id}'})-[:PRODUCED_FINDING]->(f:Finding) RETURN count(f)")"
orphan_runs="$(read_count "MATCH (r:Run {batch_id:'${batch_id}'}) WHERE NOT (r)-[:PRODUCED_FINDING]->() RETURN count(r)")"

status="pass"
reason="graph_consistent"

if (( run_count <= 0 || finding_links <= 0 || orphan_runs > 0 )); then
  status="blocked"
  reason="graph_integrity_failure"
elif (( capability_links != run_count )); then
  status="pass_with_notes"
  reason="minor_graph_inconsistency"
fi

emit_report "${status}" "${reason}" "${batch_id}" "${run_count}" "${capability_links}" "${finding_links}" "${orphan_runs}"
