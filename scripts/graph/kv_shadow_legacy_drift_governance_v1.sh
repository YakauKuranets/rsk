#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
AUDIT_JSON="${ROOT_DIR}/docs/phase33_shadow_batch_field_audit_v1.json"
BACKFILL_JSON="${ROOT_DIR}/docs/phase33_shadow_batch_field_backfill_v1.json"
OUT_JSON="${ROOT_DIR}/docs/phase33_legacy_drift_governance_v1.json"
OUT_MD="${ROOT_DIR}/docs/phase33_legacy_drift_governance_v1.md"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

THRESHOLD_RUN_BATCH_ONLY_WARN="${KV_LEGACY_RUN_BATCH_ONLY_WARN:-5}"
THRESHOLD_RUN_BATCH_ONLY_BLOCK="${KV_LEGACY_RUN_BATCH_ONLY_BLOCK:-20}"
THRESHOLD_NEITHER_BLOCK="${KV_LEGACY_NEITHER_BLOCK:-1}"
THRESHOLD_CANONICAL_COVERAGE_MIN="${KV_CANONICAL_COVERAGE_MIN:-0.95}"

if [[ ! -f "${AUDIT_JSON}" ]]; then
  status="blocked"
  reason="missing_audit_artifact"
  marker="KV_SHADOW_LEGACY_GOVERNANCE_V1|status=${status}|reason=${reason}"

  cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_legacy_drift_governance_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${status}",
  "reason": "${reason}",
  "thresholds": {
    "run_batch_id_only_warn": ${THRESHOLD_RUN_BATCH_ONLY_WARN},
    "run_batch_id_only_block": ${THRESHOLD_RUN_BATCH_ONLY_BLOCK},
    "neither_block": ${THRESHOLD_NEITHER_BLOCK},
    "canonical_coverage_min": ${THRESHOLD_CANONICAL_COVERAGE_MIN}
  },
  "observed": {
    "batch_id_only": 0,
    "run_batch_id_only": 0,
    "both": 0,
    "neither": 0,
    "total_runs": 0,
    "canonical_coverage": 0
  },
  "backfill": {
    "present": false,
    "legacy_only_before": null,
    "updated": null,
    "legacy_only_after": null
  },
  "marker": "${marker}"
}
JSON

  cat > "${OUT_MD}" <<MD
# Phase 33 Legacy Drift Governance v1

Generated at: ${NOW_UTC}

Marker: \`${marker}\`

- status: **${status}**
- reason: **${reason}**
- note: audit artifact is missing, governance cannot evaluate drift.
MD

  echo "${marker}"
  exit 0
fi

read -r batch_only run_batch_only both neither <<EOFCOUNTS
$(python - <<PY
import json
with open('${AUDIT_JSON}') as f:
    d=json.load(f)
c=d.get('counts',{})
print(c.get('batch_id_only',0), c.get('run_batch_id_only',0), c.get('both',0), c.get('neither',0))
PY
)
EOFCOUNTS

total_runs=$((batch_only + run_batch_only + both + neither))
canonical_present=$((batch_only + both))

canonical_coverage="0"
if (( total_runs > 0 )); then
  canonical_coverage="$(python - <<PY
print(${canonical_present}/${total_runs})
PY
)"
fi

status="pass"
reason="legacy_drift_within_threshold"

if (( neither >= THRESHOLD_NEITHER_BLOCK )); then
  status="blocked"
  reason="orphan_batch_fields_detected"
elif (( run_batch_only >= THRESHOLD_RUN_BATCH_ONLY_BLOCK )); then
  status="blocked"
  reason="legacy_drift_exceeds_threshold"
else
  canonical_below_min="$(python - <<PY
print(1 if float('${canonical_coverage}') < float('${THRESHOLD_CANONICAL_COVERAGE_MIN}') else 0)
PY
)"
  if [[ "${canonical_below_min}" == "1" ]]; then
    status="blocked"
    reason="legacy_drift_exceeds_threshold"
  elif (( run_batch_only > THRESHOLD_RUN_BATCH_ONLY_WARN )); then
    status="pass_with_notes"
    reason="minor_legacy_drift_remaining"
  fi
fi

backfill_present=false
legacy_before=null
updated=null
legacy_after=null

if [[ -f "${BACKFILL_JSON}" ]]; then
  backfill_present=true
  read -r legacy_before updated legacy_after <<EOFBF
$(python - <<PY
import json
with open('${BACKFILL_JSON}') as f:
    d=json.load(f)
c=d.get('counts',{})
print(c.get('legacy_only_before','null'), c.get('updated','null'), c.get('legacy_only_after','null'))
PY
)
EOFBF
fi

marker="KV_SHADOW_LEGACY_GOVERNANCE_V1|status=${status}|reason=${reason}"

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_legacy_drift_governance_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${status}",
  "reason": "${reason}",
  "thresholds": {
    "run_batch_id_only_warn": ${THRESHOLD_RUN_BATCH_ONLY_WARN},
    "run_batch_id_only_block": ${THRESHOLD_RUN_BATCH_ONLY_BLOCK},
    "neither_block": ${THRESHOLD_NEITHER_BLOCK},
    "canonical_coverage_min": ${THRESHOLD_CANONICAL_COVERAGE_MIN}
  },
  "observed": {
    "batch_id_only": ${batch_only},
    "run_batch_id_only": ${run_batch_only},
    "both": ${both},
    "neither": ${neither},
    "total_runs": ${total_runs},
    "canonical_coverage": ${canonical_coverage}
  },
  "backfill": {
    "present": ${backfill_present},
    "legacy_only_before": ${legacy_before},
    "updated": ${updated},
    "legacy_only_after": ${legacy_after}
  },
  "marker": "${marker}"
}
JSON

cat > "${OUT_MD}" <<MD
# Phase 33 Legacy Drift Governance v1

Generated at: ${NOW_UTC}

Marker: \`${marker}\`

- status: **${status}**
- reason: **${reason}**

## Thresholds
- run_batch_id_only_warn: ${THRESHOLD_RUN_BATCH_ONLY_WARN}
- run_batch_id_only_block: ${THRESHOLD_RUN_BATCH_ONLY_BLOCK}
- neither_block: ${THRESHOLD_NEITHER_BLOCK}
- canonical_coverage_min: ${THRESHOLD_CANONICAL_COVERAGE_MIN}

## Observed counts
- batch_id_only: ${batch_only}
- run_batch_id_only: ${run_batch_only}
- both: ${both}
- neither: ${neither}
- total_runs: ${total_runs}
- canonical_coverage: ${canonical_coverage}

## Backfill artifact
- present: ${backfill_present}
- legacy_only_before: ${legacy_before}
- updated: ${updated}
- legacy_only_after: ${legacy_after}
MD

echo "${marker}"
