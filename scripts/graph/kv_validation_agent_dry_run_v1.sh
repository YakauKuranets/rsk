#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export OP_READINESS_JSON="${ROOT_DIR}/docs/phase33_operator_readiness_v1.json"
export VA_PLANNING_JSON="${ROOT_DIR}/docs/phase34_validation_agent_planning_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export NOTE_BACKLOG_JSON="${ROOT_DIR}/docs/phase34_operator_note_closure_backlog_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
baseline_path = Path(os.environ["BASELINE_JSON"])
policy_path = Path(os.environ["OP_POLICY_JSON"])
readiness_path = Path(os.environ["OP_READINESS_JSON"])
planning_path = Path(os.environ["VA_PLANNING_JSON"])
triage_path = Path(os.environ["TRIAGE_JSON"])
note_backlog_path = Path(os.environ["NOTE_BACKLOG_JSON"])
handoff_path = Path(os.environ["HANDOFF_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

required = [
    ("phase33_baseline_freeze", baseline_path),
    ("phase33_operator_policy", policy_path),
    ("phase33_operator_readiness", readiness_path),
    ("phase34_validation_agent_planning", planning_path),
    ("phase34_operator_backlog_triage", triage_path),
]
optional = [
    ("phase34_operator_note_closure_backlog", note_backlog_path),
    ("phase33_handoff_pack", handoff_path),
]

required_presence = {name: path.exists() for name, path in required}
optional_presence = {name: path.exists() for name, path in optional}

missing_required = [name for name, ok in required_presence.items() if not ok]

if not missing_required:
    baseline = json.loads(baseline_path.read_text())
    policy = json.loads(policy_path.read_text())
    readiness = json.loads(readiness_path.read_text())
    planning = json.loads(planning_path.read_text())
    triage = json.loads(triage_path.read_text())
else:
    baseline, policy, readiness, planning, triage = {}, {}, {}, {}, {}

note_backlog = json.loads(note_backlog_path.read_text()) if optional_presence["phase34_operator_note_closure_backlog"] else {}
handoff = json.loads(handoff_path.read_text()) if optional_presence["phase33_handoff_pack"] else {}

input_artifacts = {
    "required": {
        "phase33_baseline_freeze_v1": {
            "path": "docs/phase33_baseline_freeze_v1.json",
            "present": required_presence["phase33_baseline_freeze"],
            "status": baseline.get("baseline_status", "") if baseline else "",
        },
        "phase33_operator_policy_v1": {
            "path": "docs/phase33_operator_policy_v1.json",
            "present": required_presence["phase33_operator_policy"],
            "status": policy.get("status", "") if policy else "",
        },
        "phase33_operator_readiness_v1": {
            "path": "docs/phase33_operator_readiness_v1.json",
            "present": required_presence["phase33_operator_readiness"],
            "status": readiness.get("status", "") if readiness else "",
        },
        "phase34_validation_agent_planning_v1": {
            "path": "docs/phase34_validation_agent_planning_v1.json",
            "present": required_presence["phase34_validation_agent_planning"],
            "status": planning.get("status", "") if planning else "",
        },
        "phase34_operator_backlog_triage_v1": {
            "path": "docs/phase34_operator_backlog_triage_v1.json",
            "present": required_presence["phase34_operator_backlog_triage"],
            "status": triage.get("status", "") if triage else "",
        },
    },
    "optional": {
        "phase34_operator_note_closure_backlog_v1": {
            "path": "docs/phase34_operator_note_closure_backlog_v1.json",
            "present": optional_presence["phase34_operator_note_closure_backlog"],
            "status": note_backlog.get("status", "") if note_backlog else "",
        },
        "phase33_handoff_pack_v1": {
            "path": "docs/phase33_handoff_pack_v1.json",
            "present": optional_presence["phase33_handoff_pack"],
            "status": handoff.get("status", "") if handoff else "",
        },
    },
    "missing_required": missing_required,
}

agent_mode = {
    "agent_mode": "dry_run",
    "execution_permitted": False,
    "graph_write_permitted": False,
    "auto_remediation_permitted": False,
    "side_effects_permitted": False,
    "mode_contract": "recommendation-only scaffold; no runtime execution",
}

baseline_state = baseline.get("baseline_status", "unknown")
readiness_state = readiness.get("status", "unknown")
policy_state = policy.get("status", "unknown")
triage_state = triage.get("status", "unknown")
planning_state = planning.get("status", "unknown")

safe_entry_in_principle = (
    baseline_state in {"baseline_frozen", "baseline_frozen_with_notes"}
    and readiness_state not in {"blocked", ""}
    and policy_state not in {"blocked", ""}
    and triage_state in {"triage_ready", "triage_ready_with_notes"}
    and planning_state in {"validation_agent_planning_ready", "validation_agent_planning_ready_with_notes"}
)

dry_run_findings = [
    {
        "topic": "baseline_state",
        "state": baseline_state,
        "assessment": "ok" if baseline_state in {"baseline_frozen", "baseline_frozen_with_notes"} else "attention_required",
        "detail": baseline.get("reason", ""),
    },
    {
        "topic": "operator_readiness_state",
        "state": readiness_state,
        "assessment": "ok" if readiness_state not in {"blocked", ""} else "attention_required",
        "detail": readiness.get("reason", ""),
    },
    {
        "topic": "operator_policy_state",
        "state": policy_state,
        "assessment": "ok" if policy_state not in {"blocked", ""} else "attention_required",
        "detail": policy.get("reason", ""),
    },
    {
        "topic": "triage_state",
        "state": triage_state,
        "assessment": "ok" if triage_state in {"triage_ready", "triage_ready_with_notes"} else "attention_required",
        "detail": triage.get("reason", ""),
    },
    {
        "topic": "next_track_entry_safe_in_principle",
        "state": str(safe_entry_in_principle).lower(),
        "assessment": "ok" if safe_entry_in_principle else "attention_required",
        "detail": "Derived from baseline/readiness/policy/triage/planning states in dry-run mode.",
    },
]

policy_alignment = {
    "operator_policy_is_authoritative": True,
    "policy_marker": policy.get("marker", ""),
    "planning_marker": planning.get("marker", ""),
    "triage_marker": triage.get("marker", ""),
    "alignment_result": "aligned" if planning.get("planning_mode_only") is True else "needs_review",
    "alignment_notes": [
        "Dry-run output remains recommendation-only.",
        "No policy bypass path is allowed in this scaffold.",
        "All actions require explicit approval before any future execution phase.",
    ],
}

recommended_actions = [
    "Produce recommendation-level output only.",
    "Do not execute remediation directly.",
    "Do not produce side effects in graph/runtime/UI.",
    "Do not use hidden state changes.",
    "Escalate unresolved blockers via operator policy/handoff chain.",
    "Keep dry-run reports deterministic and artifact-backed.",
]

forbidden_actions_confirmation = [
    "no autonomous remediation",
    "no direct graph mutation",
    "no policy bypass",
    "no baseline bypass",
    "no runtime execution",
    "no auto-approval behavior",
]

approval_requirements = {
    "approval_required": True,
    "approval_scope": "Any action beyond recommendation-only dry-run output",
    "required_approvers": ["operator"],
    "approval_gates": [
        "operator policy gate",
        "frozen baseline gate",
        "handoff chain gate",
    ],
}

if missing_required:
    status = "dry_run_blocked"
    reason = "safe_dry_run_reference_missing"
else:
    raw_states = [baseline_state, readiness_state, policy_state, triage_state, planning_state]
    has_blocked = any(state.endswith("blocked") or state == "blocked" for state in raw_states)
    has_notes = any("with_notes" in state for state in raw_states)
    if has_blocked:
        status = "dry_run_ready_with_notes"
        reason = "safe_dry_run_reference_ready_with_notes"
    elif has_notes:
        status = "dry_run_ready_with_notes"
        reason = "safe_dry_run_reference_ready_with_notes"
    else:
        status = "dry_run_ready"
        reason = "safe_dry_run_reference_ready"

next_safe_step = {
    "step": "Generate operator-reviewed dry-run summary and escalate only recommendation-level actions.",
    "entry_safe_in_principle": safe_entry_in_principle,
    "execution_gate": "Keep execution disabled until explicit approval layer and runtime phase are opened.",
}

marker = f"KV_VALIDATION_AGENT_DRY_RUN_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_validation_agent_dry_run_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "input_artifacts": input_artifacts,
    "agent_mode": agent_mode,
    "dry_run_findings": dry_run_findings,
    "policy_alignment": policy_alignment,
    "recommended_actions": recommended_actions,
    "forbidden_actions_confirmation": forbidden_actions_confirmation,
    "approval_requirements": approval_requirements,
    "next_safe_step": next_safe_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Phase 34 ValidationAgent Dry-Run v1",
    "",
    f"Generated at: {now}",
    "",
    f"Marker: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "",
    "## input_artifacts",
    "### required",
]

for name, meta in input_artifacts["required"].items():
    lines.append(f"- {name}: present={meta['present']} status={meta['status']} path={meta['path']}")

lines += ["", "### optional"]
for name, meta in input_artifacts["optional"].items():
    lines.append(f"- {name}: present={meta['present']} status={meta['status']} path={meta['path']}")

lines.append(f"- missing_required: {input_artifacts['missing_required']}")

lines += ["", "## agent_mode"]
for k, v in agent_mode.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## dry_run_findings"]
for f in dry_run_findings:
    lines.append(f"- {f['topic']}: state={f['state']} assessment={f['assessment']} detail={f['detail']}")

lines += ["", "## policy_alignment"]
for k, v in policy_alignment.items():
    if isinstance(v, list):
        lines.append(f"- {k}:")
        for item in v:
            lines.append(f"  - {item}")
    else:
        lines.append(f"- {k}: {v}")

lines += ["", "## recommended_actions"]
for item in recommended_actions:
    lines.append(f"- {item}")

lines += ["", "## forbidden_actions_confirmation"]
for item in forbidden_actions_confirmation:
    lines.append(f"- {item}")

lines += ["", "## approval_requirements"]
for k, v in approval_requirements.items():
    if isinstance(v, list):
        lines.append(f"- {k}:")
        for item in v:
            lines.append(f"  - {item}")
    else:
        lines.append(f"- {k}: {v}")

lines += ["", "## next_safe_step"]
for k, v in next_safe_step.items():
    lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(lines) + "\n")
print(marker)
PY
