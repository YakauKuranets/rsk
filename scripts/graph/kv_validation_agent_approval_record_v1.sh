#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export DRY_RUN_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
approval_contract_path = Path(os.environ["APPROVAL_CONTRACT_JSON"])
dry_run_path = Path(os.environ["DRY_RUN_JSON"])
policy_path = Path(os.environ["OP_POLICY_JSON"])
handoff_path = Path(os.environ["HANDOFF_JSON"])
baseline_path = Path(os.environ["BASELINE_JSON"])
triage_path = Path(os.environ["TRIAGE_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

required = [
    ("phase34_validation_agent_approval_contract", approval_contract_path),
    ("phase34_validation_agent_dry_run", dry_run_path),
    ("phase33_operator_policy", policy_path),
    ("phase33_handoff_pack", handoff_path),
    ("phase33_baseline_freeze", baseline_path),
]
required_presence = {name: path.exists() for name, path in required}
triage_present = triage_path.exists()
missing_required = [name for name, ok in required_presence.items() if not ok]

if not missing_required:
    approval_contract = json.loads(approval_contract_path.read_text())
    dry_run = json.loads(dry_run_path.read_text())
    policy = json.loads(policy_path.read_text())
    handoff = json.loads(handoff_path.read_text())
    baseline = json.loads(baseline_path.read_text())
    triage = json.loads(triage_path.read_text()) if triage_present else {}
else:
    approval_contract, dry_run, policy, handoff, baseline, triage = {}, {}, {}, {}, {}, {}

required_evidence = [
    {"name": "approval_contract_marker", "value": approval_contract.get("marker", ""), "required": True},
    {"name": "dry_run_marker", "value": dry_run.get("marker", ""), "required": True},
    {"name": "operator_policy_marker", "value": policy.get("marker", ""), "required": True},
    {"name": "baseline_freeze_marker", "value": baseline.get("marker", ""), "required": True},
    {"name": "handoff_marker", "value": handoff.get("marker", ""), "required": True},
]
if triage_present:
    required_evidence.append(
        {"name": "triage_marker", "value": triage.get("marker", ""), "required": True}
    )

missing_evidence = [e["name"] for e in required_evidence if not e["value"]]

decision_fields = [
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

non_executable_confirmation = {
    "approval_record_is_format_only": True,
    "execution_authorized": False,
    "graph_write_authorized": False,
    "remediation_authorized": False,
    "approval_record_does_not_start_execution": True,
    "approval_record_does_not_remove_policy_or_baseline_gates": True,
    "post_approval_execution_remains_forbidden_until_separate_runtime_phase": True,
    "operator_message_ru": "Даже после approval исполнение запрещено до отдельной разрешённой runtime-фазы.",
}

post_approval_restrictions = [
    "Исполнение не запускается автоматически после approval record.",
    "Remediation по-прежнему запрещён.",
    "Graph writes по-прежнему запрещены.",
    "Policy/baseline/handoff gates остаются обязательными.",
    "Approval record открывает только допустимость будущего рассмотрения, но не исполнение.",
]

operator_confirmation_requirements = [
    "Оператор обязан явно подтвердить границы approved_scope и explicit_non_scope.",
    "Оператор обязан подтвердить, что execution_authorized=false.",
    "Оператор обязан подтвердить, что graph_write_authorized=false.",
    "Оператор обязан подтвердить, что remediation_authorized=false.",
    "Оператор обязан приложить ссылки на обязательные evidence markers.",
]

decision_scope = {
    "scope_type": "manual_approval_required_packet_format_only",
    "scope_note_ru": "Пакет решения описывает формат ручного approval и не выполняет никаких действий.",
    "allowed_outcome": "Только фиксация операторского решения и ссылок на evidence.",
    "forbidden_outcome": "Любое runtime-исполнение, remediation и graph writes.",
}

if missing_required:
    status = "approval_record_blocked"
    reason = "safe_approval_record_reference_missing"
elif missing_evidence:
    status = "approval_record_ready_with_notes"
    reason = "safe_approval_record_ready_with_notes"
else:
    raw_states = [
        approval_contract.get("status", ""),
        dry_run.get("status", ""),
        policy.get("status", ""),
        baseline.get("baseline_status", ""),
        handoff.get("status", ""),
        triage.get("status", "") if triage_present else "",
    ]
    has_blocked = any(s.endswith("blocked") or s == "blocked" for s in raw_states if s)
    has_notes = any("with_notes" in s for s in raw_states if s)
    if has_blocked or has_notes:
        status = "approval_record_ready_with_notes"
        reason = "safe_approval_record_ready_with_notes"
    else:
        status = "approval_record_ready"
        reason = "safe_approval_record_ready"

approval_record_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": missing_required,
    "missing_required_evidence": missing_evidence,
    "input_presence": {
        **required_presence,
        "phase34_operator_backlog_triage_optional": triage_present,
    },
    "operator_message_ru": "Approval record сформирован как read-only шаблон операторского решения.",
}

next_safe_step = {
    "step_ru": "Заполнить decision_fields оператором и сохранить execution_authorized=false, graph_write_authorized=false, remediation_authorized=false.",
    "control_ru": "Перед любым будущим runtime-треком требуется отдельная разрешённая фаза и явный внешний gate.",
}

marker = f"KV_VALIDATION_AGENT_APPROVAL_RECORD_V1|status={status}|reason={reason}"

payload = {
    "version": "phase34_validation_agent_approval_record_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "approval_record_status": approval_record_status,
    "decision_scope": decision_scope,
    "required_evidence": required_evidence,
    "decision_fields": decision_fields,
    "operator_confirmation_requirements": operator_confirmation_requirements,
    "non_executable_confirmation": non_executable_confirmation,
    "post_approval_restrictions": post_approval_restrictions,
    "next_safe_step": next_safe_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

lines = [
    "# Фаза 34 — ValidationAgent Approval Record v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- status: **{status}**",
    f"- reason: **{reason}**",
    "",
    "## approval_record_status",
]
for k, v in approval_record_status.items():
    if isinstance(v, dict):
        lines.append(f"- {k}:")
        for ik, iv in v.items():
            lines.append(f"  - {ik}: {iv}")
    elif isinstance(v, list):
        lines.append(f"- {k}: {v}")
    else:
        lines.append(f"- {k}: {v}")

lines += ["", "## decision_scope"]
for k, v in decision_scope.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## required_evidence"]
for item in required_evidence:
    lines.append(f"- {item['name']}: required={item['required']} value={item['value']}")

lines += ["", "## decision_fields"]
for field in decision_fields:
    lines.append(f"- {field}")

lines += ["", "## operator_confirmation_requirements"]
for req in operator_confirmation_requirements:
    lines.append(f"- {req}")

lines += ["", "## non_executable_confirmation"]
for k, v in non_executable_confirmation.items():
    lines.append(f"- {k}: {v}")

lines += ["", "## post_approval_restrictions"]
for item in post_approval_restrictions:
    lines.append(f"- {item}")

lines += ["", "## next_safe_step"]
for k, v in next_safe_step.items():
    lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(lines) + "\n")
print(f"Готово: сформирован approval record (read-only).")
print(f"Маркер: {marker}")
PY
