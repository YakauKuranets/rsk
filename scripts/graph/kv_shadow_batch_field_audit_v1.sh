#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_JSON="${ROOT_DIR}/docs/phase33_shadow_batch_field_audit_v1.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

# shellcheck source=scripts/graph/_kv_cypher.sh
source "${ROOT_DIR}/scripts/graph/_kv_cypher.sh"
kv_load_env

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

both_q="MATCH (r:Run) WHERE r.batch_id IS NOT NULL AND r.run_batch_id IS NOT NULL RETURN count(r)"
batch_only_q="MATCH (r:Run) WHERE r.batch_id IS NOT NULL AND r.run_batch_id IS NULL RETURN count(r)"
run_batch_only_q="MATCH (r:Run) WHERE r.batch_id IS NULL AND r.run_batch_id IS NOT NULL RETURN count(r)"
neither_q="MATCH (r:Run) WHERE r.batch_id IS NULL AND r.run_batch_id IS NULL RETURN count(r)"

both_count="$(read_count "${both_q}")"
batch_only_count="$(read_count "${batch_only_q}")"
run_batch_only_count="$(read_count "${run_batch_only_q}")"
neither_count="$(read_count "${neither_q}")"

status="pass"
reason="canonical_batch_id_clean"
if (( run_batch_only_count > 0 || neither_count > 0 )); then
  status="pass_with_notes"
  reason="legacy_field_drift_detected"
fi

marker="KV_SHADOW_BATCH_FIELD_AUDIT_V1|status=${status}|reason=${reason}|runBatchOnly=${run_batch_only_count}|neither=${neither_count}"

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_shadow_batch_field_audit_v1",
  "generated_at": "${NOW_UTC}",
  "canonical_field": "batch_id",
  "legacy_field": "run_batch_id",
  "status": "${status}",
  "reason": "${reason}",
  "counts": {
    "batch_id_only": ${batch_only_count},
    "run_batch_id_only": ${run_batch_only_count},
    "both": ${both_count},
    "neither": ${neither_count}
  },
  "marker": "${marker}"
}
JSON

echo "${marker}"
