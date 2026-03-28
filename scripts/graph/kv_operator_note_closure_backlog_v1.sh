#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export OP_READINESS_JSON="${ROOT_DIR}/docs/phase33_operator_readiness_v1.json"
export PLANNING_JSON="${ROOT_DIR}/docs/phase34_next_phase_planning_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_operator_note_closure_backlog_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_operator_note_closure_backlog_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
baseline_path = Path(os.environ["BASELINE_JSON"])
policy_path = Path(os.environ["OP_POLICY_JSON"])
readiness_path = Path(os.environ["OP_READINESS_JSON"])
planning_path = Path(os.environ["PLANNING_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

required = [
    ("phase33_baseline_freeze", baseline_path),
    ("phase33_operator_policy", policy_path),
    ("phase33_operator_readiness", readiness_path),
    ("phase34_next_phase_planning", planning_path),
]

presence = {name: path.exists() for name, path in required}

if not all(presence.values()):
    status = "backlog_blocked"
    reason = "missing_note_source_artifact"
    baseline = {}
    policy = {}
    readiness = {}
    planning = {}
else:
    baseline = json.loads(baseline_path.read_text())
    policy = json.loads(policy_path.read_text())
    readiness = json.loads(readiness_path.read_text())
    planning = json.loads(planning_path.read_text())
    status = "backlog_ready"
    reason = "notes_structured_for_closure"

items = []

def add_item(item_id, title, source, item_reason, priority, action, closure):
    items.append({
        "id": item_id,
        "title": title,
        "source_artifact": source,
        "reason": item_reason,
        "priority": priority,
        "operator_action": action,
        "closure_rule": closure,
    })

if all(presence.values()):
    baseline_status = baseline.get("baseline_status", "")
    policy_status = policy.get("status", "")
    policy_reason = policy.get("reason", "")
    readiness_status = readiness.get("status", "")
    readiness_reason = readiness.get("reason", "")
    planning_status = planning.get("status", "")

    if baseline_status != "baseline_frozen":
        add_item(
            "BKL-001",
            "Stabilize baseline to frozen state",
            "docs/phase33_baseline_freeze_v1.json",
            f"baseline_status={baseline_status}",
            "high",
            "Close missing artifacts/operational blockers, then regenerate baseline freeze artifact.",
            "baseline_status becomes baseline_frozen or baseline_frozen_with_notes with explicit accepted notes.",
        )

    if policy_status == "blocked":
        add_item(
            "BKL-002",
            "Clear operator policy blockers",
            "docs/phase33_operator_policy_v1.json",
            f"operator_policy blocked ({policy_reason})",
            "high",
            "Resolve policy-required artifacts and rerun policy generation in allowed maintenance window.",
            "operator_policy status is no longer blocked and remediation triggers are clear.",
        )

    if readiness_status == "blocked":
        add_item(
            "BKL-003",
            "Clear operator readiness blockers",
            "docs/phase33_operator_readiness_v1.json",
            f"operator_readiness blocked ({readiness_reason})",
            "high",
            "Close readiness artifact gaps and align section verdicts with operator policy.",
            "operator_readiness status is no longer blocked and section checks are resolved.",
        )

    if planning_status == "planning_ready_with_notes":
        add_item(
            "BKL-004",
            "Convert planning notes into tracked closure tasks",
            "docs/phase34_next_phase_planning_v1.json",
            "planning remains with notes and requires explicit closure sequencing",
            "medium",
            "Create operator-owned checklist for every carry-forward note and assign closure evidence format.",
            "all carry_forward_notes are mapped to closed or deferred backlog items with explicit owner/date.",
        )

    carry_notes = planning.get("carry_forward_notes") or baseline.get("notes_to_carry_forward") or []
    for idx, note in enumerate(carry_notes[:3], start=1):
        add_item(
            f"BKL-1{idx:02d}",
            f"Carry-forward note closure #{idx}",
            "docs/phase34_next_phase_planning_v1.json",
            str(note),
            "medium",
            "Translate note into a concrete operator task with evidence checkpoint.",
            "note has evidence link and is marked closed or deferred with rationale.",
        )

    for idx, constraint in enumerate((planning.get("preserved_constraints") or [])[:2], start=1):
        add_item(
            f"BKL-2{idx:02d}",
            f"Constraint drift watch #{idx}",
            "docs/phase34_next_phase_planning_v1.json",
            f"monitor constraint: {constraint}",
            "low",
            "Keep periodic operator check that the constraint remains intact during planning execution.",
            "no violations recorded for the constraint across the next phase checkpoint.",
        )

    if any(i["priority"] == "low" for i in items):
        status = "backlog_ready_with_notes"
        reason = "notes_structured_with_deferred_items"

marker = f"KV_OPERATOR_NOTE_CLOSURE_BACKLOG_V1|status={status}|reason={reason}"

priority_buckets = {
    "high": [i["id"] for i in items if i["priority"] == "high"],
    "medium": [i["id"] for i in items if i["priority"] == "medium"],
    "low": [i["id"] for i in items if i["priority"] == "low"],
}

operator_followups = [
    "Review high-priority items first; do not start major-track execution while high blockers remain open.",
    "Use operator policy artifact as authoritative gate for closure validation.",
    "Update planning artifact references after each closure checkpoint to keep continuity deterministic.",
]

closure_conditions = [
    "All high-priority backlog items are closed with evidence.",
    "Medium-priority note closures are either closed or explicitly deferred with rationale.",
    "No forbidden regression from planning artifact is violated during closure work.",
    "A refreshed baseline/planning snapshot exists before entering next major track.",
]

deferred_items = [
    {
        "id": i["id"],
        "title": i["title"],
        "reason": i["reason"],
        "defer_rule": "Allowed only when all high-priority items are closed and risk remains controlled.",
    }
    for i in items
    if i["priority"] == "low"
]

current_note_state = {
    "baseline_status": baseline.get("baseline_status", "") if baseline else "",
    "baseline_reason": baseline.get("reason", "") if baseline else "",
    "operator_policy_status": policy.get("status", "") if policy else "",
    "operator_policy_reason": policy.get("reason", "") if policy else "",
    "operator_readiness_status": readiness.get("status", "") if readiness else "",
    "operator_readiness_reason": readiness.get("reason", "") if readiness else "",
    "planning_status": planning.get("status", "") if planning else "",
    "planning_reason": planning.get("reason", "") if planning else "",
}

payload = {
    "version": "phase34_operator_note_closure_backlog_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "current_note_state": current_note_state,
    "backlog_items": items,
    "priority_buckets": priority_buckets,
    "operator_followups": operator_followups,
    "closure_conditions": closure_conditions,
    "deferred_items": deferred_items,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Phase 34 Operator Note Closure Backlog v1",
    "",
    f"Generated at: {now}",
    "",
    f"Marker: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "",
    "## current_note_state",
]
for k, v in current_note_state.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## backlog_items"]
if items:
    for i in items:
        lines += [
            f"- {i['id']} | {i['priority']} | {i['title']}",
            f"  - source_artifact: {i['source_artifact']}",
            f"  - reason: {i['reason']}",
            f"  - operator_action: {i['operator_action']}",
            f"  - closure_rule: {i['closure_rule']}",
        ]
else:
    lines.append("- no backlog items (source artifacts missing or no notes detected)")

lines += ["", "## priority_buckets"]
for p in ["high", "medium", "low"]:
    ids = priority_buckets[p]
    lines.append(f"- {p}: {', '.join(ids) if ids else 'none'}")

lines += ["", "## operator_followups"]
for f in operator_followups:
    lines.append(f"- {f}")

lines += ["", "## closure_conditions"]
for c in closure_conditions:
    lines.append(f"- {c}")

lines += ["", "## deferred_items"]
if deferred_items:
    for d in deferred_items:
        lines.append(f"- {d['id']} | {d['title']}")
        lines.append(f"  - reason: {d['reason']}")
        lines.append(f"  - defer_rule: {d['defer_rule']}")
else:
    lines.append("- none")

out_md.write_text("\n".join(lines) + "\n")
print(marker)
PY
