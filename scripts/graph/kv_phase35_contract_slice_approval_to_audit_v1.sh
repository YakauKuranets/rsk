#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export BLUEPRINT_JSON="${ROOT_DIR}/docs/phase35_validation_agent_design_blueprint_v1.json"
export LAYER_CONTRACTS_JSON="${ROOT_DIR}/docs/phase35_validation_agent_layer_contracts_v1.json"
export SLICE_ARTIFACT_POLICY_JSON="${ROOT_DIR}/docs/phase35_contract_slice_artifact_to_policy_v1.json"
export SLICE_POLICY_DRYRUN_JSON="${ROOT_DIR}/docs/phase35_contract_slice_policy_to_dryrun_v1.json"
export SLICE_DRYRUN_APPROVAL_JSON="${ROOT_DIR}/docs/phase35_contract_slice_dryrun_to_approval_v1.json"
export ENTRY_PACK_JSON="${ROOT_DIR}/docs/phase35_entry_pack_v1.json"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export APPROVAL_RECORD_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.json"
export OPERATOR_GATE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_operator_gate_v1.json"
export REVIEW_CYCLE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_review_cycle_bundle_v1.json"
export POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase35_contract_slice_approval_to_audit_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase35_contract_slice_approval_to_audit_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path

now = os.environ["NOW_UTC"]
required = {
    "phase35_validation_agent_design_blueprint": Path(os.environ["BLUEPRINT_JSON"]),
    "phase35_validation_agent_layer_contracts": Path(os.environ["LAYER_CONTRACTS_JSON"]),
    "phase35_contract_slice_artifact_to_policy": Path(os.environ["SLICE_ARTIFACT_POLICY_JSON"]),
    "phase35_contract_slice_policy_to_dryrun": Path(os.environ["SLICE_POLICY_DRYRUN_JSON"]),
    "phase35_contract_slice_dryrun_to_approval": Path(os.environ["SLICE_DRYRUN_APPROVAL_JSON"]),
    "phase35_entry_pack": Path(os.environ["ENTRY_PACK_JSON"]),
    "phase34_validation_agent_approval_contract": Path(os.environ["APPROVAL_CONTRACT_JSON"]),
    "phase34_validation_agent_approval_record": Path(os.environ["APPROVAL_RECORD_JSON"]),
    "phase34_validation_agent_operator_gate": Path(os.environ["OPERATOR_GATE_JSON"]),
    "phase34_validation_agent_review_cycle_bundle": Path(os.environ["REVIEW_CYCLE_JSON"]),
    "phase33_operator_policy": Path(os.environ["POLICY_JSON"]),
    "phase33_baseline_freeze": Path(os.environ["BASELINE_JSON"]),
    "phase33_handoff_pack": Path(os.environ["HANDOFF_JSON"]),
}
triage_path = Path(os.environ["TRIAGE_JSON"])
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])

missing_required = [k for k, p in required.items() if not p.exists()]
parse_errors = []
triage_present = triage_path.exists()

def load_json(path: Path):
    return json.loads(path.read_text())

docs = {}
for name, path in required.items():
    if not path.exists():
        docs[name] = {}
        continue
    try:
        docs[name] = load_json(path)
    except json.JSONDecodeError as exc:
        docs[name] = {}
        missing_required.append(name)
        parse_errors.append(f"{name}:invalid_json:{exc.msg}")

triage = {}
if triage_present:
    try:
        triage = load_json(triage_path)
    except json.JSONDecodeError as exc:
        parse_errors.append(f"phase34_operator_backlog_triage:invalid_json:{exc.msg}")

required_markers = {
    "approval_contract_marker": docs["phase34_validation_agent_approval_contract"].get("marker", ""),
    "approval_record_marker": docs["phase34_validation_agent_approval_record"].get("marker", ""),
    "operator_gate_marker": docs["phase34_validation_agent_operator_gate"].get("marker", ""),
    "policy_marker": docs["phase33_operator_policy"].get("marker", ""),
    "baseline_marker": docs["phase33_baseline_freeze"].get("marker", ""),
    "phase35_blueprint_marker": docs["phase35_validation_agent_design_blueprint"].get("marker", ""),
    "layer_contracts_marker": docs["phase35_validation_agent_layer_contracts"].get("marker", ""),
    "contract_slice_artifact_to_policy_marker": docs["phase35_contract_slice_artifact_to_policy"].get("marker", ""),
    "contract_slice_policy_to_dryrun_marker": docs["phase35_contract_slice_policy_to_dryrun"].get("marker", ""),
    "contract_slice_dryrun_to_approval_marker": docs["phase35_contract_slice_dryrun_to_approval"].get("marker", ""),
    "handoff_marker": docs["phase33_handoff_pack"].get("marker", ""),
}
if triage_present:
    required_markers["triage_marker_optional"] = triage.get("marker", "")

