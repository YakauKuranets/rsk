#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
READINESS_JSON="${ROOT_DIR}/docs/phase33_operator_readiness_v1.json"
PHASE32_JSON="${ROOT_DIR}/docs/phase32_exit_remediation_report_v1.json"
VALIDATION_JSON="${ROOT_DIR}/docs/phase33_shadow_validation_v1.json"
GOV_JSON="${ROOT_DIR}/docs/phase33_legacy_drift_governance_v1.json"
OUT_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
OUT_MD="${ROOT_DIR}/docs/phase33_operator_policy_v1.md"
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

readiness_present="$(artifact_present "${READINESS_JSON}")"
phase32_present="$(artifact_present "${PHASE32_JSON}")"
validation_present="$(artifact_present "${VALIDATION_JSON}")"
gov_present="$(artifact_present "${GOV_JSON}")"

status="blocked"
reason="validation_artifact_missing"
if [[ "${readiness_present}" == "true" ]]; then
  status="$(read_json_field "${READINESS_JSON}" "status")"
  reason="$(read_json_field "${READINESS_JSON}" "reason")"
fi

readiness_policy="stop"
operator_actions="Stop operations until remediation is complete."
remediation_triggers="Run missing upstream checks and regenerate readiness artifact."
escalation_rules="Escalate to platform owner if blocked state persists after one remediation cycle."
notes=""

case "${reason}" in
  operator_ready)
    readiness_policy="proceed"
    operator_actions="Proceed with operator workflow. Continue routine monitoring."
    remediation_triggers="No immediate remediation required."
    escalation_rules="Escalate only on new blocking drift or runtime health regression."
    ;;
  operator_ready_with_notes)
    readiness_policy="proceed_with_notes"
    operator_actions="Proceed with workflow, but track follow-up items from readiness notes."
    remediation_triggers="Resolve noted drift/issues in the next maintenance window."
    escalation_rules="Escalate if noted items increase or become blocking."
    notes="Follow-up actions are mandatory before next phase gate."
    ;;
  validation_artifact_missing)
    readiness_policy="stop"
    operator_actions="Do not proceed. Re-run missing upstream validation artifacts."
    remediation_triggers="Run audit/validation/governance scripts and regenerate readiness summary."
    escalation_rules="Escalate if artifacts cannot be generated due environment/runtime issues."
    ;;
  remediation_not_healthy)
    readiness_policy="stop"
    operator_actions="Do not proceed. Re-run remediation pipeline and confirm healthy outcome."
    remediation_triggers="Run Phase 32 remediation rerun, then regenerate readiness summary."
    escalation_rules="Escalate to remediation owner if status remains blocked."
    ;;
  legacy_drift_not_acceptable)
    readiness_policy="stop"
    operator_actions="Do not proceed. Run legacy backfill, then re-audit and governance."
    remediation_triggers="Run batch-field backfill + audit + governance sequence."
    escalation_rules="Escalate if run_batch_id_only or neither remains above thresholds."
    ;;
  graph_validation_blocked)
    readiness_policy="stop"
    operator_actions="Do not proceed. Investigate shadow validation failures and fix graph consistency."
    remediation_triggers="Repair consistency issues, rerun shadow validation and readiness summary."
    escalation_rules="Escalate if graph integrity failures recur after fix."
    ;;
  *)
    readiness_policy="stop"
    operator_actions="Do not proceed until readiness reason is explicitly resolved."
    remediation_triggers="Re-run readiness pipeline and inspect upstream artifacts."
    escalation_rules="Escalate unknown reason to platform owner."
    ;;
esac

marker="KV_SHADOW_OPERATOR_POLICY_V1|status=${status}|reason=${reason}"

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase33_operator_policy_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${status}",
  "reason": "${reason}",
  "readiness_policy": {
    "mode": "${readiness_policy}",
    "summary": "${operator_actions}"
  },
  "artifact_requirements": {
    "phase33_operator_readiness_v1": {"path": "docs/phase33_operator_readiness_v1.json", "present": ${readiness_present}},
    "phase32_exit_remediation_report_v1": {"path": "docs/phase32_exit_remediation_report_v1.json", "present": ${phase32_present}},
    "phase33_shadow_validation_v1": {"path": "docs/phase33_shadow_validation_v1.json", "present": ${validation_present}},
    "phase33_legacy_drift_governance_v1": {"path": "docs/phase33_legacy_drift_governance_v1.json", "present": ${gov_present}}
  },
  "operator_actions": {
    "primary": "${operator_actions}",
    "notes": "${notes}"
  },
  "remediation_triggers": {
    "trigger": "${reason}",
    "actions": "${remediation_triggers}"
  },
  "escalation_rules": {
    "policy": "${escalation_rules}"
  },
  "handoff_summary": {
    "decision": "${readiness_policy}",
    "next_owner": "operator",
    "requires_blocker_clearance": $( [[ "${readiness_policy}" == "stop" ]] && echo true || echo false )
  },
  "marker": "${marker}"
}
JSON

cat > "${OUT_MD}" <<MD
# Phase 33 Operator Policy v1

Generated at: ${NOW_UTC}

Marker: \`${marker}\`

- status: **${status}**
- reason: **${reason}**
- readiness_policy: **${readiness_policy}**

## Artifact requirements
- phase33_operator_readiness_v1: ${readiness_present}
- phase32_exit_remediation_report_v1: ${phase32_present}
- phase33_shadow_validation_v1: ${validation_present}
- phase33_legacy_drift_governance_v1: ${gov_present}

## Operator actions
- primary: ${operator_actions}
- notes: ${notes}

## Remediation triggers
- trigger: ${reason}
- actions: ${remediation_triggers}

## Escalation rules
- ${escalation_rules}

## Handoff summary
- decision: ${readiness_policy}
- next_owner: operator
- requires_blocker_clearance: $( [[ "${readiness_policy}" == "stop" ]] && echo true || echo false )
MD

echo "${marker}"
