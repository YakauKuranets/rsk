#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export NOTE_BACKLOG_JSON="${ROOT_DIR}/docs/phase34_operator_note_closure_backlog_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export PLANNING_JSON="${ROOT_DIR}/docs/phase34_next_phase_planning_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
backlog_path = Path(os.environ["NOTE_BACKLOG_JSON"])
baseline_path = Path(os.environ["BASELINE_JSON"])
policy_path = Path(os.environ["OP_POLICY_JSON"])
planning_path = Path(os.environ["PLANNING_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

required = [
    ("phase34_operator_note_closure_backlog", backlog_path),
    ("phase33_baseline_freeze", baseline_path),
    ("phase33_operator_policy", policy_path),
    ("phase34_next_phase_planning", planning_path),
]
presence = {name: path.exists() for name, path in required}

if not all(presence.values()):
    status = "triage_blocked"
    reason = "unresolved_true_blockers_remain"
    backlog_payload = {}
    baseline = {}
    policy = {}
    planning = {}
    missing_inputs = [name for name, ok in presence.items() if not ok]
else:
    backlog_payload = json.loads(backlog_path.read_text())
    baseline = json.loads(baseline_path.read_text())
    policy = json.loads(policy_path.read_text())
    planning = json.loads(planning_path.read_text())
    missing_inputs = []

baseline_status = baseline.get("baseline_status", "")
policy_status = policy.get("status", "")
planning_status = planning.get("status", "")

source_items = backlog_payload.get("backlog_items") or []
reclassified_items = []
true_blockers = []
accepted_notes = []
carry_forward_work = []


def normalize_priority(value: str) -> str:
    if value in {"high", "medium", "low"}:
        return value
    return "medium"


for item in source_items:
    item_id = item.get("id", "UNKNOWN")
    original_priority = normalize_priority(item.get("priority", "medium"))
    title = item.get("title", "")
    src_reason = item.get("reason", "")

    new_priority = original_priority
    priority_action = "keep"
    is_true_blocker = original_priority == "high"
    is_accepted_note = original_priority == "low"
    carry_forward = original_priority in {"medium", "low"}
    triage_comment = ""

    if item_id == "BKL-001":
        if baseline_status == "baseline_frozen_with_notes":
            new_priority = "medium"
            priority_action = "downgrade"
            is_true_blocker = False
            is_accepted_note = True
            carry_forward = True
            triage_comment = "Baseline is already baseline_frozen_with_notes; emergency blocker posture removed."
        elif baseline_status == "baseline_frozen":
            new_priority = "low"
            priority_action = "downgrade"
            is_true_blocker = False
            is_accepted_note = True
            carry_forward = False
            triage_comment = "Baseline is frozen; item treated as closed-note verification only."
        else:
            is_true_blocker = True
            is_accepted_note = False
            carry_forward = False
            triage_comment = f"Baseline remains non-frozen ({baseline_status or 'unknown'}); blocker remains active."
    elif item_id in {"BKL-002", "BKL-003"}:
        if policy_status == "blocked":
            new_priority = "high"
            priority_action = "keep"
            is_true_blocker = True
            is_accepted_note = False
            carry_forward = False
            triage_comment = "Policy gate is blocked; treat as true blocker before next major track."
        else:
            new_priority = "medium"
            priority_action = "downgrade" if original_priority == "high" else "keep"
            is_true_blocker = False
            is_accepted_note = False
            carry_forward = True
            triage_comment = "Policy gate is not blocked; can move to managed carry-forward execution."
    elif item_id == "BKL-004":
        new_priority = "medium"
        priority_action = "keep" if original_priority == "medium" else "set"
        is_true_blocker = False
        is_accepted_note = True
        carry_forward = True
        triage_comment = "Planning note-closure remains accepted track-prep work."
    elif item_id.startswith("BKL-10"):
        new_priority = "medium"
        priority_action = "keep" if original_priority == "medium" else "set"
        is_true_blocker = False
        is_accepted_note = True
        carry_forward = True
        triage_comment = "Carry-forward note should stay visible but not block track entry by itself."
    elif item_id.startswith("BKL-20"):
        new_priority = "low"
        priority_action = "keep" if original_priority == "low" else "set"
        is_true_blocker = False
        is_accepted_note = True
        carry_forward = True
        triage_comment = "Constraint watch item accepted as low-priority operational guardrail."
    else:
        if original_priority == "high":
            triage_comment = "Unmapped high-priority item remains blocker by default."
            is_true_blocker = True
            is_accepted_note = False
            carry_forward = False
        elif original_priority == "medium":
            triage_comment = "Unmapped medium-priority item carried forward for next-track planning."
            is_true_blocker = False
            is_accepted_note = True
            carry_forward = True
        else:
            triage_comment = "Unmapped low-priority item treated as accepted note."
            is_true_blocker = False
            is_accepted_note = True
            carry_forward = True

    rec = {
        "id": item_id,
        "title": title,
        "source_artifact": item.get("source_artifact", ""),
        "original_priority": original_priority,
        "triaged_priority": new_priority,
        "priority_action": priority_action,
        "is_true_blocker": is_true_blocker,
        "is_accepted_note": is_accepted_note,
        "carry_forward": carry_forward,
        "triage_comment": triage_comment,
        "reason": src_reason,
        "operator_action": item.get("operator_action", ""),
        "closure_rule": item.get("closure_rule", ""),
    }
    reclassified_items.append(rec)

    if is_true_blocker:
        true_blockers.append(rec)
    if is_accepted_note:
        accepted_notes.append(rec)
    if carry_forward:
        carry_forward_work.append(rec)

triage_summary = {
    "input_backlog_items": len(source_items),
    "true_blockers_count": len(true_blockers),
    "accepted_notes_count": len(accepted_notes),
    "carry_forward_work_count": len(carry_forward_work),
    "baseline_status": baseline_status,
    "operator_policy_status": policy_status,
    "planning_status": planning_status,
    "missing_inputs": missing_inputs,
}

if all(presence.values()):
    if true_blockers:
        status = "triage_blocked"
        reason = "unresolved_true_blockers_remain"
    elif accepted_notes:
        status = "triage_ready_with_notes"
        reason = "backlog_aligned_with_notes"
    else:
        status = "triage_ready"
        reason = "backlog_aligned_for_next_track"

next_track_readiness = {
    "ready_for_next_major_track": status in {"triage_ready", "triage_ready_with_notes"},
    "status": status,
    "reason": reason,
    "operator_gate_decision": "do_not_open_next_major_track" if status == "triage_blocked" else "can_open_next_major_track_with_triage_controls",
    "entry_instruction": (
        "Resolve true blockers before opening next major track."
        if status == "triage_blocked"
        else "Proceed with next major track while preserving accepted notes and carry-forward controls."
    ),
}

marker = f"KV_OPERATOR_BACKLOG_TRIAGE_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_operator_backlog_triage_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "triage_summary": triage_summary,
    "reclassified_items": reclassified_items,
    "true_blockers": true_blockers,
    "accepted_notes": accepted_notes,
    "carry_forward_work": carry_forward_work,
    "next_track_readiness": next_track_readiness,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Phase 34 Operator Backlog Triage v1",
    "",
    f"Generated at: {now}",
    "",
    f"Marker: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "",
    "## triage_summary",
]
for k, v in triage_summary.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## reclassified_items"]
if reclassified_items:
    for item in reclassified_items:
        lines += [
            f"- {item['id']} | {item['original_priority']} -> {item['triaged_priority']} | blocker={str(item['is_true_blocker']).lower()} | accepted_note={str(item['is_accepted_note']).lower()} | carry_forward={str(item['carry_forward']).lower()}",
            f"  - title: {item['title']}",
            f"  - reason: {item['reason']}",
            f"  - triage_comment: {item['triage_comment']}",
            f"  - operator_action: {item['operator_action']}",
            f"  - closure_rule: {item['closure_rule']}",
        ]
else:
    lines.append("- none")


def render_simple_section(section_name, items):
    lines.append("")
    lines.append(f"## {section_name}")
    if items:
        for item in items:
            lines.append(f"- {item['id']} | {item['triaged_priority']} | {item['title']}")
            lines.append(f"  - triage_comment: {item['triage_comment']}")
    else:
        lines.append("- none")


render_simple_section("true_blockers", true_blockers)
render_simple_section("accepted_notes", accepted_notes)
render_simple_section("carry_forward_work", carry_forward_work)

lines += ["", "## next_track_readiness"]
for k, v in next_track_readiness.items():
    lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(lines) + "\n")
print(marker)
PY
