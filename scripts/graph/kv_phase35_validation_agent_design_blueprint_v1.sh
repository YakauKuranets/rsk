#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export ENTRY_PACK_JSON="${ROOT_DIR}/docs/phase35_entry_pack_v1.json"
export REVIEW_CYCLE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_review_cycle_bundle_v1.json"
export RUNTIME_ENTRY_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_entry_contract_v1.json"
export DECISION_MEMO_JSON="${ROOT_DIR}/docs/phase34_validation_agent_gate_decision_memo_v1.json"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export DRY_RUN_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase35_validation_agent_design_blueprint_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase35_validation_agent_design_blueprint_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
entry_pack_path = Path(os.environ["ENTRY_PACK_JSON"])
review_cycle_path = Path(os.environ["REVIEW_CYCLE_JSON"])
runtime_entry_path = Path(os.environ["RUNTIME_ENTRY_JSON"])
decision_memo_path = Path(os.environ["DECISION_MEMO_JSON"])
approval_contract_path = Path(os.environ["APPROVAL_CONTRACT_JSON"])
dry_run_path = Path(os.environ["DRY_RUN_JSON"])
policy_path = Path(os.environ["POLICY_JSON"])
baseline_path = Path(os.environ["BASELINE_JSON"])
handoff_path = Path(os.environ["HANDOFF_JSON"])
triage_path = Path(os.environ["TRIAGE_JSON"])
out_json_path = Path(os.environ["OUT_JSON"])
out_md_path = Path(os.environ["OUT_MD"])

required = [
    ("phase35_entry_pack", entry_pack_path),
    ("review_cycle_bundle", review_cycle_path),
    ("runtime_entry_contract", runtime_entry_path),
    ("decision_memo", decision_memo_path),
    ("approval_contract", approval_contract_path),
    ("dry_run", dry_run_path),
    ("operator_policy", policy_path),
    ("baseline_freeze", baseline_path),
    ("handoff_pack", handoff_path),
]

presence = {name: path.exists() for name, path in required}
missing_required = [name for name, ok in presence.items() if not ok]
triage_present = triage_path.exists()

if missing_required:
    entry_pack = review_cycle = runtime_entry = decision_memo = approval_contract = dry_run = policy = baseline = handoff = triage = {}
else:
    entry_pack = json.loads(entry_pack_path.read_text())
    review_cycle = json.loads(review_cycle_path.read_text())
    runtime_entry = json.loads(runtime_entry_path.read_text())
    decision_memo = json.loads(decision_memo_path.read_text())
    approval_contract = json.loads(approval_contract_path.read_text())
    dry_run = json.loads(dry_run_path.read_text())
    policy = json.loads(policy_path.read_text())
    baseline = json.loads(baseline_path.read_text())
    handoff = json.loads(handoff_path.read_text())
    triage = json.loads(triage_path.read_text()) if triage_present else {}

validated_refs = {
    "phase35_entry_pack_marker": entry_pack.get("marker", ""),
    "review_cycle_marker": review_cycle.get("marker", ""),
    "runtime_entry_marker": runtime_entry.get("marker", ""),
    "decision_memo_marker": decision_memo.get("marker", ""),
    "approval_contract_marker": approval_contract.get("marker", ""),
    "dry_run_marker": dry_run.get("marker", ""),
    "policy_marker": policy.get("marker", ""),
    "baseline_marker": baseline.get("marker", ""),
    "handoff_marker": handoff.get("marker", ""),
}
if triage_present:
    validated_refs["triage_marker"] = triage.get("marker", "")

missing_markers = [k for k, v in validated_refs.items() if not v]

states = [
    entry_pack.get("status", ""),
    review_cycle.get("status", ""),
    runtime_entry.get("status", ""),
    decision_memo.get("status", ""),
    approval_contract.get("status", ""),
    dry_run.get("status", ""),
    policy.get("status", ""),
    baseline.get("baseline_status", ""),
    handoff.get("status", ""),
]
if triage_present:
    states.append(triage.get("status", ""))

blocked_detected = any((s.endswith("blocked") or s == "blocked") for s in states if s)
notes_detected = any("with_notes" in s for s in states if s)

if missing_required:
    status = "phase35_blueprint_blocked"
    reason = "safe_phase35_design_reference_missing"
elif missing_markers:
    status = "phase35_blueprint_blocked"
    reason = "safe_phase35_design_reference_missing"
elif blocked_detected or notes_detected:
    status = "phase35_blueprint_ready_with_notes"
    reason = "safe_phase35_design_reference_ready_with_notes"
else:
    status = "phase35_blueprint_ready"
    reason = "safe_phase35_design_reference_ready"

phase35_start_mode = "design_only_with_notes" if status == "phase35_blueprint_ready_with_notes" else "design_only"

flags = {
    "execution_authorized": False,
    "graph_write_authorized": False,
    "remediation_authorized": False,
    "runtime_phase_open": False,
}

phase35_blueprint_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": missing_required,
    "missing_reference_markers": missing_markers,
    "operator_message_ru": "Blueprint сформирован только как design-only архитектурный слой Phase 35.",
}

design_scope = {
    "scope_ru": "Архитектурный дизайн ValidationAgent без исполнения и без побочных эффектов.",
    "explicit_non_goals_ru": [
        "Запуск runtime execution.",
        "Любые graph writes.",
        "Любой remediation/backfill.",
        "Активация ValidationAgent в runtime режиме.",
    ],
}

proposed_agent_layers = [
    "artifact intake layer",
    "policy reasoning layer",
    "dry-run recommendation layer",
    "approval interface layer",
    "audit/evidence layer",
    "future runtime boundary layer",
]

control_boundaries = [
    "policy boundary",
    "baseline boundary",
    "approval boundary",
    "audit boundary",
    "runtime boundary",
]

