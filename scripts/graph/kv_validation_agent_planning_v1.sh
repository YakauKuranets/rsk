#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export NEXT_PHASE_JSON="${ROOT_DIR}/docs/phase34_next_phase_planning_v1.json"
export BACKLOG_JSON="${ROOT_DIR}/docs/phase34_operator_note_closure_backlog_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_planning_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_validation_agent_planning_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
baseline_path = Path(os.environ["BASELINE_JSON"])
policy_path = Path(os.environ["OP_POLICY_JSON"])
next_phase_path = Path(os.environ["NEXT_PHASE_JSON"])
backlog_path = Path(os.environ["BACKLOG_JSON"])
triage_path = Path(os.environ["TRIAGE_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

required = [
    ("phase33_baseline_freeze", baseline_path),
    ("phase33_operator_policy", policy_path),
    ("phase34_next_phase_planning", next_phase_path),
    ("phase34_operator_note_closure_backlog", backlog_path),
]
presence = {name: path.exists() for name, path in required}
triage_present = triage_path.exists()

if all(presence.values()):
    baseline = json.loads(baseline_path.read_text())
    policy = json.loads(policy_path.read_text())
    next_phase = json.loads(next_phase_path.read_text())
    backlog = json.loads(backlog_path.read_text())
    triage = json.loads(triage_path.read_text()) if triage_present else {}
else:
    baseline, policy, next_phase, backlog, triage = {}, {}, {}, {}, {}

missing_required = [name for name, ok in presence.items() if not ok]

allowed_capabilities = [
    "artifact inspection",
    "readiness evaluation",
    "policy reasoning",
    "planning recommendation",
    "validation summary generation",
]

forbidden_actions = [
    "autonomous remediation",
    "direct graph mutation",
    "bypass of policy/handoff chain",
    "hidden state changes",
    "runtime decisions without explicit approval layer",
    "auto-execution without approval",
    "graph write without separate gate",
    "operator policy bypass",
    "frozen baseline bypass",
]

execution_modes = {
    "planning_mode": {
        "enabled": True,
        "description": "ValidationAgent remains planning-only; no runtime execution is permitted.",
    },
    "allowed_modes": [
        "dry_run",
        "shadow_only",
        "manual_approval_required",
    ],
    "mode_rules": [
        "dry_run: generate recommendations and summaries only, no side effects.",
        "shadow_only: evaluate artifacts against policy/baseline without enforcement actions.",
        "manual_approval_required: every actionable recommendation requires explicit operator approval before any downstream execution.",
    ],
}

approval_requirements = {
    "approval_layer_required": True,
    "required_approvers": ["operator"],
    "mandatory_gates": [
        "operator policy gate",
        "handoff chain consistency gate",
        "frozen baseline adherence gate",
    ],
    "approval_rules": [
        "No automatic remediation or execution may proceed without explicit operator approval.",
        "Any graph write proposal requires separate gate outside this planning artifact.",
        "Any deviation from frozen baseline requires new freeze/policy cycle before execution.",
    ],
}

baseline_status = baseline.get("baseline_status", "")
policy_status = policy.get("status", "")
next_status = next_phase.get("status", "")
backlog_status = backlog.get("status", "")
triage_status = triage.get("status", "") if triage_present else "not_available"

baseline_reference = {
    "baseline_artifact": "docs/phase33_baseline_freeze_v1.json",
    "baseline_present": presence.get("phase33_baseline_freeze", False),
    "baseline_status": baseline_status,
    "baseline_reason": baseline.get("reason", ""),
    "baseline_marker": baseline.get("marker", ""),
    "operator_policy_artifact": "docs/phase33_operator_policy_v1.json",
    "operator_policy_status": policy_status,
    "operator_policy_reason": policy.get("reason", ""),
    "operator_policy_marker": policy.get("marker", ""),
    "planning_artifact": "docs/phase34_next_phase_planning_v1.json",
    "planning_status": next_status,
    "planning_reason": next_phase.get("reason", ""),
    "planning_marker": next_phase.get("marker", ""),
    "note_closure_backlog_artifact": "docs/phase34_operator_note_closure_backlog_v1.json",
    "note_closure_backlog_status": backlog_status,
    "note_closure_backlog_reason": backlog.get("reason", ""),
    "triage_artifact": "docs/phase34_operator_backlog_triage_v1.json" if triage_present else "not_present",
    "triage_status": triage_status,
    "triage_reason": triage.get("reason", "") if triage_present else "",
}

safety_constraints = [
    "read-only only",
    "no graph writes",
    "no backfill",
    "no reruns",
    "no UI/runtime/ValidationAgent execution",
    "no feature expansion beyond planning layer",
    "preserve operator policy and handoff decision chain",
    "respect frozen baseline as authoritative reference",
]

entry_conditions = [
    "Baseline reference artifact must exist and be parseable.",
    "Operator policy artifact must exist and remain authoritative for go/no-go.",
    "Planning artifacts must preserve read-only and no-write constraints.",
    "ValidationAgent stays in planning mode only until explicit approval layer for execution is introduced.",
    "No forbidden action is allowed under any execution mode in this phase.",
]

recommended_first_slice = {
    "name": "validation_agent_planning_contract",
    "objective": "Produce deterministic planning-only validation summaries from baseline/policy/backlog artifacts.",
    "scope": [
        "artifact inspection checklist",
        "readiness and policy reasoning template",
        "manual approval handoff format",
    ],
    "non_goals": [
        "runtime execution",
        "autonomous remediation",
        "graph mutation",
    ],
}

if missing_required:
    status = "validation_agent_planning_blocked"
    reason = "safe_planning_reference_missing"
elif triage_present and triage_status == "triage_blocked":
    status = "validation_agent_planning_ready_with_notes"
    reason = "safe_planning_reference_ready_with_notes"
elif baseline_status in {"baseline_frozen", "baseline_frozen_with_notes"} and policy_status != "blocked":
    status = "validation_agent_planning_ready"
    reason = "safe_planning_reference_ready"
else:
    status = "validation_agent_planning_ready_with_notes"
    reason = "safe_planning_reference_ready_with_notes"

marker = f"KV_VALIDATION_AGENT_PLANNING_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_validation_agent_planning_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "baseline_reference": baseline_reference,
    "safety_constraints": safety_constraints,
    "allowed_capabilities": allowed_capabilities,
    "forbidden_actions": forbidden_actions,
    "execution_modes": execution_modes,
    "approval_requirements": approval_requirements,
    "entry_conditions": entry_conditions,
    "recommended_first_slice": recommended_first_slice,
    "planning_mode_only": True,
    "missing_required_inputs": missing_required,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Phase 34 ValidationAgent Planning v1",
    "",
    f"Generated at: {now}",
    "",
    f"Marker: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "- planning_mode_only: **true**",
    "",
    "## baseline_reference",
]
for k, v in baseline_reference.items():
    lines.append(f"- {k}: {v}")


def add_list_section(name, values):
    lines.append("")
    lines.append(f"## {name}")
    for value in values:
        lines.append(f"- {value}")


add_list_section("safety_constraints", safety_constraints)
add_list_section("allowed_capabilities", allowed_capabilities)
add_list_section("forbidden_actions", forbidden_actions)

lines += ["", "## execution_modes"]
lines.append(f"- planning_mode.enabled: {execution_modes['planning_mode']['enabled']}")
lines.append(f"- planning_mode.description: {execution_modes['planning_mode']['description']}")
lines.append(f"- allowed_modes: {', '.join(execution_modes['allowed_modes'])}")
for rule in execution_modes["mode_rules"]:
    lines.append(f"- mode_rule: {rule}")

lines += ["", "## approval_requirements"]
for key, value in approval_requirements.items():
    if isinstance(value, list):
        lines.append(f"- {key}:")
        for item in value:
            lines.append(f"  - {item}")
    else:
        lines.append(f"- {key}: {value}")

add_list_section("entry_conditions", entry_conditions)

lines += ["", "## recommended_first_slice"]
for key, value in recommended_first_slice.items():
    if isinstance(value, list):
        lines.append(f"- {key}:")
        for item in value:
            lines.append(f"  - {item}")
    else:
        lines.append(f"- {key}: {value}")

out_md.write_text("\n".join(lines) + "\n")
print(marker)
PY
