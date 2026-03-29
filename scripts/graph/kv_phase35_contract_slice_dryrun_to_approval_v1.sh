#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export BLUEPRINT_JSON="${ROOT_DIR}/docs/phase35_validation_agent_design_blueprint_v1.json"
export LAYER_CONTRACTS_JSON="${ROOT_DIR}/docs/phase35_validation_agent_layer_contracts_v1.json"
export SLICE_ARTIFACT_POLICY_JSON="${ROOT_DIR}/docs/phase35_contract_slice_artifact_to_policy_v1.json"
export SLICE_POLICY_DRYRUN_JSON="${ROOT_DIR}/docs/phase35_contract_slice_policy_to_dryrun_v1.json"
export ENTRY_PACK_JSON="${ROOT_DIR}/docs/phase35_entry_pack_v1.json"
export DRY_RUN_JSON="${ROOT_DIR}/docs/phase34_validation_agent_dry_run_v1.json"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export APPROVAL_RECORD_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.json"
export OPERATOR_GATE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_operator_gate_v1.json"
export POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase35_contract_slice_dryrun_to_approval_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase35_contract_slice_dryrun_to_approval_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
required_paths = {
    "phase35_validation_agent_design_blueprint": Path(os.environ["BLUEPRINT_JSON"]),
    "phase35_validation_agent_layer_contracts": Path(os.environ["LAYER_CONTRACTS_JSON"]),
    "phase35_contract_slice_artifact_to_policy": Path(os.environ["SLICE_ARTIFACT_POLICY_JSON"]),
    "phase35_contract_slice_policy_to_dryrun": Path(os.environ["SLICE_POLICY_DRYRUN_JSON"]),
    "phase35_entry_pack": Path(os.environ["ENTRY_PACK_JSON"]),
    "phase34_validation_agent_dry_run": Path(os.environ["DRY_RUN_JSON"]),
    "phase34_validation_agent_approval_contract": Path(os.environ["APPROVAL_CONTRACT_JSON"]),
    "phase34_validation_agent_approval_record": Path(os.environ["APPROVAL_RECORD_JSON"]),
    "phase34_validation_agent_operator_gate": Path(os.environ["OPERATOR_GATE_JSON"]),
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
    "dry_run_marker": docs["phase34_validation_agent_dry_run"].get("marker", ""),
    "policy_marker": docs["phase33_operator_policy"].get("marker", ""),
    "baseline_marker": docs["phase33_baseline_freeze"].get("marker", ""),
    "phase35_blueprint_marker": docs["phase35_validation_agent_design_blueprint"].get("marker", ""),
    "layer_contracts_marker": docs["phase35_validation_agent_layer_contracts"].get("marker", ""),
    "contract_slice_policy_to_dryrun_marker": docs["phase35_contract_slice_policy_to_dryrun"].get("marker", ""),
    "approval_contract_marker": docs["phase34_validation_agent_approval_contract"].get("marker", ""),
    "handoff_marker": docs["phase33_handoff_pack"].get("marker", ""),
}
if triage_present:
    required_markers["triage_marker_optional"] = triage_doc.get("marker", "")

missing_required_markers = [k for k, v in required_markers.items() if (k != "triage_marker_optional" and not v)]

state_values = [
    docs["phase35_validation_agent_design_blueprint"].get("status", ""),
    docs["phase35_validation_agent_layer_contracts"].get("status", ""),
    docs["phase35_contract_slice_artifact_to_policy"].get("status", ""),
    docs["phase35_contract_slice_policy_to_dryrun"].get("status", ""),
    docs["phase35_entry_pack"].get("status", ""),
    docs["phase34_validation_agent_dry_run"].get("status", ""),
    docs["phase34_validation_agent_approval_contract"].get("status", ""),
    docs["phase34_validation_agent_approval_record"].get("status", ""),
    docs["phase34_validation_agent_operator_gate"].get("status", ""),
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
    "layer_id": "dry_run_recommendation_layer",
    "role_ru": "Формирует recommendation packet для интерфейса approval без сигнала исполнения.",
    "recommendation_outputs": [
        "dry_run_recommendation_packet",
        "recommendation_summary",
        "recommendation_details",
        "constraint_flags",
        "operator_review_notes",
    ],
    "required_packet_fields": [
        "recommendation_id",
        "recommendation_status",
        "reasoning_ref",
        "policy_ref",
        "baseline_ref",
        "recommendation_marker",
        "recommendation_summary",
        "recommendation_details",
        "constraint_flags",
        "evidence_refs",
        "generated_at",
    ],
    "allowed_summary_forms": ["summary_table", "priority_bucket_summary", "operator_notes_summary"],
    "not_approval_signal": True,
    "forbidden_actions": ["runtime_execution", "graph_mutation", "remediation_actions", "approval_auto_emit"],
}

target_layer_contract = {
    "layer_id": "approval_interface_layer",
    "role_ru": "Принимает recommendation packet и формирует approval-facing/operator-facing представление без запуска исполнения.",
    "accepted_packet_fields": source_layer_contract["required_packet_fields"],
    "allowed_approval_facing_fields": [
        "approval_review_context",
        "approval_gate_requirements",
        "recommendation_traceability_map",
    ],
    "allowed_operator_facing_fields": [
        "operator_review_summary",
        "operator_action_checklist",
        "risk_visibility_notes",
    ],
    "forbidden_actions": ["runtime_execution", "graph_mutation", "remediation_actions", "implicit_approval_open"],
    "no_implicit_approval_no_runtime_open": True,
}

