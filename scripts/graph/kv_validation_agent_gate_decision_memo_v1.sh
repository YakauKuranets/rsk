#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export OP_GATE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_operator_gate_v1.json"
export APPROVAL_RECORD_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.json"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export APPROVAL_REHEARSAL_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_rehearsal_v1.json"
export DRY_RUN_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_gate_decision_memo_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_validation_agent_gate_decision_memo_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
op_gate_path = Path(os.environ["OP_GATE_JSON"])
approval_record_path = Path(os.environ["APPROVAL_RECORD_JSON"])
approval_contract_path = Path(os.environ["APPROVAL_CONTRACT_JSON"])
approval_rehearsal_path = Path(os.environ["APPROVAL_REHEARSAL_JSON"])
dry_run_path = Path(os.environ["DRY_RUN_JSON"])
policy_path = Path(os.environ["OP_POLICY_JSON"])
baseline_path = Path(os.environ["BASELINE_JSON"])
handoff_path = Path(os.environ["HANDOFF_JSON"])
triage_path = Path(os.environ["TRIAGE_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

required = [
    ("phase34_validation_agent_operator_gate", op_gate_path),
    ("phase34_validation_agent_approval_record", approval_record_path),
    ("phase34_validation_agent_approval_contract", approval_contract_path),
    ("phase34_validation_agent_approval_rehearsal", approval_rehearsal_path),
    ("phase34_validation_agent_dry_run", dry_run_path),
    ("phase33_operator_policy", policy_path),
    ("phase33_baseline_freeze", baseline_path),
    ("phase33_handoff_pack", handoff_path),
]
required_presence = {name: path.exists() for name, path in required}
triage_present = triage_path.exists()
missing_required = [name for name, ok in required_presence.items() if not ok]

if not missing_required:
    op_gate = json.loads(op_gate_path.read_text())
    approval_record = json.loads(approval_record_path.read_text())
    approval_contract = json.loads(approval_contract_path.read_text())
    approval_rehearsal = json.loads(approval_rehearsal_path.read_text())
    dry_run = json.loads(dry_run_path.read_text())
    policy = json.loads(policy_path.read_text())
    baseline = json.loads(baseline_path.read_text())
    handoff = json.loads(handoff_path.read_text())
    triage = json.loads(triage_path.read_text()) if triage_present else {}
else:
    op_gate, approval_record, approval_contract, approval_rehearsal, dry_run, policy, baseline, handoff, triage = {}, {}, {}, {}, {}, {}, {}, {}, {}

blocked_detected = any(
    s.endswith("blocked") or s == "blocked"
    for s in [
        op_gate.get("status", ""),
        approval_record.get("status", ""),
        approval_contract.get("status", ""),
        approval_rehearsal.get("status", ""),
        dry_run.get("status", ""),
        policy.get("status", ""),
        baseline.get("baseline_status", ""),
        handoff.get("status", ""),
        triage.get("status", "") if triage_present else "",
    ]
    if s
)
notes_detected = any(
    "with_notes" in s
    for s in [
        op_gate.get("status", ""),
        approval_record.get("status", ""),
        approval_contract.get("status", ""),
        approval_rehearsal.get("status", ""),
        dry_run.get("status", ""),
        triage.get("status", "") if triage_present else "",
    ]
)

if missing_required:
    status = "decision_memo_blocked"
    reason = "safe_decision_memo_reference_missing"
    operator_decision_position = "not_ready_for_manual_approval_design_only"
elif blocked_detected:
    status = "decision_memo_ready_with_notes"
    reason = "safe_decision_memo_reference_ready_with_notes"
    operator_decision_position = "ready_with_notes_for_manual_approval_design_only"
elif notes_detected:
    status = "decision_memo_ready_with_notes"
    reason = "safe_decision_memo_reference_ready_with_notes"
    operator_decision_position = "ready_with_notes_for_manual_approval_design_only"
else:
    status = "decision_memo_ready"
    reason = "safe_decision_memo_reference_ready"
    operator_decision_position = "ready_for_manual_approval_design_only"

decision_basis = {
    "operator_gate_status": op_gate.get("status", ""),
    "approval_record_status": approval_record.get("status", ""),
    "approval_contract_status": approval_contract.get("status", ""),
    "approval_rehearsal_status": approval_rehearsal.get("status", ""),
    "dry_run_status": dry_run.get("status", ""),
    "operator_policy_status": policy.get("status", ""),
    "baseline_status": baseline.get("baseline_status", ""),
    "handoff_status": handoff.get("status", ""),
    "triage_status": triage.get("status", "") if triage_present else "not_present",
    "operator_message_ru": "Decision memo фиксирует позицию gate-процесса и не является разрешением на исполнение.",
}

gate_summary = {
    "required_checks_total": len(op_gate.get("required_gate_checks", [])),
    "checklist_total": len(op_gate.get("operator_checklist", [])),
    "gate_failure_conditions_total": len(op_gate.get("gate_failure_conditions", [])),
    "gate_pass_conditions_total": len(op_gate.get("gate_pass_conditions", [])),
    "operator_message_ru": "Сводка gate основана на актуальном operator gate scaffold.",
}

evidence_summary = {
    "evidence_bundle": op_gate.get("evidence_bundle", {}),
    "required_evidence_refs": approval_record.get("required_evidence", []),
    "evidence_bundle_complete": len(op_gate.get("gate_status", {}).get("missing_evidence", [])) == 0,
    "operator_message_ru": "Evidence bundle оценивается как пакет подтверждений, а не как допуск к runtime.",
}

non_execution_constraints = {
    "execution_authorized": False,
    "graph_write_authorized": False,
    "remediation_authorized": False,
    "runtime_phase_open": False,
    "memo_does_not_allow_runtime_execution": True,
    "memo_does_not_remove_policy_or_baseline_gates": True,
    "memo_does_not_replace_separate_runtime_phase": True,
}

allowed_next_actions = [
    "recommendation review",
    "operator memo review",
    "evidence refresh",
    "approval packet refinement",
    "dry-run summary publication",
]

forbidden_next_actions = [
    "runtime execution",
    "remediation execution",
    "graph mutation",
    "hidden state changes",
    "policy bypass",
    "baseline bypass",
    "implicit approval",
    "silent fallback to execution",
]

decision_memo_status = {
    "status": status,
    "reason": reason,
    "operator_decision_position": operator_decision_position,
    "missing_required_inputs": missing_required,
    "operator_message_ru": "Итоговый memo сформирован как operator-facing gate packet в read-only режиме.",
}

next_safe_step = {
    "step_ru": "Провести операторский review memo и обновить evidence при необходимости без запуска исполнения.",
    "control_ru": "Переход к runtime возможен только через отдельную разрешённую фазу после внешнего утверждения.",
}

marker = f"KV_VALIDATION_AGENT_GATE_DECISION_MEMO_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_validation_agent_gate_decision_memo_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "decision_memo_status": decision_memo_status,
    "decision_basis": decision_basis,
    "gate_summary": gate_summary,
    "evidence_summary": evidence_summary,
    "operator_decision_position": operator_decision_position,
    "non_execution_constraints": non_execution_constraints,
    "allowed_next_actions": allowed_next_actions,
    "forbidden_next_actions": forbidden_next_actions,
    "next_safe_step": next_safe_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Фаза 34 — ValidationAgent Gate Decision Memo v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    f"- operator_decision_position: **{operator_decision_position}**",
    "",
    "## decision_memo_status",
]
for k, v in decision_memo_status.items():
    lines.append(f"- {k}: {v}")

for sec_name, sec_val in [
    ("decision_basis", decision_basis),
    ("gate_summary", gate_summary),
    ("evidence_summary", evidence_summary),
    ("non_execution_constraints", non_execution_constraints),
]:
    lines += ["", f"## {sec_name}"]
    for k, v in sec_val.items():
        lines.append(f"- {k}: {v}")

lines += ["", "## allowed_next_actions"]
for item in allowed_next_actions:
    lines.append(f"- {item}")

lines += ["", "## forbidden_next_actions"]
for item in forbidden_next_actions:
    lines.append(f"- {item}")

lines += ["", "## next_safe_step"]
for k, v in next_safe_step.items():
    lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(lines) + "\n")
print("Готово: decision memo / gate decision packet сформирован (read-only).")
print(f"Маркер: {marker}")
PY
