#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PHASE32_JSON="${ROOT_DIR}/docs/phase32_exit_remediation_report_v1.json"
SHADOW_VALIDATION_JSON="${ROOT_DIR}/docs/phase33_shadow_validation_v1.json"
BATCH_AUDIT_JSON="${ROOT_DIR}/docs/phase33_shadow_batch_field_audit_v1.json"
LEGACY_GOV_JSON="${ROOT_DIR}/docs/phase33_legacy_drift_governance_v1.json"
OP_READINESS_JSON="${ROOT_DIR}/docs/phase33_operator_readiness_v1.json"
OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
OUT_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
OUT_MD="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.md"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

artifact_present() { [[ -f "$1" ]] && echo "true" || echo "false"; }

read_json_field() {
  local file="$1"
  local path="$2"
  python - <<PY
import json
from pathlib import Path
p = Path(${file@Q})
if not p.exists():
    print("")
    raise SystemExit(0)
obj = json.loads(p.read_text())
cur = obj
for k in ${path@Q}.split('.'):
    if isinstance(cur, dict) and k in cur:
        cur = cur[k]
    else:
        print("")
        raise SystemExit(0)
if isinstance(cur, bool):
    print("true" if cur else "false")
elif cur is None:
    print("")
else:
    print(str(cur))
PY
}

phase32_present="$(artifact_present "${PHASE32_JSON}")"
shadow_validation_present="$(artifact_present "${SHADOW_VALIDATION_JSON}")"
batch_audit_present="$(artifact_present "${BATCH_AUDIT_JSON}")"
legacy_gov_present="$(artifact_present "${LEGACY_GOV_JSON}")"
op_readiness_present="$(artifact_present "${OP_READINESS_JSON}")"
op_policy_present="$(artifact_present "${OP_POLICY_JSON}")"
handoff_present="$(artifact_present "${HANDOFF_JSON}")"

phase32_status="$(read_json_field "${PHASE32_JSON}" "overall_status")"
shadow_validation_status="$(read_json_field "${SHADOW_VALIDATION_JSON}" "status")"
batch_audit_status="$(read_json_field "${BATCH_AUDIT_JSON}" "status")"
legacy_gov_status="$(read_json_field "${LEGACY_GOV_JSON}" "status")"
op_readiness_status="$(read_json_field "${OP_READINESS_JSON}" "status")"
op_policy_status="$(read_json_field "${OP_POLICY_JSON}" "status")"
op_policy_reason="$(read_json_field "${OP_POLICY_JSON}" "reason")"
handoff_status="$(read_json_field "${HANDOFF_JSON}" "status")"
handoff_reason="$(read_json_field "${HANDOFF_JSON}" "reason")"

baseline_status="baseline_frozen"
reason="baseline_ready"

if [[
  "${phase32_present}" != "true" ||
  "${shadow_validation_present}" != "true" ||
  "${batch_audit_present}" != "true" ||
  "${legacy_gov_present}" != "true" ||
  "${op_readiness_present}" != "true" ||
  "${op_policy_present}" != "true" ||
  "${handoff_present}" != "true"
]]; then
  baseline_status="baseline_freeze_blocked"
  reason="baseline_artifact_missing"
elif [[
  "${phase32_status}" == "blocked" ||
  "${shadow_validation_status}" == "blocked" ||
  "${batch_audit_status}" == "blocked" ||
  "${legacy_gov_status}" == "blocked" ||
  "${op_readiness_status}" == "blocked" ||
  "${op_policy_status}" == "blocked" ||
  "${handoff_status}" == "blocked"
]]; then
  baseline_status="baseline_freeze_blocked"
  reason="baseline_not_operationally_ready"
elif [[
  "${shadow_validation_status}" == "pass_with_notes" ||
  "${batch_audit_status}" == "pass_with_notes" ||
  "${legacy_gov_status}" == "pass_with_notes" ||
  "${op_readiness_status}" == "pass_with_notes" ||
  "${op_policy_status}" == "pass_with_notes" ||
  "${handoff_status}" == "pass_with_notes"
]]; then
  baseline_status="baseline_frozen_with_notes"
  reason="baseline_ready_with_notes"
fi

marker="KV_SHADOW_BASELINE_FREEZE_V1|status=${baseline_status}|reason=${reason}"

carry_note_1="Source verdicts are frozen snapshot values and must be re-frozen only after explicit operational update."
carry_note_2="Operator policy remains the primary gate decision artifact for go/no-go calls."
carry_note_3="Any unresolved notes from readiness/governance/handoff remain active until explicitly cleared."

