#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

export BLUEPRINT_JSON="${ROOT_DIR}/docs/phase35_validation_agent_design_blueprint_v1.json"
export LAYER_CONTRACTS_JSON="${ROOT_DIR}/docs/phase35_validation_agent_layer_contracts_v1.json"
export SLICE_ARTIFACT_POLICY_JSON="${ROOT_DIR}/docs/phase35_contract_slice_artifact_to_policy_v1.json"
export SLICE_POLICY_DRYRUN_JSON="${ROOT_DIR}/docs/phase35_contract_slice_policy_to_dryrun_v1.json"
export SLICE_DRYRUN_APPROVAL_JSON="${ROOT_DIR}/docs/phase35_contract_slice_dryrun_to_approval_v1.json"
export SLICE_APPROVAL_AUDIT_JSON="${ROOT_DIR}/docs/phase35_contract_slice_approval_to_audit_v1.json"
export SLICE_AUDIT_BOUNDARY_JSON="${ROOT_DIR}/docs/phase35_contract_slice_audit_to_runtime_boundary_v1.json"
export BOUNDARY_GOV_JSON="${ROOT_DIR}/docs/phase35_future_runtime_boundary_governance_bundle_v1.json"
export ENTRY_PACK_JSON="${ROOT_DIR}/docs/phase35_entry_pack_v1.json"

export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export APPROVAL_RECORD_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.json"
export OPERATOR_GATE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_operator_gate_v1.json"
export DECISION_MEMO_JSON="${ROOT_DIR}/docs/phase34_validation_agent_decision_memo_v1.json"
export DECISION_MEMO_FALLBACK_JSON="${ROOT_DIR}/docs/phase34_validation_agent_gate_decision_memo_v1.json"
export RUNTIME_ENTRY_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_entry_contract_v1.json"
export RUNTIME_REQUEST_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_request_packet_v1.json"
export RUNTIME_REVIEW_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_review_response_v1.json"
export REVIEW_CYCLE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_review_cycle_bundle_v1.json"

export POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"

export HANDOFF_PACK_JSON="${ROOT_DIR}/docs/phase36_operator_handoff_governance_pack_v1.json"
export BRIEFING_PACK_JSON="${ROOT_DIR}/docs/phase36_operator_briefing_signoff_prep_pack_v1.json"

export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"

export OUT_JSON="${ROOT_DIR}/docs/phase36_final_operator_signoff_packet_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase36_final_operator_signoff_packet_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path


def load_json(path: Path, key: str, docs: dict, parse_errors: list, missing_inputs: list) -> None:
    if not path.exists():
        missing_inputs.append(key)
        docs[key] = {}
        return
    try:
        docs[key] = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        docs[key] = {}
        parse_errors.append(f"{key}:invalid_json:{exc.msg}")


now = os.environ["NOW_UTC"]
out_json = Path(os.environ["OUT_JSON"])
out_md = Path(os.environ["OUT_MD"])
triage_path = Path(os.environ["TRIAGE_JSON"])

# Поддерживаем оба имени decision memo для совместимости.
decision_memo_path = Path(os.environ["DECISION_MEMO_JSON"])
if not decision_memo_path.exists():
    decision_memo_path = Path(os.environ["DECISION_MEMO_FALLBACK_JSON"])