recommendation_packet_schema = [
    {"field": "recommendation_id", "required": True, "type": "string", "description_ru": "Уникальный идентификатор recommendation packet."},
    {"field": "recommendation_status", "required": True, "type": "string", "description_ru": "Статус recommendation packet."},
    {"field": "reasoning_ref", "required": True, "type": "string", "description_ru": "Ссылка на reasoning output предыдущего слоя."},
    {"field": "policy_ref", "required": True, "type": "string", "description_ru": "Ссылка на policy marker/ref."},
    {"field": "baseline_ref", "required": True, "type": "string", "description_ru": "Ссылка на baseline marker/ref."},
    {"field": "recommendation_marker", "required": True, "type": "string", "description_ru": "Маркер recommendation packet."},
    {"field": "recommendation_summary", "required": True, "type": "object", "description_ru": "Краткая summary-структура рекомендации."},
    {"field": "recommendation_details", "required": True, "type": "array<object>", "description_ru": "Детализированные recommendation записи."},
    {"field": "constraint_flags", "required": True, "type": "array<string>", "description_ru": "Constraint-флаги policy/baseline цепочки."},
    {"field": "evidence_refs", "required": True, "type": "array<string>", "description_ru": "Ссылки на evidence chain."},
    {"field": "generated_at", "required": True, "type": "string(datetime)", "description_ru": "UTC-время генерации recommendation packet."},
]

interface_invariants = [
    "recommendation-only flow",
    "no approval signal",
    "no runtime execution",
    "no graph mutation",
    "no remediation",
    "no hidden side effects",
    "no policy bypass",
    "no baseline bypass",
    "no implicit transition from recommendation to approval",
    "no silent execution fallback",
]

validation_rules = [
    "recommendation_packet_has_required_fields",
    "required_markers_present",
    "recommendation_summary_and_details_are_well_formed",
    "evidence_refs_are_well_formed",
    "output_is_compatible_with_approval_interface_input",
    "execution_related_flags_absent",
    "constraint_flags_align_with_policy_baseline_chain",
]

rejection_rules = [
    "missing_required_fields",
    "missing_required_markers",
    "stale_policy_or_baseline_refs",
    "malformed_recommendation_summary",
    "malformed_recommendation_details",
    "malformed_evidence_refs",
    "execution_related_flags_present",
    "hidden_action_fields_detected",
    "implicit_approval_fields_detected",
]

recommended_next_contract_slice = {
    "slice_id": "approval_interface_to_audit_evidence_v1",
    "goal_ru": "Зафиксировать следующий read-only интерфейс между approval interface и audit evidence layer.",
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
    "operator_message_ru": "Сформирован только read-only контракт dry-run→approval без права на runtime или approval execution.",
}

marker = f"KV_PHASE35_CONTRACT_SLICE_DRYRUN_TO_APPROVAL_V1|status={status}|reason={reason}"

payload = {
    "version": "phase35_contract_slice_dryrun_to_approval_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "contract_slice_status": contract_slice_status,
    "source_layer_contract": source_layer_contract,
    "target_layer_contract": target_layer_contract,
    "recommendation_packet_schema": recommendation_packet_schema,
    "required_markers": required_markers,
    "interface_invariants": interface_invariants,
    "validation_rules": validation_rules,
    "rejection_rules": rejection_rules,
    "non_execution_confirmation": {
        "execution_authorized": False,
        "graph_write_authorized": False,
        "remediation_authorized": False,
        "runtime_phase_open": False,
        "contract_slice_is_not_runtime_or_approval_execution_permission": True,
    },
    "recommended_next_contract_slice": recommended_next_contract_slice,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")

md_lines = [
    "# Фаза 35.5 — Contract Slice v3: dry_run_recommendation_layer -> approval_interface_layer",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- Статус: **{status}**",
    f"- Причина: **{reason}**",
    "- Этот документ описывает только read-only интерфейсный срез dry-run→approval.",
    "- Даже детальный contract slice не является разрешением на runtime или approval execution.",
    "",
    "## contract_slice_status",
]
for k, v in contract_slice_status.items():
    md_lines.append(f"- {k}: {v}")

for section_name, section in [
    ("source_layer_contract", source_layer_contract),
    ("target_layer_contract", target_layer_contract),
]:
    md_lines += ["", f"## {section_name}"]
    for k, v in section.items():
        md_lines.append(f"- {k}: {v}")

md_lines += ["", "## recommendation_packet_schema"]
for field in recommendation_packet_schema:
    md_lines.append(f"- {field['field']} | required={field['required']} | type={field['type']} | {field['description_ru']}")

for section_name, section in [
    ("required_markers", required_markers),
]:
    md_lines += ["", f"## {section_name}"]
    for k, v in section.items():
        md_lines.append(f"- {k}: {v}")

for section_name, values in [
    ("interface_invariants", interface_invariants),
    ("validation_rules", validation_rules),
    ("rejection_rules", rejection_rules),
]:
    md_lines += ["", f"## {section_name}"]
    for value in values:
        md_lines.append(f"- {value}")

md_lines += ["", "## non_execution_confirmation"]
for k, v in payload["non_execution_confirmation"].items():
    md_lines.append(f"- {k}: {v}")

md_lines += ["", "## recommended_next_contract_slice"]
for k, v in recommended_next_contract_slice.items():
    md_lines.append(f"- {k}: {v}")

out_md.write_text("\n".join(md_lines) + "\n")

print("Готово: сформирован contract slice v3 dry-run→approval в read-only режиме.")
print("Исполнение, запись в граф и remediation остаются запрещены.")
print(f"Маркер: {marker}")
PY
