#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export APPROVAL_RECORD_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.json"
export APPROVAL_REHEARSAL_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_rehearsal_v1.json"
export DRY_RUN_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_operator_gate_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_validation_agent_operator_gate_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
approval_contract_path = Path(os.environ["APPROVAL_CONTRACT_JSON"])
approval_record_path = Path(os.environ["APPROVAL_RECORD_JSON"])
approval_rehearsal_path = Path(os.environ["APPROVAL_REHEARSAL_JSON"])
dry_run_path = Path(os.environ["DRY_RUN_JSON"])
policy_path = Path(os.environ["OP_POLICY_JSON"])
baseline_path = Path(os.environ["BASELINE_JSON"])
handoff_path = Path(os.environ["HANDOFF_JSON"])
triage_path = Path(os.environ["TRIAGE_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

required = [
    ("phase34_validation_agent_approval_contract", approval_contract_path),
    ("phase34_validation_agent_approval_record", approval_record_path),
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
    approval_contract = json.loads(approval_contract_path.read_text())
    approval_record = json.loads(approval_record_path.read_text())
    approval_rehearsal = json.loads(approval_rehearsal_path.read_text())
    dry_run = json.loads(dry_run_path.read_text())
    policy = json.loads(policy_path.read_text())
    baseline = json.loads(baseline_path.read_text())
    handoff = json.loads(handoff_path.read_text())
    triage = json.loads(triage_path.read_text()) if triage_present else {}
else:
    approval_contract, approval_record, approval_rehearsal, dry_run, policy, baseline, handoff, triage = {}, {}, {}, {}, {}, {}, {}, {}

required_gate_checks = [
    "baseline reference current",
    "operator policy current",
    "dry-run current",
    "approval contract current",
    "approval record current",
    "approval rehearsal current",
    "handoff consistency confirmed",
]

evidence_bundle = {
    "baseline_marker": baseline.get("marker", ""),
    "policy_marker": policy.get("marker", ""),
    "dry_run_marker": dry_run.get("marker", ""),
    "approval_contract_marker": approval_contract.get("marker", ""),
    "approval_record_marker": approval_record.get("marker", ""),
    "approval_rehearsal_marker": approval_rehearsal.get("marker", ""),
    "handoff_marker": handoff.get("marker", ""),
}
if triage_present:
    evidence_bundle["triage_marker"] = triage.get("marker", "")

missing_evidence = [k for k, v in evidence_bundle.items() if not v]

sample_packet = approval_rehearsal.get("sample_decision_packet") or {}
execution_flag_ok = sample_packet.get("execution_authorized") is False
write_flag_ok = sample_packet.get("graph_write_authorized") is False
remediation_flag_ok = sample_packet.get("remediation_authorized") is False

operator_checklist = [
    "Проверить актуальность baseline.",
    "Проверить актуальность policy.",
    "Проверить completeness evidence bundle.",
    "Проверить, что execution_authorized остаётся false.",
    "Проверить, что graph_write_authorized остаётся false.",
    "Проверить, что remediation_authorized остаётся false.",
]

gate_failure_conditions = [
    "missing evidence",
    "stale baseline reference",
    "blocked operator policy",
    "blocked dry-run / approval scaffolds",
    "любое implicit или silent разрешение на исполнение",
]

gate_pass_conditions = [
    "Все required gate checks подтверждены.",
    "Evidence bundle полон и непротиворечив.",
    "execution_authorized=false подтверждено.",
    "graph_write_authorized=false подтверждено.",
    "remediation_authorized=false подтверждено.",
    "Operator gate используется только как проверочный слой и не заменяет policy/contract.",
]

non_execution_confirmation = {
    "operator_gate_is_read_only": True,
    "runtime_execution_permitted": False,
    "remediation_permitted": False,
    "graph_writes_permitted": False,
    "gate_does_not_replace_approval_contract": True,
    "gate_does_not_replace_operator_policy": True,
    "full_evidence_bundle_still_not_execution_permission": True,
    "operator_message_ru": "Даже при полном evidence bundle исполнение запрещено до отдельной runtime-фазы.",
}

blocked_detected = any(
    s.endswith("blocked") or s == "blocked"
    for s in [
        policy.get("status", ""),
        baseline.get("baseline_status", ""),
        dry_run.get("status", ""),
        approval_contract.get("status", ""),
        approval_record.get("status", ""),
        approval_rehearsal.get("status", ""),
        handoff.get("status", ""),
        triage.get("status", "") if triage_present else "",
    ]
    if s
)

notes_detected = any(
    "with_notes" in s
    for s in [
        dry_run.get("status", ""),
        approval_contract.get("status", ""),
        approval_record.get("status", ""),
        approval_rehearsal.get("status", ""),
        triage.get("status", "") if triage_present else "",
    ]
)

if missing_required:
    status = "operator_gate_blocked"
    reason = "safe_operator_gate_reference_missing"
elif missing_evidence:
    status = "operator_gate_ready_with_notes"
    reason = "safe_operator_gate_reference_ready_with_notes"
elif blocked_detected or notes_detected or not (execution_flag_ok and write_flag_ok and remediation_flag_ok):
    status = "operator_gate_ready_with_notes"
    reason = "safe_operator_gate_reference_ready_with_notes"
else:
    status = "operator_gate_ready"
    reason = "safe_operator_gate_reference_ready"

gate_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": missing_required,
    "missing_evidence": missing_evidence,
    "required_gate_checks_count": len(required_gate_checks),
    "operator_message_ru": "Operator gate сформирован как read-only checklist/evidence слой.",
}

next_safe_step = {
    "step_ru": "Пройти operator_checklist и подтвердить gate_pass_conditions без запуска исполнения.",
    "control_ru": "Любой переход к manual_approval_required runtime возможен только в отдельной разрешённой фазе.",
}

marker = f"KV_VALIDATION_AGENT_OPERATOR_GATE_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_validation_agent_operator_gate_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "gate_status": gate_status,
    "required_gate_checks": required_gate_checks,
    "evidence_bundle": evidence_bundle,
    "operator_checklist": operator_checklist,
    "gate_failure_conditions": gate_failure_conditions,
    "gate_pass_conditions": gate_pass_conditions,
    "non_execution_confirmation": non_execution_confirmation,
    "next_safe_step": next_safe_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Фаза 34 — ValidationAgent Operator Gate v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "",
    "## gate_status",
]
for k, v in gate_status.items():
    lines.append(f"- {k}: {v}")

for sec_name, sec_values in [
    ("required_gate_checks", required_gate_checks),
    ("operator_checklist", operator_checklist),
    ("gate_failure_conditions", gate_failure_conditions),
    ("gate_pass_conditions", gate_pass_conditions),
]:
    lines += ["", f"## {sec_name}"]
    for value in sec_values:
        lines.append(f"- {value}")

lines += ["", "## evidence_bundle"]
for k, v in evidence_bundle.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## non_execution_confirmation"]
for k, v in non_execution_confirmation.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## next_safe_step"]
for k, v in next_safe_step.items():
    lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(lines) + "\n")
print("Готово: operator gate checklist и evidence bundle сформированы (read-only).")
print(f"Маркер: {marker}")
PY
