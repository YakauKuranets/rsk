#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export APPROVAL_RECORD_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.json"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export DRY_RUN_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_rehearsal_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_validation_agent_approval_rehearsal_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
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
    approval_record = json.loads(approval_record_path.read_text())
    approval_contract = json.loads(approval_contract_path.read_text())
    dry_run = json.loads(dry_run_path.read_text())
    policy = json.loads(policy_path.read_text())
    baseline = json.loads(baseline_path.read_text())
    handoff = json.loads(handoff_path.read_text())
    triage = json.loads(triage_path.read_text()) if triage_present else {}
else:
    approval_record, approval_contract, dry_run, policy, baseline, handoff, triage = {}, {}, {}, {}, {}, {}, {}

decision_fields_required = approval_record.get("decision_fields") or [
    "decision_id",
    "decision_time",
    "operator_id",
    "artifacts_reviewed",
    "approved_scope",
    "explicit_non_scope",
    "evidence_refs",
    "decision_notes",
    "execution_authorized",
    "graph_write_authorized",
    "remediation_authorized",
]

sample_decision_packet = {
    "decision_id": "APR-REHEARSAL-0001",
    "decision_time": now,
    "operator_id": "operator_demo",
    "artifacts_reviewed": [
        "docs/phase34_validation_agent_approval_record_v1.json",
        "docs/phase34_validation_agent_approval_contract_v1.json",
        "docs/phase34_validation_agent_dry_run_v1.json",
        "docs/phase33_operator_policy_v1.json",
        "docs/phase33_baseline_freeze_v1.json",
        "docs/phase33_handoff_pack_v1.json",
    ],
    "approved_scope": [
        "Проверка полноты пакета решения оператора",
        "Проверка наличия evidence markers",
        "Подтверждение готовности gate-процесса без запуска исполнения",
    ],
    "explicit_non_scope": [
        "Runtime execution",
        "Remediation execution",
        "Graph writes",
        "Автоматическое одобрение или silent fallback к исполнению",
    ],
    "evidence_refs": [
        approval_contract.get("marker", ""),
        approval_record.get("marker", ""),
        dry_run.get("marker", ""),
        policy.get("marker", ""),
        baseline.get("marker", ""),
        handoff.get("marker", ""),
    ] + ([triage.get("marker", "")] if triage_present else []),
    "decision_notes": "Репетиция подтверждает формат пакета и готовность operator gate без активации runtime.",
    "execution_authorized": False,
    "graph_write_authorized": False,
    "remediation_authorized": False,
}

packet_missing_fields = [f for f in decision_fields_required if f not in sample_decision_packet]
packet_boolean_flags_ok = (
    sample_decision_packet.get("execution_authorized") is False
    and sample_decision_packet.get("graph_write_authorized") is False
    and sample_decision_packet.get("remediation_authorized") is False
)

required_evidence_list = [
    "approval_contract_marker",
    "dry_run_marker",
    "operator_policy_marker",
    "baseline_freeze_marker",
    "handoff_marker",
] + (["triage_marker"] if triage_present else [])

evidence_map = {
    "approval_contract_marker": approval_contract.get("marker", ""),
    "dry_run_marker": dry_run.get("marker", ""),
    "operator_policy_marker": policy.get("marker", ""),
    "baseline_freeze_marker": baseline.get("marker", ""),
    "handoff_marker": handoff.get("marker", ""),
}
if triage_present:
    evidence_map["triage_marker"] = triage.get("marker", "")

missing_evidence = [name for name in required_evidence_list if not evidence_map.get(name)]

packet_completeness_check = {
    "required_fields": decision_fields_required,
    "missing_fields": packet_missing_fields,
    "all_required_fields_present": len(packet_missing_fields) == 0,
    "non_execution_flags_valid": packet_boolean_flags_ok,
    "operator_message_ru": "Репетиция проверяет только форму пакета и обязательные поля.",
}

required_evidence_check = {
    "required_evidence": required_evidence_list,
    "present_evidence": evidence_map,
    "missing_evidence": missing_evidence,
    "evidence_sufficient_for_rehearsal": len(missing_evidence) == 0,
    "operator_message_ru": "Evidence проверяется на полноту для репетиции gate-процесса.",
}

