#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export BLUEPRINT_JSON="${ROOT_DIR}/docs/phase35_validation_agent_design_blueprint_v1.json"
export ENTRY_PACK_JSON="${ROOT_DIR}/docs/phase35_entry_pack_v1.json"
export REVIEW_CYCLE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_review_cycle_bundle_v1.json"
export RUNTIME_ENTRY_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_entry_contract_v1.json"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export DRY_RUN_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase35_validation_agent_layer_contracts_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase35_validation_agent_layer_contracts_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]

paths = {
    "phase35_validation_agent_design_blueprint": Path(os.environ["BLUEPRINT_JSON"]),
    "phase35_entry_pack": Path(os.environ["ENTRY_PACK_JSON"]),
    "phase34_validation_agent_review_cycle_bundle": Path(os.environ["REVIEW_CYCLE_JSON"]),
    "phase34_validation_agent_runtime_entry_contract": Path(os.environ["RUNTIME_ENTRY_JSON"]),
    "phase34_validation_agent_approval_contract": Path(os.environ["APPROVAL_CONTRACT_JSON"]),
    "phase34_validation_agent_dry_run": Path(os.environ["DRY_RUN_JSON"]),
    "phase33_operator_policy": Path(os.environ["POLICY_JSON"]),
    "phase33_baseline_freeze": Path(os.environ["BASELINE_JSON"]),
    "phase33_handoff_pack": Path(os.environ["HANDOFF_JSON"]),
}
triage_path = Path(os.environ["TRIAGE_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

missing_required_inputs = [name for name, p in paths.items() if not p.exists()]
triage_present = triage_path.exists()


def load_json(path: Path):
    return json.loads(path.read_text())


docs = {}
parse_errors = []
for name, path in paths.items():
    if not path.exists():
        docs[name] = {}
        continue
    try:
        docs[name] = load_json(path)
    except json.JSONDecodeError as exc:
        docs[name] = {}
        parse_errors.append(f"{name}:invalid_json:{exc.msg}")
        missing_required_inputs.append(name)

triage = {}
if triage_present:
    try:
        triage = load_json(triage_path)
    except json.JSONDecodeError as exc:
        parse_errors.append(f"phase34_operator_backlog_triage:invalid_json:{exc.msg}")

reference_markers = {
    "blueprint_marker": docs["phase35_validation_agent_design_blueprint"].get("marker", ""),
    "entry_pack_marker": docs["phase35_entry_pack"].get("marker", ""),
    "review_cycle_marker": docs["phase34_validation_agent_review_cycle_bundle"].get("marker", ""),
    "runtime_entry_marker": docs["phase34_validation_agent_runtime_entry_contract"].get("marker", ""),
    "approval_contract_marker": docs["phase34_validation_agent_approval_contract"].get("marker", ""),
    "dry_run_marker": docs["phase34_validation_agent_dry_run"].get("marker", ""),
    "operator_policy_marker": docs["phase33_operator_policy"].get("marker", ""),
    "baseline_marker": docs["phase33_baseline_freeze"].get("marker", ""),
    "handoff_marker": docs["phase33_handoff_pack"].get("marker", ""),
}
if triage_present:
    reference_markers["triage_marker"] = triage.get("marker", "")

missing_reference_markers = [k for k, v in reference_markers.items() if not v]

state_values = [
    docs["phase35_validation_agent_design_blueprint"].get("status", ""),
    docs["phase35_entry_pack"].get("status", ""),
    docs["phase34_validation_agent_review_cycle_bundle"].get("status", ""),
    docs["phase34_validation_agent_runtime_entry_contract"].get("status", ""),
    docs["phase34_validation_agent_approval_contract"].get("status", ""),
    docs["phase34_validation_agent_dry_run"].get("status", ""),
    docs["phase33_operator_policy"].get("status", ""),
    docs["phase33_baseline_freeze"].get("baseline_status", ""),
    docs["phase33_handoff_pack"].get("status", ""),
]
if triage_present:
    state_values.append(triage.get("status", ""))

blocked_detected = any((value == "blocked" or value.endswith("_blocked")) for value in state_values if value)
notes_detected = any("with_notes" in value for value in state_values if value)

if missing_required_inputs or parse_errors or missing_reference_markers:
    status = "layer_contracts_blocked"
    reason = "safe_layer_contract_reference_missing"
elif blocked_detected or notes_detected:
    status = "layer_contracts_ready_with_notes"
    reason = "safe_layer_contract_reference_ready_with_notes"
else:
    status = "layer_contracts_ready"
    reason = "safe_layer_contract_reference_ready"

layer_inventory = [
    {
        "layer_id": "artifact_intake_layer",
        "purpose_ru": "Приём и структурная валидация входных артефактов и маркеров без исполнения.",
        "depends_on": ["policy", "baseline", "approval_chain"],
        "allowed_inputs": [
            "phase35_validation_agent_design_blueprint_v1.json",
            "phase35_entry_pack_v1.json",
            "phase34_validation_agent_review_cycle_bundle_v1.json",
            "phase34_validation_agent_runtime_entry_contract_v1.json",
            "phase34_validation_agent_approval_contract_v1.json",
            "phase34_validation_agent_dry_run_v1.json",
            "phase33_operator_policy_v1.json",
            "phase33_baseline_freeze_v1.json",
            "phase33_handoff_pack_v1.json",
            "phase34_operator_backlog_triage_v1.json (optional)",
        ],
        "allowed_outputs": ["normalized_artifact_manifest", "marker_presence_report", "input_quality_notes"],
        "forbidden_actions": ["runtime_execution", "graph_mutation", "remediation_actions", "policy_override"],
    },
    {
        "layer_id": "policy_reasoning_layer",
        "purpose_ru": "Связывание policy/baseline/approval условий в единые логические контракты интерфейсов.",
        "depends_on": ["policy", "baseline", "approval_chain"],
        "allowed_inputs": ["normalized_artifact_manifest", "marker_presence_report", "policy_and_baseline_states"],
        "allowed_outputs": ["policy_consistency_contract", "control_boundary_flags", "contract_risk_notes"],
        "forbidden_actions": ["policy_bypass", "baseline_bypass", "runtime_trigger", "hidden_side_effects"],
    },
    {
        "layer_id": "dry_run_recommendation_layer",
        "purpose_ru": "Формирование безопасных dry-run рекомендаций по цепочке артефактов без перехода в runtime.",
        "depends_on": ["policy", "baseline", "approval_chain"],
        "allowed_inputs": ["policy_consistency_contract", "phase34_validation_agent_dry_run_v1", "review_cycle_bundle"],
        "allowed_outputs": ["dry_run_contract_recommendations", "operator_note_bundle", "non_execution_constraints"],
        "forbidden_actions": ["runtime_execution", "graph_mutation", "remediation_actions", "implicit_runtime_transition"],
    },
    {
        "layer_id": "approval_interface_layer",
        "purpose_ru": "Фиксация интерфейсов approval chain и требований к evidence без права запуска исполнения.",
        "depends_on": ["policy", "baseline", "approval_chain"],
        "allowed_inputs": ["approval_contract", "runtime_entry_contract", "dry_run_contract_recommendations"],
        "allowed_outputs": ["approval_interface_contract_map", "approval_gate_requirements", "approval_boundary_assertions"],
        "forbidden_actions": ["approval_bypass", "runtime_opening", "silent_execution_fallback", "graph_mutation"],
    },
    {
        "layer_id": "audit_evidence_layer",
        "purpose_ru": "Сбор и трассировка read-only evidence-цепочки по всем интерфейсам.",
        "depends_on": ["policy", "baseline", "approval_chain"],
        "allowed_inputs": ["approval_interface_contract_map", "operator_note_bundle", "marker_presence_report"],
        "allowed_outputs": ["audit_evidence_index", "interface_traceability_matrix", "control_boundary_audit_notes"],
        "forbidden_actions": ["evidence_tampering", "hidden_side_effects", "graph_mutation", "runtime_execution"],
    },
    {
        "layer_id": "future_runtime_boundary_layer",
        "purpose_ru": "Отдельная неактивная граница для будущей runtime-фазы; в текущей фазе только декларация ограничений.",
        "depends_on": ["policy", "baseline", "approval_chain"],
        "allowed_inputs": ["audit_evidence_index", "approval_gate_requirements", "control_boundary_flags"],
        "allowed_outputs": ["runtime_boundary_constraints_only"],
        "forbidden_actions": ["runtime_activation", "runtime_execution", "graph_mutation", "remediation_actions"],
    },
]

layer_input_contracts = [
    {
        "layer_id": layer["layer_id"],
        "contract_type": "read_only_inputs_only",
        "accepted_inputs": layer["allowed_inputs"],
        "input_validation_rules": [
            "marker_must_be_present_when_required",
            "artifact_format_must_be_json_contract_or_documented_equivalent",
            "missing_or_invalid_inputs_keep_runtime_closed",
        ],
    }
    for layer in layer_inventory
]

layer_output_contracts = [
    {
        "layer_id": layer["layer_id"],
        "contract_type": "declarative_outputs_only",
        "allowed_outputs": layer["allowed_outputs"],
        "output_guarantees": [
            "no_runtime_side_effects",
            "no_graph_writes",
            "control_boundaries_preserved",
        ],
    }
    for layer in layer_inventory
]

interface_map = {
    "interfaces": [
        {
            "from": "artifact_intake_layer",
            "to": "policy_reasoning_layer",
            "transfer_mode": "data_artifacts_only",
            "execution_permitted": False,
            "notes_ru": "Передача только нормализованных артефактов и статусов маркеров.",
        },
        {
            "from": "policy_reasoning_layer",
            "to": "dry_run_recommendation_layer",
            "transfer_mode": "data_artifacts_only",
            "execution_permitted": False,
            "notes_ru": "Передаются только логические ограничения и policy/baseline выводы.",
        },
        {
            "from": "dry_run_recommendation_layer",
            "to": "approval_interface_layer",
            "transfer_mode": "data_artifacts_only",
            "execution_permitted": False,
            "notes_ru": "Передаются dry-run рекомендации без запуска remediation или runtime.",
        },
        {
            "from": "approval_interface_layer",
            "to": "audit_evidence_layer",
            "transfer_mode": "data_artifacts_only",
            "execution_permitted": False,
            "notes_ru": "Передаются только approval-контракты и требования evidence-цепочки.",
        },
        {
            "from": "audit_evidence_layer",
            "to": "future_runtime_boundary_layer",
            "transfer_mode": "data_artifacts_only",
            "execution_permitted": False,
            "notes_ru": "Передаются только декларативные ограничения; runtime-граница остаётся закрытой.",
        },
    ],
    "non_executable_layers": [
        "artifact_intake_layer",
        "policy_reasoning_layer",
        "dry_run_recommendation_layer",
        "approval_interface_layer",
        "audit_evidence_layer",
        "future_runtime_boundary_layer",
    ],
    "future_runtime_boundary": {
        "layer_id": "future_runtime_boundary_layer",
        "active": False,
        "open": False,
        "notes_ru": "Слой существует только как контрактная граница; runtime не активирован и не открыт.",
    },
}

control_boundaries_enforcement = {
    "policy_boundary": "Все интерфейсы обязаны наследовать policy-ограничения из phase33_operator_policy_v1.",
    "baseline_boundary": "Все интерфейсы обязаны уважать baseline-статус и не обходить baseline freeze gate.",
    "approval_boundary": "Без полного approval-chain интерфейсы остаются в read-only и не открывают исполнение.",
    "audit_boundary": "Каждый интерфейс обязан оставлять трассируемый evidence-след без скрытых эффектов.",
    "runtime_boundary": "Runtime boundary слой закрыт; любая попытка implicit runtime transition запрещена.",
}

non_execution_interface_rules = [
    "no runtime execution",
    "no graph mutation",
    "no remediation",
    "no hidden side effects",
    "no policy bypass",
    "no baseline bypass",
    "no implicit transition across interfaces",
    "no silent execution fallback across interfaces",
]

recommended_first_contract_slice = {
    "slice_id": "artifact_intake_to_policy_reasoning_contract_v1",
    "goal_ru": "Сначала формализовать контракт между artifact_intake_layer и policy_reasoning_layer.",
    "includes_layers": ["artifact_intake_layer", "policy_reasoning_layer"],
    "expected_output_ru": "Явная схема входов/выходов, проверок маркеров и запретов на переход к исполнению.",
    "execution_guardrails": {
        "execution_authorized": False,
        "graph_write_authorized": False,
        "remediation_authorized": False,
        "runtime_phase_open": False,
    },
}

layer_contracts_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": sorted(set(missing_required_inputs)),
    "missing_reference_markers": missing_reference_markers,
    "parse_errors": parse_errors,
    "triage_artifact_present": triage_present,
    "operator_message_ru": "Контракты слоёв сформированы как read-only scaffold без права на runtime-исполнение.",
}

marker = f"KV_PHASE35_VALIDATION_AGENT_LAYER_CONTRACTS_V1|status={status}|reason={reason}"

payload = {
    "version": "phase35_validation_agent_layer_contracts_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "layer_contracts_status": layer_contracts_status,
    "layer_inventory": layer_inventory,
    "layer_input_contracts": layer_input_contracts,
    "layer_output_contracts": layer_output_contracts,
    "interface_map": interface_map,
    "control_boundaries_enforcement": control_boundaries_enforcement,
    "non_execution_interface_rules": non_execution_interface_rules,
    "recommended_first_contract_slice": recommended_first_contract_slice,
    "non_execution_confirmation": {
        "execution_authorized": False,
        "graph_write_authorized": False,
        "remediation_authorized": False,
        "runtime_phase_open": False,
        "runtime_map_is_not_runtime_permission": True,
    },
    "validated_reference_chain": reference_markers,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

layer_labels = {
    "artifact_intake_layer": "artifact intake layer (слой приёма артефактов)",
    "policy_reasoning_layer": "policy reasoning layer (слой policy-анализа)",
    "dry_run_recommendation_layer": "dry-run recommendation layer (слой dry-run рекомендаций)",
    "approval_interface_layer": "approval interface layer (слой approval-интерфейсов)",
    "audit_evidence_layer": "audit evidence layer (слой аудита и evidence)",
    "future_runtime_boundary_layer": "future runtime boundary layer (граница будущего runtime)",
}

md = [
    "# Фаза 35.2 — Layer Contracts / Interface Map v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- Статус: **{status}**",
    f"- Причина: **{reason}**",
    "- Этот артефакт является только read-only контрактной картой.",
    "- Даже полная карта интерфейсов не является разрешением на runtime.",
    "",
    "## layer_contracts_status",
]
for k, v in layer_contracts_status.items():
    md.append(f"- {k}: {v}")

md += ["", "## layer_inventory"]
for layer in layer_inventory:
    md.append(f"- {layer_labels.get(layer['layer_id'], layer['layer_id'])}")
    md.append(f"  - Назначение: {layer['purpose_ru']}")
    md.append(f"  - Зависимости: {', '.join(layer['depends_on'])}")
    md.append(f"  - Допустимые входы: {', '.join(layer['allowed_inputs'])}")
    md.append(f"  - Допустимые выходы: {', '.join(layer['allowed_outputs'])}")
    md.append(f"  - Запрещённые действия: {', '.join(layer['forbidden_actions'])}")

md += ["", "## layer_input_contracts"]
for item in layer_input_contracts:
    md.append(f"- layer_id: {item['layer_id']}")
    md.append(f"  - contract_type: {item['contract_type']}")
    md.append(f"  - accepted_inputs: {item['accepted_inputs']}")
    md.append(f"  - input_validation_rules: {item['input_validation_rules']}")

md += ["", "## layer_output_contracts"]
for item in layer_output_contracts:
    md.append(f"- layer_id: {item['layer_id']}")
    md.append(f"  - contract_type: {item['contract_type']}")
    md.append(f"  - allowed_outputs: {item['allowed_outputs']}")
    md.append(f"  - output_guarantees: {item['output_guarantees']}")

md += ["", "## interface_map", "### Интерфейсные переходы"]
for edge in interface_map["interfaces"]:
    md.append(
        f"- {edge['from']} -> {edge['to']} | режим={edge['transfer_mode']} | execution_permitted={edge['execution_permitted']}"
    )
    md.append(f"  - Примечание: {edge['notes_ru']}")

md += ["", "### Слои без права исполнения"]
for layer in interface_map["non_executable_layers"]:
    md.append(f"- {layer}")

md += ["", "### Граница будущего runtime"]
for k, v in interface_map["future_runtime_boundary"].items():
    md.append(f"- {k}: {v}")

md += ["", "## control_boundaries_enforcement"]
for k, v in control_boundaries_enforcement.items():
    md.append(f"- {k}: {v}")

md += ["", "## non_execution_interface_rules"]
for rule in non_execution_interface_rules:
    md.append(f"- {rule}")

md += ["", "## recommended_first_contract_slice"]
for k, v in recommended_first_contract_slice.items():
    md.append(f"- {k}: {v}")

md += ["", "## non_execution_confirmation"]
for k, v in payload["non_execution_confirmation"].items():
    md.append(f"- {k}: {v}")

md += ["", "## validated_reference_chain"]
for k, v in reference_markers.items():
    md.append(f"- {k}: {v}")

out_md.write_text("\n".join(md) + "\n")

print("Готово: сформирован read-only контрактный слой Phase 35.2.")
print("Исполнение, запись в граф и remediation остаются запрещены.")
print(f"Маркер: {marker}")
PY