non_execution_architecture_rules = [
    "no runtime execution",
    "no graph mutation",
    "no remediation",
    "no hidden side effects",
    "no policy bypass",
    "no baseline bypass",
    "no implicit transition to runtime",
]

future_runtime_separation = {
    "rules": [
        "runtime logic must remain isolated from design-only artifacts",
        "any future runtime requires separate gate and separate phase",
        "approval does not imply execution",
        "evidence chain must remain intact before any future runtime activation",
    ],
    "operator_message_ru": "Даже готовый blueprint не является разрешением на runtime.",
}

recommended_first_design_slice = {
    "slice_name": "artifact_intake_and_policy_reasoning_contract_v1",
    "slice_goal_ru": "Определить контракт входных артефактов и policy reasoning интерфейс без runtime вызовов.",
    "safe_output_ru": "Только спецификация интерфейсов, инвариантов и контрольных проверок в read-only виде.",
}

marker = f"KV_PHASE35_VALIDATION_AGENT_DESIGN_BLUEPRINT_V1|status={status}|reason={reason}"

payload = {
    "version": "phase35_validation_agent_design_blueprint_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "phase35_blueprint_status": phase35_blueprint_status,
    "phase35_start_mode": phase35_start_mode,
    "design_scope": design_scope,
    "proposed_agent_layers": proposed_agent_layers,
    "control_boundaries": control_boundaries,
    "non_execution_architecture_rules": non_execution_architecture_rules,
    "future_runtime_separation": future_runtime_separation,
    "recommended_first_design_slice": recommended_first_design_slice,
    "non_execution_confirmation": {
        **flags,
        "blueprint_does_not_open_runtime": True,
        "blueprint_does_not_remove_policy_baseline_gates": True,
        "operator_message_ru": "Runtime execution остаётся запрещённым до отдельной разрешённой runtime-фазы.",
    },
    "validated_reference_chain": validated_refs,
}

out_json_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

md_lines = [
    "# Фаза 35.1 — ValidationAgent Design Blueprint v1",
    "",
    f"Сформировано: {now}",
    "",
    "Документ фиксирует только design-only архитектурный blueprint. Исполнение не разрешено.",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- Статус: **{status}**",
    f"- Причина: **{reason}**",
    f"- Режим старта Phase 35: **{phase35_start_mode}**",
    "",
    "## phase35_blueprint_status",
]
for k, v in phase35_blueprint_status.items():
    md_lines.append(f"- {k}: {v}")

md_lines += ["", "## phase35_start_mode", f"- {phase35_start_mode}"]

md_lines += ["", "## design_scope"]
for k, v in design_scope.items():
    md_lines.append(f"- {k}: {v}")

md_lines += ["", "## proposed_agent_layers"]
layer_labels_ru = {
    "artifact intake layer": "слой приёма и валидации артефактов",
    "policy reasoning layer": "слой policy-анализа и выводов",
    "dry-run recommendation layer": "слой рекомендаций dry-run режима",
    "approval interface layer": "слой интерфейса approval-процесса",
    "audit/evidence layer": "слой аудита и evidence-цепочки",
    "future runtime boundary layer": "слой границы с будущим runtime-контуром",
}
for item in proposed_agent_layers:
    md_lines.append(f"- {layer_labels_ru.get(item, item)}")

md_lines += ["", "## control_boundaries"]
boundary_labels_ru = {
    "policy boundary": "граница policy-контроля",
    "baseline boundary": "граница baseline-контроля",
    "approval boundary": "граница approval-контроля",
    "audit boundary": "граница audit-контроля",
    "runtime boundary": "граница runtime-контура",
}
for item in control_boundaries:
    md_lines.append(f"- {boundary_labels_ru.get(item, item)}")

md_lines += ["", "## non_execution_architecture_rules"]
rule_labels_ru = {
    "no runtime execution": "запрещено runtime-исполнение",
    "no graph mutation": "запрещены изменения графа",
    "no remediation": "запрещён remediation",
    "no hidden side effects": "запрещены скрытые побочные эффекты",
    "no policy bypass": "запрещён обход policy",
    "no baseline bypass": "запрещён обход baseline",
    "no implicit transition to runtime": "запрещён неявный переход в runtime",
}
for item in non_execution_architecture_rules:
    md_lines.append(f"- {rule_labels_ru.get(item, item)}")

md_lines += ["", "## future_runtime_separation"]
future_rules_ru = {
    "runtime logic must remain isolated from design-only artifacts": "runtime-логика должна оставаться изолированной от design-only артефактов",
    "any future runtime requires separate gate and separate phase": "любой будущий runtime требует отдельного gate и отдельной фазы",
    "approval does not imply execution": "approval не означает разрешение на execution",
    "evidence chain must remain intact before any future runtime activation": "перед любой будущей runtime-активацией evidence-цепочка должна оставаться целостной",
}
for rule in future_runtime_separation["rules"]:
    md_lines.append(f"- {future_rules_ru.get(rule, rule)}")
md_lines.append(f"- operator_message_ru: {future_runtime_separation['operator_message_ru']}")

md_lines += ["", "## recommended_first_design_slice"]
for k, v in recommended_first_design_slice.items():
    md_lines.append(f"- {k}: {v}")

md_lines += ["", "## non_execution_confirmation"]
for k, v in payload["non_execution_confirmation"].items():
    md_lines.append(f"- {k}: {v}")

md_lines += ["", "## validated_reference_chain"]
for k, v in validated_refs.items():
    md_lines.append(f"- {k}: {v}")

out_md_path.write_text("\n".join(md_lines) + "\n")

print("Готово: сформирован design-only blueprint для Phase 35.1 без исполнения.")
print(f"Маркер: {marker}")
PY