missing_markers = [k for k, v in required_markers.items() if (k != "triage_marker_optional" and not v)]

state_values = [
    docs["phase35_validation_agent_design_blueprint"].get("status", ""),
    docs["phase35_validation_agent_layer_contracts"].get("status", ""),
    docs["phase35_contract_slice_artifact_to_policy"].get("status", ""),
    docs["phase35_contract_slice_policy_to_dryrun"].get("status", ""),
    docs["phase35_contract_slice_dryrun_to_approval"].get("status", ""),
    docs["phase35_entry_pack"].get("status", ""),
    docs["phase34_validation_agent_approval_contract"].get("status", ""),
    docs["phase34_validation_agent_approval_record"].get("status", ""),
    docs["phase34_validation_agent_operator_gate"].get("status", ""),
    docs["phase34_validation_agent_review_cycle_bundle"].get("status", ""),
    docs["phase33_operator_policy"].get("status", ""),
    docs["phase33_baseline_freeze"].get("baseline_status", ""),
    docs["phase33_handoff_pack"].get("status", ""),
]
if triage_present:
    state_values.append(triage.get("status", ""))

blocked = any((v == "blocked" or v.endswith("_blocked")) for v in state_values if v)
notes = any("with_notes" in v for v in state_values if v)

if missing_required or parse_errors or missing_markers:
    status, reason = "contract_slice_blocked", "safe_contract_slice_reference_missing"
elif blocked or notes:
    status, reason = "contract_slice_ready_with_notes", "safe_contract_slice_reference_ready_with_notes"
else:
    status, reason = "contract_slice_ready", "safe_contract_slice_reference_ready"

source_layer_contract = {
    "layer_id": "approval_interface_layer",
    "role_ru": "Передаёт в audit evidence только approval-facing и operator-facing контекст без права исполнения.",
    "approval_facing_fields": [
        "approval_review_context",
        "approval_gate_requirements",
        "decision_ref",
        "approval_contract_ref",
    ],
    "operator_facing_decision_fields": [
        "operator_decision_summary",
        "operator_action_checklist",
        "operator_notes",
    ],
    "required_evidence_refs": ["evidence_marker_set", "traceability_refs", "archive_summary"],
    "forbidden_actions": [
        "runtime_execution",
        "graph_mutation",
        "remediation_actions",
        "implicit_approval_emit",
        "hidden_action_side_effects",
    ],
    "runtime_open_allowed": False,
    "implicit_approval_allowed": False,
}

target_layer_contract = {
    "layer_id": "audit_evidence_layer",
    "role_ru": "Только фиксирует, связывает и проверяет evidence chain без исполнения.",
    "accepted_evidence_packet_fields": [
        "evidence_packet_id",
        "approval_ref",
        "decision_ref",
        "policy_ref",
        "baseline_ref",
        "handoff_ref",
        "evidence_marker_set",
        "traceability_refs",
        "archive_summary",
        "operator_notes",
        "generated_at",
    ],
    "required_archive_traceability_fields": ["archive_summary", "traceability_refs", "evidence_marker_set"],
    "allowed_audit_ready_summaries": [
        "audit_chain_health_summary",
        "traceability_coverage_summary",
        "control_boundary_compliance_summary",
    ],
    "forbidden_actions": [
        "runtime_execution",
        "graph_mutation",
        "remediation_actions",
    ],
    "audit_layer_scope_ru": "Только фиксация/связывание/проверка evidence chain.",
}