lim_1="no graph writes"
lim_2="no backfill"
lim_3="no reruns"
lim_4="no UI/runtime/ValidationAgent changes"

entry_1="All required source artifacts must exist and be current for the target session."
entry_2="operator_readiness and operator_policy must not be blocked."
entry_3="handoff pack must be present and consistent with frozen verdicts."
entry_4="No accepted limitation may be violated before next major phase entry."

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_baseline_freeze_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${baseline_status}",
  "baseline_status": "${baseline_status}",
  "reason": "${reason}",
  "marker": "${marker}",
  "source_artifacts": {
    "phase32_exit_remediation": {"path": "docs/phase32_exit_remediation_report_v1.json", "present": ${phase32_present}},
    "phase33_shadow_validation": {"path": "docs/phase33_shadow_validation_v1.json", "present": ${shadow_validation_present}},
    "phase33_shadow_batch_field_audit": {"path": "docs/phase33_shadow_batch_field_audit_v1.json", "present": ${batch_audit_present}},
    "phase33_legacy_drift_governance": {"path": "docs/phase33_legacy_drift_governance_v1.json", "present": ${legacy_gov_present}},
    "phase33_operator_readiness": {"path": "docs/phase33_operator_readiness_v1.json", "present": ${op_readiness_present}},
    "phase33_operator_policy": {"path": "docs/phase33_operator_policy_v1.json", "present": ${op_policy_present}},
    "phase33_handoff_pack": {"path": "docs/phase33_handoff_pack_v1.json", "present": ${handoff_present}}
  },
  "frozen_verdicts": {
    "phase32_status": "${phase32_status}",
    "shadow_validation_status": "${shadow_validation_status}",
    "shadow_batch_field_audit_status": "${batch_audit_status}",
    "legacy_governance_status": "${legacy_gov_status}",
    "operator_readiness_status": "${op_readiness_status}",
    "operator_policy_status": "${op_policy_status}",
    "operator_policy_reason": "${op_policy_reason}",
    "handoff_status": "${handoff_status}",
    "handoff_reason": "${handoff_reason}"
  },
  "notes_to_carry_forward": [
    "${carry_note_1}",
    "${carry_note_2}",
    "${carry_note_3}"
  ],
  "accepted_limitations": [
    "${lim_1}",
    "${lim_2}",
    "${lim_3}",
    "${lim_4}"
  ],
  "next_phase_entry_conditions": [
    "${entry_1}",
    "${entry_2}",
    "${entry_3}",
    "${entry_4}"
  ]
}
JSON

cat > "${OUT_MD}" <<MD
# Phase 33 Baseline Freeze v1

Generated at: ${NOW_UTC}

Marker: \`${marker}\`

- baseline_status: **${baseline_status}**
- reason: **${reason}**

## baseline_status
- official_baseline_frozen: $( [[ "${baseline_status}" == "baseline_frozen" || "${baseline_status}" == "baseline_frozen_with_notes" ]] && echo "true" || echo "false" )
- status: ${baseline_status}
- reason: ${reason}

## source_artifacts
- phase32_exit_remediation: ${phase32_present}
- phase33_shadow_validation: ${shadow_validation_present}
- phase33_shadow_batch_field_audit: ${batch_audit_present}
- phase33_legacy_drift_governance: ${legacy_gov_present}
- phase33_operator_readiness: ${op_readiness_present}
- phase33_operator_policy: ${op_policy_present}
- phase33_handoff_pack: ${handoff_present}

## frozen_verdicts
- phase32_status: ${phase32_status}
- shadow_validation_status: ${shadow_validation_status}
- shadow_batch_field_audit_status: ${batch_audit_status}
- legacy_governance_status: ${legacy_gov_status}
- operator_readiness_status: ${op_readiness_status}
- operator_policy_status: ${op_policy_status}
- operator_policy_reason: ${op_policy_reason}
- handoff_status: ${handoff_status}
- handoff_reason: ${handoff_reason}

## notes_to_carry_forward
- ${carry_note_1}
- ${carry_note_2}
- ${carry_note_3}

## accepted_limitations
- ${lim_1}
- ${lim_2}
- ${lim_3}
- ${lim_4}

## next_phase_entry_conditions
- ${entry_1}
- ${entry_2}
- ${entry_3}
- ${entry_4}
MD

echo "${marker}"
