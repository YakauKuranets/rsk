#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export BLUEPRINT_JSON="${ROOT_DIR}/docs/phase35_validation_agent_design_blueprint_v1.json"
export LAYER_CONTRACTS_JSON="${ROOT_DIR}/docs/phase35_validation_agent_layer_contracts_v1.json"
export ENTRY_PACK_JSON="${ROOT_DIR}/docs/phase35_entry_pack_v1.json"
export REVIEW_CYCLE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_review_cycle_bundle_v1.json"
export RUNTIME_ENTRY_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_entry_contract_v1.json"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export DRY_RUN_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase35_contract_slice_artifact_to_policy_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase35_contract_slice_artifact_to_policy_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]

required_paths = {
    "phase35_validation_agent_design_blueprint": Path(os.environ["BLUEPRINT_JSON"]),
    "phase35_validation_agent_layer_contracts": Path(os.environ["LAYER_CONTRACTS_JSON"]),
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

missing_required_inputs = [name for name, path in required_paths.items() if not path.exists()]
parse_errors = []
triage_present = triage_path.exists()


def load_json(path: Path):
    return json.loads(path.read_text())


docs = {}
for name, path in required_paths.items():
    if not path.exists():
        docs[name] = {}
        continue
    try:
        docs[name] = load_json(path)
    except json.JSONDecodeError as exc:
        docs[name] = {}
        parse_errors.append(f"{name}:invalid_json:{exc.msg}")
        missing_required_inputs.append(name)

triage_doc = {}
if triage_present:
    try:
        triage_doc = load_json(triage_path)
    except json.JSONDecodeError as exc:
        parse_errors.append(f"phase34_operator_backlog_triage:invalid_json:{exc.msg}")

required_markers = {
    "baseline_marker": docs["phase33_baseline_freeze"].get("marker", ""),
    "policy_marker": docs["phase33_operator_policy"].get("marker", ""),
    "handoff_marker": docs["phase33_handoff_pack"].get("marker", ""),
    "phase35_blueprint_marker": docs["phase35_validation_agent_design_blueprint"].get("marker", ""),
    "layer_contracts_marker": docs["phase35_validation_agent_layer_contracts"].get("marker", ""),
    "evidence_marker_set": docs["phase34_validation_agent_review_cycle_bundle"].get("marker", ""),
}
if triage_present:
    required_markers["triage_marker_optional"] = triage_doc.get("marker", "")

missing_required_markers = [k for k, v in required_markers.items() if (k != "triage_marker_optional" and not v)]

state_values = [
    docs["phase35_validation_agent_design_blueprint"].get("status", ""),
    docs["phase35_validation_agent_layer_contracts"].get("status", ""),
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
    state_values.append(triage_doc.get("status", ""))

blocked_detected = any(v == "blocked" or v.endswith("_blocked") for v in state_values if v)
notes_detected = any("with_notes" in v for v in state_values if v)

if missing_required_inputs or parse_errors or missing_required_markers:
    status = "contract_slice_blocked"
    reason = "safe_contract_slice_reference_missing"
elif blocked_detected or notes_detected:
    status = "contract_slice_ready_with_notes"
    reason = "safe_contract_slice_reference_ready_with_notes"
else:
    status = "contract_slice_ready"
    reason = "safe_contract_slice_reference_ready"

source_layer_contract = {
    "layer_id": "artifact_intake_layer",
    "role_ru": "Нормализация входного artifact context для передачи в policy reasoning.",
    "accepted_artifacts": [
        "phase35_validation_agent_design_blueprint_v1.json",
        "phase35_validation_agent_layer_contracts_v1.json",
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
    "required_fields": [
        "artifact_id",
        "artifact_type",
        "artifact_path",
        "artifact_status",
        "artifact_marker",
        "policy_ref",
        "baseline_ref",
        "handoff_ref",
        "evidence_refs",
        "ingest_timestamp",
    ],
    "allowed_technical_refs": [
        "marker",
        "source_artifact",
        "version",
        "generated_at",
        "status",
        "reason",
    ],
    "forbidden_actions": [
        "runtime_execution",
        "graph_mutation",
        "remediation_actions",
        "hidden_side_effects",
        "execution_flag_injection",
    ],
    "normalization_only": True,
}

target_layer_contract = {
    "layer_id": "policy_reasoning_layer",
    "role_ru": "Интерпретация нормализованного контекста с policy/baseline ограничениями и recommendation-only выводом.",
    "accepted_input_fields": [
        "artifact_id",
        "artifact_type",
        "artifact_status",
        "artifact_marker",
        "policy_ref",
        "baseline_ref",
        "handoff_ref",
        "triage_ref",
        "evidence_refs",
        "ingest_timestamp",
    ],
    "required_policy_baseline_refs": ["policy_ref", "baseline_ref"],
    "allowed_reasoning_outputs": [
        "policy_consistency_assessment",
        "baseline_alignment_assessment",
        "recommendation_notes",
        "control_boundary_flags",
        "reasoning_evidence_refs",
    ],
    "forbidden_actions": [
        "runtime_execution",
        "graph_mutation",
        "remediation_actions",
        "silent_execution_fallback",
    ],
    "recommendation_only": True,
}

input_field_schema = [
    {"field": "artifact_id", "required": True, "type": "string", "description_ru": "Уникальный идентификатор артефакта."},
    {"field": "artifact_type", "required": True, "type": "string", "description_ru": "Тип артефакта из поддерживаемого набора."},
    {"field": "artifact_path", "required": True, "type": "string", "description_ru": "Путь к входному артефакту в docs/."},
    {"field": "artifact_status", "required": True, "type": "string", "description_ru": "Текущий статус артефакта в цепочке."},
    {"field": "artifact_marker", "required": True, "type": "string", "description_ru": "Маркер артефакта для трассируемости."},
    {"field": "policy_ref", "required": True, "type": "string", "description_ru": "Ссылка на policy marker/ref."},
    {"field": "baseline_ref", "required": True, "type": "string", "description_ru": "Ссылка на baseline marker/ref."},
    {"field": "handoff_ref", "required": True, "type": "string", "description_ru": "Ссылка на handoff marker/ref."},
    {"field": "triage_ref", "required": False, "type": "string", "description_ru": "Опциональная ссылка на triage marker/ref."},
    {"field": "evidence_refs", "required": True, "type": "array<string>", "description_ru": "Набор ссылок на evidence chain."},
    {"field": "ingest_timestamp", "required": True, "type": "string(datetime)", "description_ru": "UTC-время ingest/нормализации."},
]

interface_invariants = [
    "artifact context only",
    "no runtime execution",
    "no graph mutation",
    "no remediation",
    "no hidden side effects",
    "no policy bypass",
    "no baseline bypass",
    "no implicit transition from intake to action",
    "no silent execution fallback",
]

validation_rules = [
    "required_fields_present",
    "required_markers_present",
    "baseline_and_policy_refs_are_current",
    "artifact_type_recognized",
    "payload_contains_no_execution_flags",
    "intake_output_is_compatible_with_policy_reasoning_input",
]

rejection_rules = [
    "missing_required_fields",
    "missing_required_markers",
    "stale_policy_or_baseline_refs",
    "execution_related_flags_present",
    "malformed_evidence_refs",
    "unsupported_artifact_type",
    "hidden_action_fields_detected",
]

recommended_next_contract_slice = {
    "slice_id": "policy_reasoning_to_dry_run_recommendation_v1",
    "goal_ru": "Зафиксировать recommendation-only контракт передачи reasoning outputs в dry-run слой.",
    "depends_on_current_slice": True,
    "runtime_authorization_change": False,
}

contract_slice_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": sorted(set(missing_required_inputs)),
    "missing_required_markers": missing_required_markers,
    "parse_errors": parse_errors,
    "triage_artifact_present": triage_present,
    "operator_message_ru": "Детализирован только интерфейсный read-only срез intake→policy без права исполнения.",
}

marker = f"KV_PHASE35_CONTRACT_SLICE_ARTIFACT_TO_POLICY_V1|status={status}|reason={reason}"

payload = {
    "version": "phase35_contract_slice_artifact_to_policy_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "contract_slice_status": contract_slice_status,
    "source_layer_contract": source_layer_contract,
    "target_layer_contract": target_layer_contract,
    "input_field_schema": input_field_schema,
    "required_markers": required_markers,
    "interface_invariants": interface_invariants,
    "validation_rules": validation_rules,
    "rejection_rules": rejection_rules,
    "non_execution_confirmation": {
        "execution_authorized": False,
        "graph_write_authorized": False,
        "remediation_authorized": False,
        "runtime_phase_open": False,
        "contract_slice_is_not_runtime_permission": True,
    },
    "recommended_next_contract_slice": recommended_next_contract_slice,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

md_lines = [
    "# Фаза 35.3 — Contract Slice v1: artifact intake → policy reasoning",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- Статус: **{status}**",
    f"- Причина: **{reason}**",
    "- Этот документ описывает только read-only интерфейсный контракт.",
    "- Даже детальный contract slice не является разрешением на runtime.",
    "",
    "## contract_slice_status",
]
for k, v in contract_slice_status.items():
    md_lines.append(f"- {k}: {v}")

md_lines += ["", "## source_layer_contract"]
for k, v in source_layer_contract.items():
    md_lines.append(f"- {k}: {v}")

md_lines += ["", "## target_layer_contract"]
for k, v in target_layer_contract.items():
    md_lines.append(f"- {k}: {v}")

md_lines += ["", "## input_field_schema"]
for field in input_field_schema:
    md_lines.append(
        f"- {field['field']} | required={field['required']} | type={field['type']} | {field['description_ru']}"
    )

md_lines += ["", "## required_markers"]
for k, v in required_markers.items():
    md_lines.append(f"- {k}: {v}")

md_lines += ["", "## interface_invariants"]
for item in interface_invariants:
    md_lines.append(f"- {item}")

md_lines += ["", "## validation_rules"]
for item in validation_rules:
    md_lines.append(f"- {item}")

md_lines += ["", "## rejection_rules"]
for item in rejection_rules:
    md_lines.append(f"- {item}")

md_lines += ["", "## non_execution_confirmation"]
for k, v in payload["non_execution_confirmation"].items():
    md_lines.append(f"- {k}: {v}")

md_lines += ["", "## recommended_next_contract_slice"]
for k, v in recommended_next_contract_slice.items():
    md_lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(md_lines) + "\n")

print("Готово: сформирован contract slice v1 intake→policy в read-only режиме.")
print("Исполнение, запись в граф и remediation остаются запрещены.")
print(f"Маркер: {marker}")
PY
