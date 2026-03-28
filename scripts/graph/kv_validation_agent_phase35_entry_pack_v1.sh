#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export REVIEW_CYCLE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_review_cycle_bundle_v1.json"
export RUNTIME_RESPONSE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_review_response_v1.json"
export RUNTIME_REQUEST_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_request_packet_v1.json"
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
export OUT_JSON="${ROOT_DIR}/docs/phase35_entry_pack_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase35_entry_pack_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
review_cycle_path = Path(os.environ["REVIEW_CYCLE_JSON"])
runtime_response_path = Path(os.environ["RUNTIME_RESPONSE_JSON"])
runtime_request_path = Path(os.environ["RUNTIME_REQUEST_JSON"])
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
    ("review_cycle", review_cycle_path),
    ("runtime_response", runtime_response_path),
    ("runtime_request", runtime_request_path),
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
    review_cycle = json.loads(review_cycle_path.read_text())
    runtime_response = json.loads(runtime_response_path.read_text())
    runtime_request = json.loads(runtime_request_path.read_text())
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
    review_cycle=runtime_response=runtime_request=runtime_entry=decision_memo=operator_gate=approval_record=approval_contract=dry_run=policy=baseline=handoff=triage={}

closure_summary = {
    "phase34_completed_segments": ["34.1-34.15 (scaffold chain)"],
    "review_cycle_status": review_cycle.get("status", ""),
    "operator_gate_status": operator_gate.get("status", ""),
    "dry_run_status": dry_run.get("status", ""),
    "baseline_status": baseline.get("baseline_status", ""),
    "policy_status": policy.get("status", ""),
    "operator_message_ru": "Closure summary отражает состояние scaffold-цепочки перед входом в Phase 35.",
}

validated_artifact_chain = {
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
    "review_cycle_bundle_marker": review_cycle.get("marker", ""),
    "handoff_marker": handoff.get("marker", ""),
}
if triage_present:
    validated_artifact_chain["triage_marker"] = triage.get("marker", "")

missing_chain = [k for k, v in validated_artifact_chain.items() if not v]

flags = {
    "execution_authorized": False,
    "graph_write_authorized": False,
    "remediation_authorized": False,
    "runtime_phase_open": False,
}

entry_conditions = {
    "all_required_artifacts_present": len(missing_required) == 0,
    "validated_chain_consistent": len(missing_chain) == 0,
    "non_execution_flags_remain_false": all(v is False for v in flags.values()),
    "policy_baseline_chain_not_broken": bool(policy.get("marker") and baseline.get("marker")),
    "phase35_start_only_design_or_control_mode": True,
}

open_notes = [
    "Текущая цепочка содержит статусы with_notes/blocked в upstream артефактах.",
    "Перед runtime требуется отдельная разрешённая фаза и внешний gate.",
]

hard_stops = [
    "missing artifact chain",
    "implicit approval path",
    "silent fallback to execution",
    "any runtime authorization flag not false",
    "broken policy/baseline references",
]

states = [
    review_cycle.get("status", ""), runtime_response.get("status", ""), runtime_request.get("status", ""),
    runtime_entry.get("status", ""), decision_memo.get("status", ""), operator_gate.get("status", ""),
    approval_record.get("status", ""), approval_contract.get("status", ""), dry_run.get("status", ""),
    policy.get("status", ""), baseline.get("baseline_status", ""), handoff.get("status", ""),
    triage.get("status", "") if triage_present else ""
]
blocked_detected = any(s.endswith("blocked") or s == "blocked" for s in states if s)
notes_detected = any("with_notes" in s for s in states if s)

if missing_required:
    status = "phase35_entry_blocked"
    reason = "safe_phase35_reference_missing"
elif missing_chain or blocked_detected or notes_detected:
    status = "phase35_entry_ready_with_notes"
    reason = "safe_phase35_reference_ready_with_notes"
else:
    status = "phase35_entry_ready"
    reason = "safe_phase35_reference_ready"

recommended_phase35_start_mode = "design_only_with_notes" if status == "phase35_entry_ready_with_notes" else "design_only"

phase35_entry_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": missing_required,
    "missing_chain_markers": missing_chain,
    "operator_message_ru": "Phase35 entry pack сформирован как финальный pre-phase35 closure пакет без исполнения.",
}

non_execution_confirmation = {
    **flags,
    "entry_pack_does_not_open_runtime": True,
    "entry_pack_does_not_remove_policy_baseline_gates": True,
    "operator_message_ru": "Даже готовность к Phase 35 не является разрешением на runtime execution.",
}

next_safe_step = {
    "step_ru": "Использовать пакет для design/control старта Phase 35 и закрыть notes перед любыми runtime-инициативами.",
    "control_ru": "Любой runtime по-прежнему допускается только в отдельной разрешённой runtime-фазе.",
}

marker = f"KV_VALIDATION_AGENT_PHASE35_ENTRY_PACK_V1|status={status}|reason={reason}"

payload = {
    "version": "phase35_entry_pack_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "phase35_entry_status": phase35_entry_status,
    "closure_summary": closure_summary,
    "validated_artifact_chain": validated_artifact_chain,
    "entry_conditions": entry_conditions,
    "open_notes": open_notes,
    "hard_stops": hard_stops,
    "non_execution_confirmation": non_execution_confirmation,
    "recommended_phase35_start_mode": recommended_phase35_start_mode,
    "next_safe_step": next_safe_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Фаза 35 — Entry Pack v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    f"- recommended_phase35_start_mode: **{recommended_phase35_start_mode}**",
    "",
    "## phase35_entry_status",
]
for k, v in phase35_entry_status.items():
    lines.append(f"- {k}: {v}")

for sec_name, sec_val in [
    ("closure_summary", closure_summary),
    ("validated_artifact_chain", validated_artifact_chain),
    ("entry_conditions", entry_conditions),
    ("non_execution_confirmation", non_execution_confirmation),
    ("next_safe_step", next_safe_step),
]:
    lines += ["", f"## {sec_name}"]
    for k, v in sec_val.items():
        lines.append(f"- {k}: {v}")

for sec_name, sec_list in [("open_notes", open_notes), ("hard_stops", hard_stops)]:
    lines += ["", f"## {sec_name}"]
    for item in sec_list:
        lines.append(f"- {item}")

out_md.write_text("\n".join(lines) + "\n")
print("Готово: final pre-phase35 closure / phase35 entry pack сформирован (read-only).")
print(f"Маркер: {marker}")
PY