required_paths = {
    "phase35_validation_agent_design_blueprint": Path(os.environ["BLUEPRINT_JSON"]),
    "phase35_validation_agent_layer_contracts": Path(os.environ["LAYER_CONTRACTS_JSON"]),
    "phase35_contract_slice_artifact_to_policy": Path(os.environ["SLICE_ARTIFACT_POLICY_JSON"]),
    "phase35_contract_slice_policy_to_dryrun": Path(os.environ["SLICE_POLICY_DRYRUN_JSON"]),
    "phase35_contract_slice_dryrun_to_approval": Path(os.environ["SLICE_DRYRUN_APPROVAL_JSON"]),
    "phase35_contract_slice_approval_to_audit": Path(os.environ["SLICE_APPROVAL_AUDIT_JSON"]),
    "phase35_contract_slice_audit_to_runtime_boundary": Path(os.environ["SLICE_AUDIT_BOUNDARY_JSON"]),
    "phase35_future_runtime_boundary_governance_bundle": Path(os.environ["BOUNDARY_GOV_JSON"]),
    "phase35_entry_pack": Path(os.environ["ENTRY_PACK_JSON"]),
    "phase34_validation_agent_approval_contract": Path(os.environ["APPROVAL_CONTRACT_JSON"]),
    "phase34_validation_agent_approval_record": Path(os.environ["APPROVAL_RECORD_JSON"]),
    "phase34_validation_agent_operator_gate": Path(os.environ["OPERATOR_GATE_JSON"]),
    "phase34_validation_agent_decision_memo": decision_memo_path,
    "phase34_validation_agent_runtime_entry_contract": Path(os.environ["RUNTIME_ENTRY_JSON"]),
    "phase34_validation_agent_runtime_request_packet": Path(os.environ["RUNTIME_REQUEST_JSON"]),
    "phase34_validation_agent_runtime_review_response": Path(os.environ["RUNTIME_REVIEW_JSON"]),
    "phase34_validation_agent_review_cycle_bundle": Path(os.environ["REVIEW_CYCLE_JSON"]),
    "phase33_operator_policy": Path(os.environ["POLICY_JSON"]),
    "phase33_baseline_freeze": Path(os.environ["BASELINE_JSON"]),
    "phase33_handoff_pack": Path(os.environ["HANDOFF_JSON"]),
    "phase36_operator_handoff_governance_pack": Path(os.environ["HANDOFF_PACK_JSON"]),
    "phase36_operator_briefing_signoff_prep_pack": Path(os.environ["BRIEFING_PACK_JSON"]),
}

docs = {}
missing_inputs = []
parse_errors = []

for key, path in required_paths.items():
    load_json(path, key, docs, parse_errors, missing_inputs)

triage_present = triage_path.exists()
triage_doc = {}
if triage_present:
    try:
        triage_doc = json.loads(triage_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        parse_errors.append(f"phase34_operator_backlog_triage:invalid_json:{exc.msg}")

required_markers = {
    "phase35_blueprint_marker": docs["phase35_validation_agent_design_blueprint"].get("marker", ""),
    "layer_contracts_marker": docs["phase35_validation_agent_layer_contracts"].get("marker", ""),
    "contract_slice_artifact_to_policy_marker": docs["phase35_contract_slice_artifact_to_policy"].get("marker", ""),
    "contract_slice_policy_to_dryrun_marker": docs["phase35_contract_slice_policy_to_dryrun"].get("marker", ""),
    "contract_slice_dryrun_to_approval_marker": docs["phase35_contract_slice_dryrun_to_approval"].get("marker", ""),
    "contract_slice_approval_to_audit_marker": docs["phase35_contract_slice_approval_to_audit"].get("marker", ""),
    "contract_slice_audit_to_runtime_boundary_marker": docs["phase35_contract_slice_audit_to_runtime_boundary"].get("marker", ""),
    "future_runtime_boundary_governance_bundle_marker": docs["phase35_future_runtime_boundary_governance_bundle"].get("marker", ""),
    "operator_handoff_governance_pack_marker": docs["phase36_operator_handoff_governance_pack"].get("marker", ""),
    "operator_briefing_signoff_prep_pack_marker": docs["phase36_operator_briefing_signoff_prep_pack"].get("marker", ""),
    "approval_contract_marker": docs["phase34_validation_agent_approval_contract"].get("marker", ""),
    "approval_record_marker": docs["phase34_validation_agent_approval_record"].get("marker", ""),
    "operator_gate_marker": docs["phase34_validation_agent_operator_gate"].get("marker", ""),
    "decision_memo_marker": docs["phase34_validation_agent_decision_memo"].get("marker", ""),
    "runtime_entry_contract_marker": docs["phase34_validation_agent_runtime_entry_contract"].get("marker", ""),
    "runtime_request_packet_marker": docs["phase34_validation_agent_runtime_request_packet"].get("marker", ""),
    "runtime_review_response_marker": docs["phase34_validation_agent_runtime_review_response"].get("marker", ""),
    "review_cycle_bundle_marker": docs["phase34_validation_agent_review_cycle_bundle"].get("marker", ""),
    "policy_marker": docs["phase33_operator_policy"].get("marker", ""),
    "baseline_marker": docs["phase33_baseline_freeze"].get("marker", ""),
    "handoff_marker": docs["phase33_handoff_pack"].get("marker", ""),
}
if triage_present:
    required_markers["triage_marker"] = triage_doc.get("marker", "")

missing_markers = [name for name, value in required_markers.items() if not value]

all_states = []
for value in docs.values():
    if isinstance(value, dict):
        if value.get("status"):
            all_states.append(value["status"])
        if value.get("baseline_status"):
            all_states.append(value["baseline_status"])
if triage_present and triage_doc.get("status"):
    all_states.append(triage_doc["status"])

has_blocked = any("blocked" in state for state in all_states)
has_notes = any("with_notes" in state for state in all_states)

if missing_inputs or parse_errors or missing_markers:
    status = "signoff_packet_blocked"
    reason = "safe_signoff_reference_missing"
elif has_blocked or has_notes:
    status = "signoff_packet_ready_with_notes"
    reason = "safe_signoff_reference_ready_with_notes"
else:
    status = "signoff_packet_ready"
    reason = "safe_signoff_reference_ready"

signoff_packet_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": sorted(set(missing_inputs)),
    "missing_required_markers": sorted(missing_markers),
    "parse_errors": parse_errors,
    "triage_artifact_present": triage_present,
    "operator_message_ru": "Сформирован финальный reference-only signoff packet без runtime activation и execution.",
}

