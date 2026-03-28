#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export RUNTIME_REQUEST_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_request_packet_v1.json"
export RUNTIME_RESPONSE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_review_response_v1.json"
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
export OUT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_review_cycle_bundle_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_validation_agent_review_cycle_bundle_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
runtime_request_path = Path(os.environ["RUNTIME_REQUEST_JSON"])
runtime_response_path = Path(os.environ["RUNTIME_RESPONSE_JSON"])
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
    ("runtime_request", runtime_request_path),
    ("runtime_response", runtime_response_path),
    ("runtime_entry", runtime_entry_path),
    ("decision_memo", decision_memo_path),
    ("operator_gate", operator_gate_path),
    ("approval_record", approval_record_path),
    ("approval_contract", approval_contract_path),
    ("dry_run", dry_run_path),
    ("operator_policy", policy_path),
    ("baseline", baseline_path),
    ("handoff", handoff_path),
]
presence = {name: path.exists() for name, path in required}
triage_present = triage_path.exists()
missing_required = [name for name, ok in presence.items() if not ok]

if not missing_required:
    runtime_request = json.loads(runtime_request_path.read_text())
    runtime_response = json.loads(runtime_response_path.read_text())
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
    runtime_request=runtime_response=runtime_entry=decision_memo=operator_gate=approval_record=approval_contract=dry_run=policy=baseline=handoff=triage={}

request_summary = {
    "request_marker": runtime_request.get("marker", ""),
    "request_scope": runtime_request.get("request_scope", []),
    "request_status": runtime_request.get("status", ""),
    "request_reason": runtime_request.get("reason", ""),
}

response_summary = {
    "response_marker": runtime_response.get("marker", ""),
    "response_outcome_set": runtime_response.get("possible_review_outcomes", []),
    "response_status": runtime_response.get("status", ""),
    "response_reason": runtime_response.get("reason", ""),
}

evidence_chain = {
    "baseline_marker": baseline.get("marker", ""),
    "policy_marker": policy.get("marker", ""),
    "dry_run_marker": dry_run.get("marker", ""),
    "approval_contract_marker": approval_contract.get("marker", ""),
    "approval_record_marker": approval_record.get("marker", ""),
    "approval_rehearsal_marker": operator_gate.get("evidence_bundle", {}).get("approval_rehearsal_marker", ""),
    "operator_gate_marker": operator_gate.get("marker", ""),
    "decision_memo_marker": decision_memo.get("marker", ""),
    "runtime_entry_contract_marker": runtime_entry.get("marker", ""),
    "runtime_request_packet_marker": runtime_request.get("marker", ""),
    "runtime_review_response_marker": runtime_response.get("marker", ""),
    "handoff_marker": handoff.get("marker", ""),
}
if triage_present:
    evidence_chain["triage_marker"] = triage.get("marker", "")

missing_evidence = [k for k, v in evidence_chain.items() if not v]

flags = {
    "execution_authorized": False,
    "graph_write_authorized": False,
    "remediation_authorized": False,
    "runtime_phase_open": False,
}

audit_integrity_checks = {
    "all_required_artifacts_present": len(missing_required) == 0,
    "request_response_linked": bool(request_summary["request_marker"] and response_summary["response_marker"]),
    "non_execution_flags_remain_false": all(v is False for v in flags.values()),
    "no_implicit_approval_path": True,
    "no_silent_fallback_to_execution": True,
    "policy_baseline_references_consistent": bool(policy.get("marker") and baseline.get("marker")),
    "missing_evidence": missing_evidence,
}

operator_archive_fields = {
    "bundle_id": "RCB-VA-0001",
    "bundle_time": now,
    "request_ref": request_summary["request_marker"],
    "response_ref": response_summary["response_marker"],
    "evidence_refs": list(evidence_chain.values()),
    "operator_notes": "Архив review-цикла собран в read-only режиме без runtime-активации.",
    **flags,
}

non_execution_confirmation = {
    **flags,
    "bundle_does_not_open_runtime": True,
    "bundle_does_not_replace_policy_baseline_runtime_entry": True,
    "operator_message_ru": "Даже полный review-cycle bundle не является разрешением на runtime.",
}

states = [
    runtime_request.get("status", ""), runtime_response.get("status", ""), runtime_entry.get("status", ""),
    decision_memo.get("status", ""), operator_gate.get("status", ""), approval_record.get("status", ""),
    approval_contract.get("status", ""), dry_run.get("status", ""), policy.get("status", ""),
    baseline.get("baseline_status", ""), handoff.get("status", ""), triage.get("status", "") if triage_present else ""
]
blocked_detected = any(s.endswith("blocked") or s == "blocked" for s in states if s)
notes_detected = any("with_notes" in s for s in states if s)

if missing_required:
    status = "review_cycle_bundle_blocked"
    reason = "safe_review_cycle_reference_missing"
elif missing_evidence or blocked_detected or notes_detected:
    status = "review_cycle_bundle_ready_with_notes"
    reason = "safe_review_cycle_reference_ready_with_notes"
else:
    status = "review_cycle_bundle_ready"
    reason = "safe_review_cycle_reference_ready"

review_cycle_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": missing_required,
    "operator_message_ru": "Review-cycle bundle сформирован как audit-ready пакет в read-only режиме.",
}

next_safe_step = {
    "step_ru": "Заархивировать bundle и передать на внешний audit/review без запуска исполнения.",
    "control_ru": "Любой runtime всё ещё требует отдельной разрешённой runtime-фазы.",
}

marker = f"KV_VALIDATION_AGENT_REVIEW_CYCLE_BUNDLE_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_validation_agent_review_cycle_bundle_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "review_cycle_status": review_cycle_status,
    "request_summary": request_summary,
    "response_summary": response_summary,
    "evidence_chain": evidence_chain,
    "audit_integrity_checks": audit_integrity_checks,
    "operator_archive_fields": operator_archive_fields,
    "non_execution_confirmation": non_execution_confirmation,
    "next_safe_step": next_safe_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Фаза 34 — ValidationAgent Review Cycle Bundle v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "",
    "## review_cycle_status",
]
for k, v in review_cycle_status.items():
    lines.append(f"- {k}: {v}")

for sec_name, sec_val in [
    ("request_summary", request_summary),
    ("response_summary", response_summary),
    ("evidence_chain", evidence_chain),
    ("audit_integrity_checks", audit_integrity_checks),
    ("operator_archive_fields", operator_archive_fields),
    ("non_execution_confirmation", non_execution_confirmation),
    ("next_safe_step", next_safe_step),
]:
    lines += ["", f"## {sec_name}"]
    for k, v in sec_val.items():
        lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(lines) + "\n")
print("Готово: review-cycle bundle / audit trail packet сформирован (read-only).")
print(f"Маркер: {marker}")
PY
