#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export DECISION_MEMO_JSON="${ROOT_DIR}/docs/phase34_validation_agent_gate_decision_memo_v1.json"
export OPERATOR_GATE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_operator_gate_v1.json"
export APPROVAL_RECORD_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.json"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export DRY_RUN_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_entry_contract_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_validation_agent_runtime_entry_contract_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
decision_memo_path = Path(os.environ["DECISION_MEMO_JSON"])
operator_gate_path = Path(os.environ["OPERATOR_GATE_JSON"])
approval_record_path = Path(os.environ["APPROVAL_RECORD_JSON"])
approval_contract_path = Path(os.environ["APPROVAL_CONTRACT_JSON"])
dry_run_path = Path(os.environ["DRY_RUN_JSON"])
policy_path = Path(os.environ["OP_POLICY_JSON"])
baseline_path = Path(os.environ["BASELINE_JSON"])
handoff_path = Path(os.environ["HANDOFF_JSON"])
triage_path = Path(os.environ["TRIAGE_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

required = [
    ("phase34_validation_agent_gate_decision_memo", decision_memo_path),
    ("phase34_validation_agent_operator_gate", operator_gate_path),
    ("phase34_validation_agent_approval_record", approval_record_path),
    ("phase34_validation_agent_approval_contract", approval_contract_path),
    ("phase34_validation_agent_dry_run", dry_run_path),
    ("phase33_operator_policy", policy_path),
    ("phase33_baseline_freeze", baseline_path),
    ("phase33_handoff_pack", handoff_path),
]
required_presence = {name: path.exists() for name, path in required}
triage_present = triage_path.exists()
missing_required = [name for name, ok in required_presence.items() if not ok]

if not missing_required:
    decision_memo = json.loads(decision_memo_path.read_text())
    operator_gate = json.loads(operator_gate_path.read_text())
    approval_record = json.loads(approval_record_path.read_text())
    approval_contract = json.loads(approval_contract_path.read_text())
    dry_run = json.loads(dry_run_path.read_text())
    policy = json.loads(policy_path.read_text())
    baseline = json.loads(baseline_path.read_text())
    handoff = json.loads(handoff_path.read_text())
    triage = json.loads(triage_path.read_text()) if triage_present else {}
else:
    decision_memo, operator_gate, approval_record, approval_contract, dry_run, policy, baseline, handoff, triage = {}, {}, {}, {}, {}, {}, {}, {}, {}

external_gate_requirements = [
    "operator approval current",
    "operator policy current",
    "baseline current",
    "handoff current",
    "decision memo current",
    "dry-run current",
    "approval record current",
    "approval contract current",
]

policy_blocked = policy.get("status", "") == "blocked"
baseline_blocked = str(baseline.get("baseline_status", "")).endswith("blocked")
operator_gate_blocked = str(operator_gate.get("status", "")).endswith("blocked")
decision_memo_blocked = str(decision_memo.get("status", "")).endswith("blocked")

runtime_opening_preconditions = {
    "all_required_artifacts_present": len(missing_required) == 0,
    "policy_not_blocked": not policy_blocked,
    "baseline_not_blocked": not baseline_blocked,
    "operator_gate_not_blocked": not operator_gate_blocked,
    "decision_memo_not_blocked": not decision_memo_blocked,
    "execution_authorized_remains_false_until_runtime_gate": True,
    "graph_write_authorized_remains_false_until_runtime_gate": True,
    "remediation_authorized_remains_false_until_runtime_gate": True,
}

approval_chain_integrity = {
    "decision_memo_marker": decision_memo.get("marker", ""),
    "operator_gate_marker": operator_gate.get("marker", ""),
    "approval_record_marker": approval_record.get("marker", ""),
    "approval_contract_marker": approval_contract.get("marker", ""),
    "dry_run_marker": dry_run.get("marker", ""),
    "operator_policy_marker": policy.get("marker", ""),
    "baseline_marker": baseline.get("marker", ""),
    "handoff_marker": handoff.get("marker", ""),
    "triage_marker": triage.get("marker", "") if triage_present else "not_present",
    "chain_integrity_for_design_phase": True,
    "operator_message_ru": "Цепочка approval/gate сохраняется как read-only проверочный контур.",
}

eligible_transition_targets = [
    "manual_approval_required_design_only",
    "runtime_phase_request_preparation",
    "external_gate_review_only",
]

blocked_transition_paths = [
    "direct runtime execution",
    "direct remediation path",
    "graph mutation path",
    "implicit approval path",
    "silent execution fallback",
    "policy bypass path",
    "baseline bypass path",
]

non_execution_until_runtime_phase = {
    "runtime_phase_open": False,
    "execution_authorized": False,
    "graph_write_authorized": False,
    "remediation_authorized": False,
    "implicit_runtime_transition_allowed": False,
    "silent_execution_fallback_allowed": False,
    "full_contract_is_not_runtime_permission": True,
    "operator_message_ru": "Даже полный runtime entry contract не является разрешением на runtime-исполнение.",
}

states = [
    decision_memo.get("status", ""),
    operator_gate.get("status", ""),
    approval_record.get("status", ""),
    approval_contract.get("status", ""),
    dry_run.get("status", ""),
    policy.get("status", ""),
    baseline.get("baseline_status", ""),
    handoff.get("status", ""),
    triage.get("status", "") if triage_present else "",
]
blocked_detected = any(s.endswith("blocked") or s == "blocked" for s in states if s)
notes_detected = any("with_notes" in s for s in states if s)

if missing_required:
    status = "runtime_entry_contract_blocked"
    reason = "safe_runtime_entry_reference_missing"
elif blocked_detected or notes_detected:
    status = "runtime_entry_contract_ready_with_notes"
    reason = "safe_runtime_entry_reference_ready_with_notes"
else:
    status = "runtime_entry_contract_ready"
    reason = "safe_runtime_entry_reference_ready"

runtime_entry_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": missing_required,
    "external_gate_requirements_total": len(external_gate_requirements),
    "operator_message_ru": "Runtime entry contract сформирован как граница входа в будущую runtime-фазу без запуска исполнения.",
}

next_safe_step = {
    "step_ru": "Использовать контракт для внешнего gate-review и подготовки запроса на runtime-фазу (без исполнения).",
    "control_ru": "Любой запуск runtime допускается только в отдельной разрешённой фазе после внешнего gate handshake.",
}

marker = f"KV_VALIDATION_AGENT_RUNTIME_ENTRY_CONTRACT_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_validation_agent_runtime_entry_contract_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "runtime_entry_status": runtime_entry_status,
    "external_gate_requirements": external_gate_requirements,
    "runtime_opening_preconditions": runtime_opening_preconditions,
    "approval_chain_integrity": approval_chain_integrity,
    "eligible_transition_targets": eligible_transition_targets,
    "blocked_transition_paths": blocked_transition_paths,
    "non_execution_until_runtime_phase": non_execution_until_runtime_phase,
    "next_safe_step": next_safe_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Фаза 34 — ValidationAgent Runtime Entry Contract v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "",
    "## runtime_entry_status",
]
for k, v in runtime_entry_status.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## external_gate_requirements"]
for req in external_gate_requirements:
    lines.append(f"- {req}")

for sec_name, sec_val in [
    ("runtime_opening_preconditions", runtime_opening_preconditions),
    ("approval_chain_integrity", approval_chain_integrity),
    ("non_execution_until_runtime_phase", non_execution_until_runtime_phase),
]:
    lines += ["", f"## {sec_name}"]
    for k, v in sec_val.items():
        lines.append(f"- {k}: {v}")

for sec_name, sec_list in [
    ("eligible_transition_targets", eligible_transition_targets),
    ("blocked_transition_paths", blocked_transition_paths),
]:
    lines += ["", f"## {sec_name}"]
    for item in sec_list:
        lines.append(f"- {item}")

lines += ["", "## next_safe_step"]
for k, v in next_safe_step.items():
    lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(lines) + "\n")
print("Готово: runtime entry contract и внешний gate handshake scaffold сформированы (read-only).")
print(f"Маркер: {marker}")
PY
