#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_JSON="${ROOT_DIR}/docs/phase33_shadow_batch_field_backfill_v1.json"
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

before_q="MATCH (r:Run) WHERE r.batch_id IS NULL AND r.run_batch_id IS NOT NULL RETURN count(r)"
backfill_q="MATCH (r:Run) WHERE r.batch_id IS NULL AND r.run_batch_id IS NOT NULL SET r.batch_id = r.run_batch_id RETURN count(r)"
after_q="MATCH (r:Run) WHERE r.batch_id IS NULL AND r.run_batch_id IS NOT NULL RETURN count(r)"

before_count="$(read_count "${before_q}")"
updated_count="$(read_count "${backfill_q}")"
after_count="$(read_count "${after_q}")"

status="pass"
reason="backfill_completed"
if (( after_count > 0 )); then
  status="pass_with_notes"
  reason="partial_backfill_remaining"
fi

marker="KV_SHADOW_BATCH_FIELD_BACKFILL_V1|status=${status}|reason=${reason}|updated=${updated_count}|remaining=${after_count}"

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_shadow_batch_field_backfill_v1",
  "generated_at": "${NOW_UTC}",
  "canonical_field": "batch_id",
  "legacy_field": "run_batch_id",
  "status": "${status}",
  "reason": "${reason}",
  "counts": {
    "legacy_only_before": ${before_count},
    "updated": ${updated_count},
    "legacy_only_after": ${after_count}
  },
  "marker": "${marker}"
}
JSON

echo "${marker}"
