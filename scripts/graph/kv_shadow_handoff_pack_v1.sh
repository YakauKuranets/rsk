#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PHASE32_JSON="${ROOT_DIR}/docs/phase32_exit_remediation_report_v1.json"
SHADOW_VALIDATION_JSON="${ROOT_DIR}/docs/phase33_shadow_validation_v1.json"
BATCH_AUDIT_JSON="${ROOT_DIR}/docs/phase33_shadow_batch_field_audit_v1.json"
LEGACY_GOV_JSON="${ROOT_DIR}/docs/phase33_legacy_drift_governance_v1.json"
OP_READINESS_JSON="${ROOT_DIR}/docs/phase33_operator_readiness_v1.json"
OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
OUT_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
OUT_MD="${ROOT_DIR}/docs/phase33_handoff_pack_v1.md"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

artifact_present() { [[ -f "$1" ]] && echo "true" || echo "false"; }

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

phase32_present="$(artifact_present "${PHASE32_JSON}")"
shadow_validation_present="$(artifact_present "${SHADOW_VALIDATION_JSON}")"
batch_audit_present="$(artifact_present "${BATCH_AUDIT_JSON}")"
legacy_gov_present="$(artifact_present "${LEGACY_GOV_JSON}")"
op_readiness_present="$(artifact_present "${OP_READINESS_JSON}")"
op_policy_present="$(artifact_present "${OP_POLICY_JSON}")"

phase32_status="$(read_json_field "${PHASE32_JSON}" "overall_status")"
shadow_validation_status="$(read_json_field "${SHADOW_VALIDATION_JSON}" "status")"
legacy_gov_status="$(read_json_field "${LEGACY_GOV_JSON}" "status")"
op_readiness_status="$(read_json_field "${OP_READINESS_JSON}" "status")"
op_policy_status="$(read_json_field "${OP_POLICY_JSON}" "status")"
op_policy_reason="$(read_json_field "${OP_POLICY_JSON}" "reason")"

status="pass"
reason="handoff_ready"

if [[ "${op_readiness_present}" != "true" || "${op_policy_present}" != "true" || "${phase32_present}" != "true" ]]; then
  status="blocked"
  reason="handoff_blocked_missing_artifacts"
elif [[ "${op_policy_status}" == "blocked" || "${op_readiness_status}" == "blocked" ]]; then
  status="blocked"
  reason="handoff_blocked_missing_artifacts"
elif [[ "${shadow_validation_present}" != "true" || "${batch_audit_present}" != "true" || "${legacy_gov_present}" != "true" ]]; then
  status="pass_with_notes"
  reason="handoff_ready_with_notes"
fi

marker="KV_SHADOW_HANDOFF_PACK_V1|status=${status}|reason=${reason}"

recommended_next="Run operator policy script and resolve blockers before proceeding."
if [[ "${status}" == "pass" ]]; then
  recommended_next="Proceed with next phase gate; keep routine monitoring and periodic governance reruns."
elif [[ "${status}" == "pass_with_notes" ]]; then
  recommended_next="Proceed cautiously with notes; close missing/non-blocking artifacts and rerun readiness/policy."
fi

key_cmd_1="bash scripts/graph/kv_shadow_operator_readiness_v1.sh"
key_cmd_2="bash scripts/graph/kv_shadow_operator_policy_v1.sh"
key_cmd_3="bash scripts/graph/kv_shadow_handoff_pack_v1.sh"

known_limitations="Artifacts may remain blocked/partial when upstream validation outputs are missing or stale."
if [[ "${op_policy_reason}" == "validation_artifact_missing" ]]; then
  known_limitations="Upstream validation artifacts missing; handoff remains informational until regenerated."
fi

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_handoff_pack_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${status}",
  "reason": "${reason}",
  "current_status": {
    "phase32": "${phase32_status}",
    "operator_readiness": "${op_readiness_status}",
    "operator_policy": "${op_policy_status}"
  },
  "required_artifacts": {
    "phase32_exit_remediation": {"path": "docs/phase32_exit_remediation_report_v1.json", "present": ${phase32_present}},
    "phase33_shadow_validation": {"path": "docs/phase33_shadow_validation_v1.json", "present": ${shadow_validation_present}},
    "phase33_shadow_batch_field_audit": {"path": "docs/phase33_shadow_batch_field_audit_v1.json", "present": ${batch_audit_present}},
    "phase33_legacy_drift_governance": {"path": "docs/phase33_legacy_drift_governance_v1.json", "present": ${legacy_gov_present}},
    "phase33_operator_readiness": {"path": "docs/phase33_operator_readiness_v1.json", "present": ${op_readiness_present}},
    "phase33_operator_policy": {"path": "docs/phase33_operator_policy_v1.json", "present": ${op_policy_present}}
  },
  "latest_verdicts": {
    "phase32_status": "${phase32_status}",
    "shadow_validation_status": "${shadow_validation_status}",
    "legacy_governance_status": "${legacy_gov_status}",
    "operator_readiness_status": "${op_readiness_status}",
    "operator_policy_status": "${op_policy_status}",
    "operator_policy_reason": "${op_policy_reason}"
  },
  "key_commands": [
    "${key_cmd_1}",
    "${key_cmd_2}",
    "${key_cmd_3}"
  ],
  "operator_notes": "Use operator policy as the primary decision artifact; this handoff pack is continuity-focused.",
  "known_limitations": "${known_limitations}",
  "recommended_next_step": "${recommended_next}",
  "marker": "${marker}"
}
JSON

cat > "${OUT_MD}" <<MD
# Phase 33 Handoff Pack v1

Generated at: ${NOW_UTC}

Marker: \`${marker}\`

- status: **${status}**
- reason: **${reason}**

## Current status
- phase32: ${phase32_status}
- operator_readiness: ${op_readiness_status}
- operator_policy: ${op_policy_status}

## Required artifacts
- phase32_exit_remediation: ${phase32_present}
- phase33_shadow_validation: ${shadow_validation_present}
- phase33_shadow_batch_field_audit: ${batch_audit_present}
- phase33_legacy_drift_governance: ${legacy_gov_present}
- phase33_operator_readiness: ${op_readiness_present}
- phase33_operator_policy: ${op_policy_present}

## Latest verdicts
- shadow_validation_status: ${shadow_validation_status}
- legacy_governance_status: ${legacy_gov_status}
- operator_policy_reason: ${op_policy_reason}

## Key commands
1. ${key_cmd_1}
2. ${key_cmd_2}
3. ${key_cmd_3}

## Operator notes
Use operator policy as the primary decision artifact; this handoff pack is continuity-focused.

## Known limitations
${known_limitations}

## Recommended next step
${recommended_next}
MD

echo "${marker}"
