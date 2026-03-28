#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export DRY_RUN_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export PLANNING_JSON="${ROOT_DIR}/docs/phase34_validation_agent_planning_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
dry_run_path = Path(os.environ["DRY_RUN_JSON"])
planning_path = Path(os.environ["PLANNING_JSON"])
policy_path = Path(os.environ["OP_POLICY_JSON"])
handoff_path = Path(os.environ["HANDOFF_JSON"])
baseline_path = Path(os.environ["BASELINE_JSON"])
triage_path = Path(os.environ["TRIAGE_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

required = [
    ("phase34_validation_agent_dry_run", dry_run_path),
    ("phase34_validation_agent_planning", planning_path),
    ("phase33_operator_policy", policy_path),
    ("phase33_handoff_pack", handoff_path),
    ("phase33_baseline_freeze", baseline_path),
]
required_presence = {name: path.exists() for name, path in required}
triage_present = triage_path.exists()
missing_required = [name for name, ok in required_presence.items() if not ok]

if not missing_required:
    dry_run = json.loads(dry_run_path.read_text())
    planning = json.loads(planning_path.read_text())
    policy = json.loads(policy_path.read_text())
    handoff = json.loads(handoff_path.read_text())
    baseline = json.loads(baseline_path.read_text())
    triage = json.loads(triage_path.read_text()) if triage_present else {}
else:
    dry_run, planning, policy, handoff, baseline, triage = {}, {}, {}, {}, {}, {}

approval_scope = {
    "scope_mode": "manual_approval_required_design_only",
    "description": "Approval layer is scaffolded only; no runtime execution is enabled in this phase.",
    "execution_still_disabled": True,
    "graph_write_still_forbidden": True,
    "remediation_still_forbidden": True,
    "approval_is_not_auto_execution": True,
    "approval_has_no_silent_execution_fallback": True,
    "approval_unlocks_eligibility_not_execution": True,
    "future_transition_rule": "Any future transition from dry_run to manual_approval_required requires explicit operator approval records and preserved policy/handoff/baseline gates.",
}

required_approvers = [
    "operator",
]

mandatory_gates = [
    "operator policy gate",
    "handoff chain consistency gate",
    "frozen baseline gate",
    "dry-run contract continuity gate",
]

eligible_action_classes = [
    "recommendation publication",
    "operator-facing summary generation",
    "artifact comparison",
    "validation report generation",
]

forbidden_action_classes = [
    "remediation execution",
    "graph mutation",
    "hidden state changes",
    "auto-approval",
    "implicit approval",
    "policy bypass",
    "baseline bypass",
    "autonomous runtime actions",
]

evidence_requirements = [
    "Reference markers for dry-run/planning/policy/baseline artifacts.",
    "Operator approval decision with timestamp and rationale.",
    "Explicit confirmation that execution/graph writes/remediation remain disabled.",
    "Explicit confirmation that approval does not silently fallback to execution.",
    "Gate-by-gate checklist result for policy, handoff, and baseline continuity.",
]

approval_record_format = {
    "record_fields": [
        "record_id",
        "requested_action_class",
        "artifact_references",
        "gate_results",
        "operator_decision",
        "decision_rationale",
        "decision_timestamp_utc",
        "execution_authorized",
    ],
    "record_constraints": [
        "execution_authorized must remain false in this scaffold phase",
        "requested_action_class must be in eligible_action_classes",
        "all mandatory_gates must be explicitly present in gate_results",
        "approval decision must not imply implicit or automatic execution",
    ],
}

policy_status = policy.get("status", "")
baseline_status = baseline.get("baseline_status", "")
dry_run_status = dry_run.get("status", "")
planning_status = planning.get("status", "")
triage_status = triage.get("status", "") if triage_present else "not_present"
handoff_status = handoff.get("status", "")

notes_detected = any(
    "with_notes" in s
    for s in [dry_run_status, planning_status, triage_status, baseline_status]
)
blocked_detected = any(
    s.endswith("blocked") or s == "blocked"
    for s in [policy_status, baseline_status, dry_run_status, planning_status, triage_status, handoff_status]
    if s
)

if missing_required:
    status = "approval_contract_blocked"
    reason = "safe_approval_reference_missing"
elif blocked_detected:
    status = "approval_contract_ready_with_notes"
    reason = "safe_approval_reference_ready_with_notes"
elif notes_detected:
    status = "approval_contract_ready_with_notes"
    reason = "safe_approval_reference_ready_with_notes"
else:
    status = "approval_contract_ready"
    reason = "safe_approval_reference_ready"

approval_contract_status = {
    "status": status,
    "reason": reason,
    "input_presence": {
        **required_presence,
        "phase34_operator_backlog_triage_optional": triage_present,
    },
    "input_states": {
        "dry_run_status": dry_run_status,
        "planning_status": planning_status,
        "operator_policy_status": policy_status,
        "handoff_status": handoff_status,
        "baseline_status": baseline_status,
        "triage_status": triage_status,
    },
}

next_safe_step = {
    "step": "Keep ValidationAgent in dry-run/reporting path and capture manual approval record template usage only.",
    "operator_instruction": "Do not enable runtime execution; validate approval record completeness and gate continuity first.",
    "transition_target": "manual_approval_required design validation (no execution)",
}

marker = f"KV_VALIDATION_AGENT_APPROVAL_CONTRACT_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_validation_agent_approval_contract_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "approval_contract_status": approval_contract_status,
    "approval_scope": approval_scope,
    "required_approvers": required_approvers,
    "mandatory_gates": mandatory_gates,
    "eligible_action_classes": eligible_action_classes,
    "forbidden_action_classes": forbidden_action_classes,
    "evidence_requirements": evidence_requirements,
    "approval_record_format": approval_record_format,
    "next_safe_step": next_safe_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Phase 34 ValidationAgent Approval Contract v1",
    "",
    f"Generated at: {now}",
    "",
    f"Marker: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "",
    "## approval_contract_status",
]
for k, v in approval_contract_status.items():
    if isinstance(v, dict):
        lines.append(f"- {k}:")
        for ik, iv in v.items():
            lines.append(f"  - {ik}: {iv}")
    else:
        lines.append(f"- {k}: {v}")

lines += ["", "## approval_scope"]
for k, v in approval_scope.items():
    lines.append(f"- {k}: {v}")

for section_name, values in [
    ("required_approvers", required_approvers),
    ("mandatory_gates", mandatory_gates),
    ("eligible_action_classes", eligible_action_classes),
    ("forbidden_action_classes", forbidden_action_classes),
    ("evidence_requirements", evidence_requirements),
]:
    lines += ["", f"## {section_name}"]
    for value in values:
        lines.append(f"- {value}")

lines += ["", "## approval_record_format"]
for k, v in approval_record_format.items():
    lines.append(f"- {k}:")
    for item in v:
        lines.append(f"  - {item}")

lines += ["", "## next_safe_step"]
for k, v in next_safe_step.items():
    lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(lines) + "\n")
print(marker)
PY