operator_gate_check = {
    "policy_gate_present": bool(policy.get("marker")),
    "baseline_gate_present": bool(baseline.get("marker")),
    "handoff_gate_present": bool(handoff.get("marker")),
    "approval_contract_present": bool(approval_contract.get("marker")),
    "approval_record_present": bool(approval_record.get("marker")),
    "triage_gate_present": bool(triage.get("marker")) if triage_present else None,
    "gate_readiness_for_rehearsal_only": True,
    "operator_message_ru": "Gate-check в этой фазе подтверждает готовность процесса, но не даёт разрешение на исполнение.",
}

non_execution_confirmation = {
    "rehearsal_is_non_executable": True,
    "execution_authorized": False,
    "graph_write_authorized": False,
    "remediation_authorized": False,
    "runtime_ui_changes_permitted": False,
    "rehearsal_only_validates_packet_shape_and_operator_gate_readiness": True,
    "operator_message_ru": "Даже заполненный sample_decision_packet не является разрешением на исполнение.",
}

operator_rehearsal_notes = [
    "Репетиция носит исключительно read-only характер.",
    "После заполнения пакета оператор обязан сохранить execution_authorized=false.",
    "После заполнения пакета оператор обязан сохранить graph_write_authorized=false.",
    "После заполнения пакета оператор обязан сохранить remediation_authorized=false.",
    "Approval rehearsal проверяет полноту и readiness, а не запуск runtime-процесса.",
]

if missing_required:
    status = "approval_rehearsal_blocked"
    reason = "operator_packet_rehearsal_reference_missing"
else:
    has_notes = bool(packet_missing_fields or missing_evidence)
    upstream_states = [
        approval_record.get("status", ""),
        approval_contract.get("status", ""),
        dry_run.get("status", ""),
        policy.get("status", ""),
        baseline.get("baseline_status", ""),
        handoff.get("status", ""),
        triage.get("status", "") if triage_present else "",
    ]
    if has_notes or any((s.endswith("blocked") or s == "blocked" or "with_notes" in s) for s in upstream_states if s):
        status = "approval_rehearsal_ready_with_notes"
        reason = "operator_packet_rehearsal_ready_with_notes"
    else:
        status = "approval_rehearsal_ready"
        reason = "operator_packet_rehearsal_ready"

rehearsal_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": missing_required,
    "operator_message_ru": "Репетиция approval packet выполнена без запуска исполнения.",
}

next_safe_step = {
    "step_ru": "Использовать sample_decision_packet как шаблон операторского заполнения и повторно валидировать completeness/evidence.",
    "control_ru": "Даже после успешной репетиции execution остаётся запрещённым до отдельной разрешённой runtime-фазы.",
}

marker = f"KV_VALIDATION_AGENT_APPROVAL_REHEARSAL_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_validation_agent_approval_rehearsal_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "rehearsal_status": rehearsal_status,
    "packet_completeness_check": packet_completeness_check,
    "required_evidence_check": required_evidence_check,
    "operator_gate_check": operator_gate_check,
    "non_execution_confirmation": non_execution_confirmation,
    "sample_decision_packet": sample_decision_packet,
    "operator_rehearsal_notes": operator_rehearsal_notes,
    "next_safe_step": next_safe_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Фаза 34 — ValidationAgent Approval Rehearsal v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "",
    "## rehearsal_status",
]
for k, v in rehearsal_status.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## packet_completeness_check"]
for k, v in packet_completeness_check.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## required_evidence_check"]
for k, v in required_evidence_check.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## operator_gate_check"]
for k, v in operator_gate_check.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## non_execution_confirmation"]
for k, v in non_execution_confirmation.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## sample_decision_packet"]
for k, v in sample_decision_packet.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## operator_rehearsal_notes"]
for note in operator_rehearsal_notes:
    lines.append(f"- {note}")

lines += ["", "## next_safe_step"]
for k, v in next_safe_step.items():
    lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(lines) + "\n")
print("Готово: репетиция approval packet выполнена (read-only).")
print(f"Маркер: {marker}")
PY
