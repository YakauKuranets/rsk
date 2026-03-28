#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PHASE32_EXIT_JSON="${ROOT_DIR}/docs/phase32_exit_remediation_report_v1.json"
SHADOW_VALIDATION_JSON="${ROOT_DIR}/docs/phase33_shadow_validation_v1.json"
BATCH_AUDIT_JSON="${ROOT_DIR}/docs/phase33_shadow_batch_field_audit_v1.json"
LEGACY_GOV_JSON="${ROOT_DIR}/docs/phase33_legacy_drift_governance_v1.json"
BACKFILL_JSON="${ROOT_DIR}/docs/phase33_shadow_batch_field_backfill_v1.json"
OUT_JSON="${ROOT_DIR}/docs/phase33_operator_readiness_v1.json"
OUT_MD="${ROOT_DIR}/docs/phase33_operator_readiness_v1.md"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

read_json_field() {
  local file="$1"
  local path="$2"
  python - <<PY
import json
from pathlib import Path
p=Path('${file}')
if not p.exists():
    print('')
    raise SystemExit(0)
obj=json.loads(p.read_text())
cur=obj
for k in '${path}'.split('.'):
    if isinstance(cur, dict) and k in cur:
        cur=cur[k]
    else:
        print('')
        raise SystemExit(0)
if isinstance(cur, bool):
    print('true' if cur else 'false')
elif cur is None:
    print('')
else:
    print(str(cur))
PY
}

artifact_present() {
  [[ -f "$1" ]] && echo "true" || echo "false"
}

phase32_present="$(artifact_present "${PHASE32_EXIT_JSON}")"
shadow_validation_present="$(artifact_present "${SHADOW_VALIDATION_JSON}")"
batch_audit_present="$(artifact_present "${BATCH_AUDIT_JSON}")"
legacy_gov_present="$(artifact_present "${LEGACY_GOV_JSON}")"
backfill_present="$(artifact_present "${BACKFILL_JSON}")"

phase32_overall="$(read_json_field "${PHASE32_EXIT_JSON}" "overall_status")"
phase32_graph_status="$(read_json_field "${PHASE32_EXIT_JSON}" "graph_env_readiness.status")"
phase32_graph_reason="$(read_json_field "${PHASE32_EXIT_JSON}" "graph_env_readiness.reason")"
phase32_reco="$(read_json_field "${PHASE32_EXIT_JSON}" "recommendation")"

shadow_validation_status="$(read_json_field "${SHADOW_VALIDATION_JSON}" "status")"
shadow_validation_reason="$(read_json_field "${SHADOW_VALIDATION_JSON}" "reason")"

batch_audit_status="$(read_json_field "${BATCH_AUDIT_JSON}" "status")"
batch_audit_reason="$(read_json_field "${BATCH_AUDIT_JSON}" "reason")"

legacy_gov_status="$(read_json_field "${LEGACY_GOV_JSON}" "status")"
legacy_gov_reason="$(read_json_field "${LEGACY_GOV_JSON}" "reason")"

status="pass"
reason="operator_ready"

if [[ "${phase32_present}" != "true" || "${shadow_validation_present}" != "true" || "${batch_audit_present}" != "true" || "${legacy_gov_present}" != "true" ]]; then
  status="blocked"
  reason="validation_artifact_missing"
elif [[ "${phase32_overall}" == "blocked" ]]; then
  status="blocked"
  reason="remediation_not_healthy"
elif [[ "${shadow_validation_status}" == "blocked" ]]; then
  status="blocked"
  reason="graph_validation_blocked"
elif [[ "${legacy_gov_status}" == "blocked" ]]; then
  status="blocked"
  reason="legacy_drift_not_acceptable"
elif [[ "${phase32_overall}" == "pass_with_notes" || "${shadow_validation_status}" == "pass_with_notes" || "${legacy_gov_status}" == "pass_with_notes" || "${batch_audit_status}" == "pass_with_notes" ]]; then
  status="pass_with_notes"
  reason="operator_ready_with_notes"
fi

marker="KV_SHADOW_OPERATOR_READINESS_V1|status=${status}|reason=${reason}"

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_operator_readiness_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${status}",
  "reason": "${reason}",
  "source_artifacts": {
    "phase32_exit_remediation": {"path": "docs/phase32_exit_remediation_report_v1.json", "present": ${phase32_present}},
    "phase33_shadow_validation": {"path": "docs/phase33_shadow_validation_v1.json", "present": ${shadow_validation_present}},
    "phase33_shadow_batch_field_audit": {"path": "docs/phase33_shadow_batch_field_audit_v1.json", "present": ${batch_audit_present}},
    "phase33_legacy_drift_governance": {"path": "docs/phase33_legacy_drift_governance_v1.json", "present": ${legacy_gov_present}},
    "phase33_shadow_batch_field_backfill": {"path": "docs/phase33_shadow_batch_field_backfill_v1.json", "present": ${backfill_present}}
  },
  "sections": {
    "graph_runtime_health": {
      "status": "${phase32_graph_status}",
      "reason": "${phase32_graph_reason}"
    },
    "remediation_health": {
      "status": "${phase32_overall}",
      "reason": "${phase32_reco}"
    },
    "shadow_validation_health": {
      "status": "${shadow_validation_status}",
      "reason": "${shadow_validation_reason}"
    },
    "legacy_drift_health": {
      "status": "${legacy_gov_status}",
      "reason": "${legacy_gov_reason}"
    },
    "operator_readiness": {
      "status": "${status}",
      "reason": "${reason}"
    }
  },
  "marker": "${marker}"
}
JSON

cat > "${OUT_MD}" <<MD
# Phase 33 Operator Readiness v1

Generated at: ${NOW_UTC}

Marker: \`${marker}\`

- status: **${status}**
- reason: **${reason}**

## Source artifacts
- phase32_exit_remediation: ${phase32_present}
- phase33_shadow_validation: ${shadow_validation_present}
- phase33_shadow_batch_field_audit: ${batch_audit_present}
- phase33_legacy_drift_governance: ${legacy_gov_present}
- phase33_shadow_batch_field_backfill (optional): ${backfill_present}

## Sections
- graph_runtime_health: ${phase32_graph_status} (${phase32_graph_reason})
- remediation_health: ${phase32_overall} (${phase32_reco})
- shadow_validation_health: ${shadow_validation_status} (${shadow_validation_reason})
- legacy_drift_health: ${legacy_gov_status} (${legacy_gov_reason})
- operator_readiness: ${status} (${reason})
MD

echo "${marker}"