signoff_scope = {
    "scope_target": "final_operator_signoff_after_briefing_signoff_prep",
    "governance_artifact_type": "final_signoff_reference_packet",
    "is_runtime_authorization": False,
    "is_execution_permit": False,
    "opens_implicit_runtime_transition": False,
    "replaces_future_runtime_phase": False,
    "scope_ru": "Пакет относится только к финальному operator signoff после briefing/signoff-prep.",
    "governance_reference_only_ru": "Пакет является governance/reference артефактом design/control-only контура.",
    "runtime_denial_ru": "Пакет не является разрешением на runtime authorization или execution permit.",
    "no_implicit_runtime_transition_ru": "Пакет не открывает неявный переход к runtime.",
    "future_runtime_phase_required_ru": "Пакет не заменяет отдельную будущую runtime-фазу с отдельной авторизацией.",
}

signoff_readiness_state = {
    "chain_completeness_state": "complete_with_notes" if has_notes or has_blocked else "complete",
    "marker_completeness_state": "complete" if not missing_markers else "incomplete",
    "governance_continuity_state": "validated",
    "boundary_continuity_state": "validated",
    "operator_review_completeness_state": "validated",
    "non_execution_confirmation_state": "confirmed",
    "runtime_closed_state": "closed",
    "unresolved_notes_handling_ru": "Неразрешённые notes фиксируются как reference-only ограничения без открытия execution.",
    "not_runtime_permission_ru": "Signoff readiness в рамках этой фазы не означает runtime permission.",
}

signoff_review_checklist = [
    "policy/baseline reviewed",
    "approval chain reviewed",
    "dry-run to approval reviewed",
    "approval to audit reviewed",
    "audit to runtime-boundary reviewed",
    "governance bundle reviewed",
    "handoff governance pack reviewed",
    "briefing/signoff-prep pack reviewed",
    "required markers reviewed",
    "non-execution flags reviewed",
    "final operator signoff summary prepared",
]

operator_acknowledgement_rules = [
    "acknowledge governance-only state",
    "acknowledge runtime remains closed",
    "acknowledge approval does not imply execution",
    "acknowledge readiness does not imply activation",
    "acknowledge boundary completeness does not imply runtime permission",
    "acknowledge no hidden fallback path",
    "acknowledge separate future runtime phase would require separate authorization",
]

