#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export RUNTIME_REQUEST_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_request_packet_v1.json"
export RUNTIME_ENTRY_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_entry_contract_v1.json"
export DECISION_MEMO_JSON="${ROOT_DIR}/docs/phase34_validation_agent_gate_decision_memo_v1.json"
export OPERATOR_GATE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_operator_gate_v1.json"
export APPROVAL_RECORD_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.json"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_review_response_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_validation_agent_runtime_review_response_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
runtime_request_path = Path(os.environ["RUNTIME_REQUEST_JSON"])
runtime_entry_path = Path(os.environ["RUNTIME_ENTRY_JSON"])
decision_memo_path = Path(os.environ["DECISION_MEMO_JSON"])
operator_gate_path = Path(os.environ["OPERATOR_GATE_JSON"])
approval_record_path = Path(os.environ["APPROVAL_RECORD_JSON"])
approval_contract_path = Path(os.environ["APPROVAL_CONTRACT_JSON"])
policy_path = Path(os.environ["OP_POLICY_JSON"])
baseline_path = Path(os.environ["BASELINE_JSON"])
handoff_path = Path(os.environ["HANDOFF_JSON"])
triage_path = Path(os.environ["TRIAGE_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

required = [
    ("phase34_validation_agent_runtime_request_packet", runtime_request_path),
    ("phase34_validation_agent_runtime_entry_contract", runtime_entry_path),
    ("phase34_validation_agent_gate_decision_memo", decision_memo_path),
    ("phase34_validation_agent_operator_gate", operator_gate_path),
    ("phase34_validation_agent_approval_record", approval_record_path),
    ("phase34_validation_agent_approval_contract", approval_contract_path),
    ("phase33_operator_policy", policy_path),
    ("phase33_baseline_freeze", baseline_path),
    ("phase33_handoff_pack", handoff_path),
]
required_presence = {name: path.exists() for name, path in required}
triage_present = triage_path.exists()
missing_required = [name for name, ok in required_presence.items() if not ok]

if not missing_required:
    runtime_request = json.loads(runtime_request_path.read_text())
    runtime_entry = json.loads(runtime_entry_path.read_text())
    decision_memo = json.loads(decision_memo_path.read_text())
    operator_gate = json.loads(operator_gate_path.read_text())
    approval_record = json.loads(approval_record_path.read_text())
    approval_contract = json.loads(approval_contract_path.read_text())
    policy = json.loads(policy_path.read_text())
    baseline = json.loads(baseline_path.read_text())
    handoff = json.loads(handoff_path.read_text())
    triage = json.loads(triage_path.read_text()) if triage_present else {}
else:
    runtime_request, runtime_entry, decision_memo, operator_gate, approval_record, approval_contract, policy, baseline, handoff, triage = {}, {}, {}, {}, {}, {}, {}, {}, {}, {}

response_scope = {
    "scaffold_type": "external_review_response_design_only",
    "description_ru": "Scaffold документирует формат внешнего review response без открытия runtime.",
    "request_packet_ref": runtime_request.get("marker", ""),
}

possible_review_outcomes = [
    "approved_design_only",
    "approved_with_notes_design_only",
    "needs_more_evidence",
    "returned_for_revision",
    "rejected_for_runtime_entry",
]

required_response_fields = [
    "response_id",
    "response_time",
    "reviewer_id",
    "request_packet_ref",
    "outcome",
    "outcome_notes",
    "required_followups",
    "evidence_gaps",
    "execution_authorized",
    "graph_write_authorized",
    "remediation_authorized",
    "runtime_phase_open",
]

outcome_interpretation_rules = {
    "approved_design_only": "Не означает разрешение на runtime execution.",
    "approved_with_notes_design_only": "Не означает разрешение на runtime execution.",
    "needs_more_evidence": "Блокирует следующий переход.",
    "returned_for_revision": "Блокирует следующий переход.",
    "rejected_for_runtime_entry": "Блокирует следующий переход.",
}

non_execution_confirmation = {
    "execution_authorized": False,
    "graph_write_authorized": False,
    "remediation_authorized": False,
    "runtime_phase_open": False,
    "positive_outcome_not_runtime_enable": True,
    "operator_message_ru": "Даже положительный review outcome не включает runtime.",
}

follow_up_actions = [
    "Обновить evidence gaps при их наличии.",
    "Перепроверить консистентность policy/baseline/handoff цепочки.",
    "Повторно подтвердить non-execution флаги.",
    "Подготовить только design-level уточнения без runtime-активации.",
]

states = [
    runtime_request.get("status", ""),
    runtime_entry.get("status", ""),
    decision_memo.get("status", ""),
    operator_gate.get("status", ""),
    approval_record.get("status", ""),
    approval_contract.get("status", ""),
    policy.get("status", ""),
    baseline.get("baseline_status", ""),
    handoff.get("status", ""),
    triage.get("status", "") if triage_present else "",
]
blocked_detected = any(s.endswith("blocked") or s == "blocked" for s in states if s)
notes_detected = any("with_notes" in s for s in states if s)

if missing_required:
    status = "review_response_blocked"
    reason = "safe_review_response_reference_missing"
elif blocked_detected or notes_detected:
    status = "review_response_ready_with_notes"
    reason = "safe_review_response_reference_ready_with_notes"
else:
    status = "review_response_ready"
    reason = "safe_review_response_reference_ready"

review_response_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": missing_required,
    "operator_message_ru": "Review response scaffold сформирован как read-only описание внешнего outcome packet.",
}

next_safe_step = {
    "step_ru": "Использовать scaffold для внешнего review-ответа и фиксировать outcome без открытия runtime.",
    "control_ru": "Любой runtime по-прежнему требует отдельной разрешённой runtime-фазы.",
}

marker = f"KV_VALIDATION_AGENT_RUNTIME_REVIEW_RESPONSE_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_validation_agent_runtime_review_response_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "review_response_status": review_response_status,
    "response_scope": response_scope,
    "possible_review_outcomes": possible_review_outcomes,
    "required_response_fields": required_response_fields,
    "outcome_interpretation_rules": outcome_interpretation_rules,
    "non_execution_confirmation": non_execution_confirmation,
    "follow_up_actions": follow_up_actions,
    "next_safe_step": next_safe_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Фаза 34 — ValidationAgent Runtime Review Response v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "",
    "## review_response_status",
]
for k, v in review_response_status.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## response_scope"]
for k, v in response_scope.items():
    lines.append(f"- {k}: {v}")

for sec_name, sec_list in [
    ("possible_review_outcomes", possible_review_outcomes),
    ("required_response_fields", required_response_fields),
    ("follow_up_actions", follow_up_actions),
]:
    lines += ["", f"## {sec_name}"]
    for item in sec_list:
        lines.append(f"- {item}")

lines += ["", "## outcome_interpretation_rules"]
for k, v in outcome_interpretation_rules.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## non_execution_confirmation"]
for k, v in non_execution_confirmation.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## next_safe_step"]
for k, v in next_safe_step.items():
    lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(lines) + "\n")
print("Готово: external review response / gate outcome packet scaffold сформирован (read-only).")
print(f"Маркер: {marker}")
PY