evidence_packet_schema = [
    {"field": "evidence_packet_id", "required": True, "type": "string", "description_ru": "Идентификатор evidence packet."},
    {"field": "approval_ref", "required": True, "type": "string", "description_ru": "Ссылка на approval packet/marker."},
    {"field": "decision_ref", "required": True, "type": "string", "description_ru": "Ссылка на operator decision context."},
    {"field": "policy_ref", "required": True, "type": "string", "description_ru": "Ссылка на policy marker/ref."},
    {"field": "baseline_ref", "required": True, "type": "string", "description_ru": "Ссылка на baseline marker/ref."},
    {"field": "handoff_ref", "required": True, "type": "string", "description_ru": "Ссылка на handoff marker/ref."},
    {"field": "evidence_marker_set", "required": True, "type": "array<string>", "description_ru": "Набор маркеров evidence chain."},
    {"field": "traceability_refs", "required": True, "type": "array<string>", "description_ru": "Ссылки трассируемости по цепочке артефактов."},
    {"field": "archive_summary", "required": True, "type": "object", "description_ru": "Сводка архивного состояния evidence."},
    {"field": "operator_notes", "required": True, "type": "array<string>", "description_ru": "Операторские заметки для аудита."},
    {"field": "generated_at", "required": True, "type": "string(datetime)", "description_ru": "UTC-время формирования evidence packet."},
]

interface_invariants = [
    "evidence-only flow",
    "no runtime execution",
    "no graph mutation",
    "no remediation",
    "no hidden side effects",
    "no policy bypass",
    "no baseline bypass",
    "no implicit transition from approval to action",
    "no silent execution fallback",
]

validation_rules = [
    "evidence_packet_has_required_fields",
    "required_markers_present",
    "traceability_refs_are_well_formed",
    "archive_summary_is_well_formed",
    "output_is_compatible_with_audit_layer_input",
    "execution_related_flags_absent",
    "evidence_chain_aligns_with_policy_baseline_chain",
]

rejection_rules = [
    "missing_required_fields",
    "missing_required_markers",
    "stale_policy_or_baseline_refs",
    "malformed_traceability_refs",
    "malformed_archive_summary",
    "malformed_evidence_marker_set",
    "execution_related_flags_present",
    "hidden_action_fields_detected",
    "implicit_approval_fields_detected",
]

recommended_next_contract_slice = {
    "slice_id": "audit_evidence_to_future_runtime_boundary_v1",
    "goal_ru": "Финализировать read-only интерфейс к future runtime boundary без её открытия.",
    "depends_on_current_slice": True,
    "runtime_authorization_change": False,
}

contract_slice_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": sorted(set(missing_required)),
    "missing_required_markers": missing_markers,
    "parse_errors": parse_errors,
    "triage_artifact_present": triage_present,
    "operator_message_ru": "Сформирован корректирующий read-only контракт approval->audit без права на runtime execution.",
}

marker = f"KV_PHASE35_CONTRACT_SLICE_APPROVAL_TO_AUDIT_V1|status={status}|reason={reason}"
payload = {
    "version": "phase35_contract_slice_approval_to_audit_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "contract_slice_status": contract_slice_status,
    "source_layer_contract": source_layer_contract,
    "target_layer_contract": target_layer_contract,
    "evidence_packet_schema": evidence_packet_schema,
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

md = [
    "# Фаза 35.6r1 — Corrective Patch: approval_interface_layer -> audit_evidence_layer",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- Статус: **{status}**",
    f"- Причина: **{reason}**",
    "- Документ фиксирует только read-only/design-only контрактный срез.",
    "- Runtime/approval execution остаются закрытыми.",
    "",
    "## contract_slice_status",
]
for k, v in contract_slice_status.items():
    md.append(f"- {k}: {v}")

for title, section in [
    ("source_layer_contract", source_layer_contract),
    ("target_layer_contract", target_layer_contract),
    ("required_markers", required_markers),
    ("recommended_next_contract_slice", recommended_next_contract_slice),
]:
    md += ["", f"## {title}"]
    for k, v in section.items():
        md.append(f"- {k}: {v}")

md += ["", "## evidence_packet_schema"]
for field in evidence_packet_schema:
    md.append(f"- {field['field']} | required={field['required']} | type={field['type']} | {field['description_ru']}")

for title, values in [
    ("interface_invariants", interface_invariants),
    ("validation_rules", validation_rules),
    ("rejection_rules", rejection_rules),
]:
    md += ["", f"## {title}"]
    for item in values:
        md.append(f"- {item}")

md += ["", "## non_execution_confirmation"]
for k, v in payload["non_execution_confirmation"].items():
    md.append(f"- {k}: {v}")

out_md.write_text("\n".join(md) + "\n")

print("Готово: выполнен корректирующий проход Phase 35.6r1 в read-only режиме.")
print("Исполнение, запись в граф и remediation остаются запрещены.")
print(f"Маркер: {marker}")
PY
