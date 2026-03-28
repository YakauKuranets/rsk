#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export RUNTIME_ENTRY_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_entry_contract_v1.json"
export DECISION_MEMO_JSON="${ROOT_DIR}/docs/phase34_validation_agent_gate_decision_memo_v1.json"
export OPERATOR_GATE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_operator_gate_v1.json"
export APPROVAL_RECORD_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.json"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export DRY_RUN_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_request_packet_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_validation_agent_runtime_request_packet_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
runtime_entry_path = Path(os.environ["RUNTIME_ENTRY_JSON"])
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
    ("phase34_validation_agent_runtime_entry_contract", runtime_entry_path),
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
    runtime_entry = json.loads(runtime_entry_path.read_text())
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
    runtime_entry, decision_memo, operator_gate, approval_record, approval_contract, dry_run, policy, baseline, handoff, triage = {}, {}, {}, {}, {}, {}, {}, {}, {}, {}

request_scope = [
    "design_only_runtime_request_preparation",
    "manual_approval_required_precheck",
    "external_gate_submission_readiness",
]

submission_requirements = [
    "baseline marker current",
    "policy marker current",
    "dry-run marker current",
    "approval contract marker current",
    "approval record marker current",
    "approval rehearsal marker current",
    "operator gate marker current",
    "decision memo marker current",
    "runtime entry contract marker current",
    "handoff marker current",
]

evidence_bundle_summary = {
    "baseline_marker": baseline.get("marker", ""),
    "policy_marker": policy.get("marker", ""),
    "dry_run_marker": dry_run.get("marker", ""),
    "approval_contract_marker": approval_contract.get("marker", ""),
    "approval_record_marker": approval_record.get("marker", ""),
    "approval_rehearsal_marker": operator_gate.get("evidence_bundle", {}).get("approval_rehearsal_marker", ""),
    "operator_gate_marker": operator_gate.get("marker", ""),
    "decision_memo_marker": decision_memo.get("marker", ""),
    "runtime_entry_contract_marker": runtime_entry.get("marker", ""),
    "handoff_marker": handoff.get("marker", ""),
}
if triage_present:
    evidence_bundle_summary["triage_marker"] = triage.get("marker", "")

missing_evidence = [k for k, v in evidence_bundle_summary.items() if not v]

approver_review_points = [
    "baseline актуален",
    "policy актуален",
    "gate chain консистентна",
    "non-execution флаги сохранены",
    "runtime phase не открыта",
    "нет implicit/silent пути к исполнению",
]

explicit_non_authorizations = {
    "execution_authorized": False,
    "graph_write_authorized": False,
    "remediation_authorized": False,
    "runtime_phase_open": False,
}

rejection_conditions = [
    "missing evidence",
    "blocked policy/baseline/gate chain",
    "stale artifacts",
    "implicit approval path",
    "silent fallback to execution",
    "any runtime authorization flag not false",
]

blocked_detected = any(
    s.endswith("blocked") or s == "blocked"
    for s in [
        runtime_entry.get("status", ""),
        decision_memo.get("status", ""),
        operator_gate.get("status", ""),
        approval_record.get("status", ""),
        approval_contract.get("status", ""),
        dry_run.get("status", ""),
        policy.get("status", ""),
        baseline.get("baseline_status", ""),
        handoff.get("status", ""),
        triage.get("status", "") if triage_present else "",
    ] if s
)
notes_detected = any(
    "with_notes" in s
    for s in [
        runtime_entry.get("status", ""),
        decision_memo.get("status", ""),
        operator_gate.get("status", ""),
        approval_record.get("status", ""),
        approval_contract.get("status", ""),
        dry_run.get("status", ""),
        triage.get("status", "") if triage_present else "",
    ] if s
)

flags_all_false = all(value is False for value in explicit_non_authorizations.values())

if missing_required:
    status = "runtime_request_packet_blocked"
    reason = "safe_runtime_request_reference_missing"
elif missing_evidence or not flags_all_false:
    status = "runtime_request_packet_ready_with_notes"
    reason = "safe_runtime_request_reference_ready_with_notes"
elif blocked_detected or notes_detected:
    status = "runtime_request_packet_ready_with_notes"
    reason = "safe_runtime_request_reference_ready_with_notes"
else:
    status = "runtime_request_packet_ready"
    reason = "safe_runtime_request_reference_ready"

request_packet_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": missing_required,
    "missing_evidence": missing_evidence,
    "operator_message_ru": "Пакет запроса сформирован только для внешнего gate-review без запуска runtime.",
}

next_safe_step = {
    "step_ru": "Передать пакет на внешний review и обновить evidence при необходимости.",
    "control_ru": "Даже после review исполнение запрещено до отдельной разрешённой runtime-фазы.",
}

marker = f"KV_VALIDATION_AGENT_RUNTIME_REQUEST_PACKET_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_validation_agent_runtime_request_packet_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "request_packet_status": request_packet_status,
    "request_scope": request_scope,
    "submission_requirements": submission_requirements,
    "evidence_bundle_summary": evidence_bundle_summary,
    "approver_review_points": approver_review_points,
    "explicit_non_authorizations": explicit_non_authorizations,
    "rejection_conditions": rejection_conditions,
    "next_safe_step": next_safe_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Фаза 34 — ValidationAgent Runtime Request Packet v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "",
    "## request_packet_status",
]
for k, v in request_packet_status.items():
    lines.append(f"- {k}: {v}")

for sec_name, sec_list in [
    ("request_scope", request_scope),
    ("submission_requirements", submission_requirements),
    ("approver_review_points", approver_review_points),
    ("rejection_conditions", rejection_conditions),
]:
    lines += ["", f"## {sec_name}"]
    for item in sec_list:
        lines.append(f"- {item}")

lines += ["", "## evidence_bundle_summary"]
for k, v in evidence_bundle_summary.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## explicit_non_authorizations"]
for k, v in explicit_non_authorizations.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## next_safe_step"]
for k, v in next_safe_step.items():
    lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(lines) + "\n")
print("Готово: external gate submission packet / runtime request bundle сформирован (read-only).")
print(f"Маркер: {marker}")
PY