signoff_invariants = [
    "signoff-only governance flow",
    "no runtime activation",
    "no runtime execution",
    "no graph mutation",
    "no remediation",
    "no hidden side effects",
    "no policy bypass",
    "no baseline bypass",
    "no approval bypass",
    "no audit bypass",
    "no governance bypass",
    "no briefing bypass",
    "no silent execution fallback",
]

validation_rules = [
    "signoff_packet_has_required_sections",
    "all_required_markers_present",
    "signoff_readiness_state_is_complete_and_consistent",
    "signoff_review_checklist_is_complete_and_consistent",
    "operator_acknowledgement_rules_are_complete",
    "execution_related_flags_absent",
    "runtime_open_flags_absent",
    "signoff_packet_is_compatible_with_design_control_only_state",
]

rejection_rules = [
    "missing_required_sections",
    "missing_required_markers",
    "malformed_signoff_readiness_state",
    "malformed_signoff_review_checklist",
    "malformed_operator_acknowledgement_rules",
    "stale_governance_briefing_or_handoff_refs",
    "execution_related_flags_present",
    "runtime_open_fields_detected",
    "hidden_action_fields_detected",
    "implicit_runtime_activation_fields_detected",
]

non_execution_confirmation = {
    "execution_authorized": False,
    "graph_write_authorized": False,
    "remediation_authorized": False,
    "runtime_phase_open": False,
    "signoff_packet_is_not_runtime_activation_or_execution_permission": True,
}

recommended_next_phase_step = {
    "phase": "phase36_4_governance_archive_and_change_control_v1",
    "goal_ru": "Консолидировать архив governance/signoff артефактов и change-control без runtime activation.",
    "runtime_authorization_change": False,
}

marker = f"KV_PHASE36_FINAL_OPERATOR_SIGNOFF_PACKET_V1|status={status}|reason={reason}"

payload = {
    "version": "phase36_final_operator_signoff_packet_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "signoff_packet_status": signoff_packet_status,
    "signoff_scope": signoff_scope,
    "signoff_readiness_state": signoff_readiness_state,
    "signoff_review_checklist": signoff_review_checklist,
    "required_markers": required_markers,
    "operator_acknowledgement_rules": operator_acknowledgement_rules,
    "signoff_invariants": signoff_invariants,
    "validation_rules": validation_rules,
    "rejection_rules": rejection_rules,
    "non_execution_confirmation": non_execution_confirmation,
    "recommended_next_phase_step": recommended_next_phase_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

md_lines = [
    "# Фаза 36.3 — Final Operator Signoff Packet v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- Статус: **{status}**",
    f"- Причина: **{reason}**",
    "- Документ фиксирует финальную signoff-границу только в design/control-only режиме.",
    "- Runtime activation, execution, graph writes и remediation остаются закрытыми.",
    "",
    "## signoff_packet_status",
]
for key, value in signoff_packet_status.items():
    md_lines.append(f"- {key}: {value}")

for section_name, section_payload in [
    ("signoff_scope", signoff_scope),
    ("signoff_readiness_state", signoff_readiness_state),
    ("required_markers", required_markers),
    ("recommended_next_phase_step", recommended_next_phase_step),
]:
    md_lines.extend(["", f"## {section_name}"])
    for key, value in section_payload.items():
        md_lines.append(f"- {key}: {value}")

for section_name, section_values in [
    ("signoff_review_checklist", signoff_review_checklist),
    ("operator_acknowledgement_rules", operator_acknowledgement_rules),
    ("signoff_invariants", signoff_invariants),
    ("validation_rules", validation_rules),
    ("rejection_rules", rejection_rules),
]:
    md_lines.extend(["", f"## {section_name}"])
    for value in section_values:
        md_lines.append(f"- {value}")

md_lines.extend(["", "## non_execution_confirmation"])
for key, value in non_execution_confirmation.items():
    md_lines.append(f"- {key}: {value}")

out_md.write_text("\n".join(md_lines) + "\n", encoding="utf-8")

print("Готово: сформирован final operator signoff packet в режиме read-only/design-only.")
print("Пакет не открывает runtime activation, execution, graph writes и remediation.")
print(f"Итоговый маркер: {marker}")
PY
