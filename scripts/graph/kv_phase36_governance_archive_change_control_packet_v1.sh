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
export FINAL_SIGNOFF_JSON="${ROOT_DIR}/docs/phase36_final_operator_signoff_packet_v1.json"

export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"

export OUT_JSON="${ROOT_DIR}/docs/phase36_governance_archive_change_control_packet_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase36_governance_archive_change_control_packet_v1.md"

python - <<'PY'
import json
import os
from pathlib import Path


def safe_load(path: Path, key: str, docs: dict, missing_inputs: list, parse_errors: list):
    if not path.exists():
        docs[key] = {}
        missing_inputs.append(key)
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
    "phase36_final_operator_signoff_packet": Path(os.environ["FINAL_SIGNOFF_JSON"]),
}

docs = {}
missing_inputs = []
parse_errors = []
for k, p in required_paths.items():
    safe_load(p, k, docs, missing_inputs, parse_errors)

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
    "final_operator_signoff_packet_marker": docs["phase36_final_operator_signoff_packet"].get("marker", ""),
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

missing_markers = [k for k, v in required_markers.items() if not v]

states = []
for d in docs.values():
    if isinstance(d, dict):
        if d.get("status"):
            states.append(d["status"])
        if d.get("baseline_status"):
            states.append(d["baseline_status"])
if triage_present and triage_doc.get("status"):
    states.append(triage_doc["status"])

has_blocked = any("blocked" in s for s in states)
has_notes = any("with_notes" in s for s in states)

if missing_inputs or parse_errors or missing_markers:
    status = "archive_packet_blocked"
    reason = "safe_archive_reference_missing"
elif has_blocked or has_notes:
    status = "archive_packet_ready_with_notes"
    reason = "safe_archive_reference_ready_with_notes"
else:
    status = "archive_packet_ready"
    reason = "safe_archive_reference_ready"

archive_packet_status = {
    "status": status,
    "reason": reason,
    "missing_required_inputs": sorted(set(missing_inputs)),
    "missing_required_markers": sorted(missing_markers),
    "parse_errors": parse_errors,
    "triage_artifact_present": triage_present,
    "operator_message_ru": "Сформирован governance archive + change-control packet в reference-only режиме.",
}

archive_scope = {
    "scope_target": "governance_archive_change_control_after_final_operator_signoff",
    "governance_artifact_type": "archive_and_change_control_reference_packet",
    "is_runtime_authorization": False,
    "is_execution_permit": False,
    "opens_implicit_runtime_transition": False,
    "replaces_future_runtime_phase": False,
    "scope_ru": "Пакет относится только к governance archive + change-control после final operator signoff.",
    "governance_reference_only_ru": "Пакет является финальным governance/reference артефактом.",
}

archived_governance_chain = {
    "phase35_contract_slice_chain_archived": True,
    "future_runtime_boundary_governance_bundle_archived": True,
    "operator_handoff_governance_pack_archived": True,
    "operator_briefing_signoff_prep_pack_archived": True,
    "final_operator_signoff_packet_archived": True,
    "policy_baseline_handoff_continuity_archived": True,
    "runtime_boundary_dependencies_archived": True,
    "operator_review_continuity_archived": True,
}

change_control_rules = [
    "any future change requires separate review",
    "any future runtime-opening change requires separate authorized phase",
    "no implicit reuse of signoff as runtime permission",
    "no silent modification of archived governance chain",
    "no policy/baseline bypass through archive state",
    "no approval/audit bypass through archive state",
    "versioned update expectation for future change packets",
    "operator-visible change traceability requirement",
]

archive_readiness_state = {
    "governance_chain_archived_completeness": "complete_with_notes" if has_blocked or has_notes else "complete",
    "marker_completeness": "complete" if not missing_markers else "incomplete",
    "signoff_packet_availability": "available" if docs["phase36_final_operator_signoff_packet"] else "missing",
    "continuity_preservation": "preserved",
    "change_control_readiness": "ready",
    "non_execution_confirmation": "confirmed",
    "runtime_closed_state": "closed",
    "unresolved_notes_carry_forward_handling_ru": "Notes переносятся в archive/change-control контур без открытия runtime.",
}

change_control_invariants = [
    "archive-only governance flow",
    "change-control-only interpretation",
    "no runtime activation",
    "no runtime execution",
    "no graph mutation",
    "no remediation",
    "no hidden side effects",
    "no policy bypass",
    "no baseline bypass",
    "no approval bypass",
    "no audit bypass",
    "no signoff-to-runtime shortcut",
    "no silent execution fallback",
]

validation_rules = [
    "archive_packet_has_required_sections",
    "all_required_markers_present",
    "archived_governance_chain_is_complete_and_consistent",
    "change_control_rules_are_complete_and_consistent",
    "archive_readiness_state_is_valid",
    "execution_related_flags_absent",
    "runtime_open_flags_absent",
    "archive_packet_is_compatible_with_design_control_only_state",
]

rejection_rules = [
    "missing_required_sections",
    "missing_required_markers",
    "malformed_archived_governance_chain",
    "malformed_change_control_rules",
    "malformed_archive_readiness_state",
    "stale_governance_signoff_refs",
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
    "archive_packet_is_not_runtime_activation_or_execution_permission": True,
}

recommended_next_phase_step = {
    "phase": "phase36_5_governance_maintenance_window_v1",
    "goal_ru": "Подготовить регламент поддержания архива и versioned change-control обновлений без открытия runtime.",
    "runtime_authorization_change": False,
}

marker = f"KV_PHASE36_GOVERNANCE_ARCHIVE_CHANGE_CONTROL_PACKET_V1|status={status}|reason={reason}"

payload = {
    "version": "phase36_governance_archive_change_control_packet_v1",
    "generated_at": now,
    "status": status,
    "reason": reason,
    "marker": marker,
    "archive_packet_status": archive_packet_status,
    "archive_scope": archive_scope,
    "archived_governance_chain": archived_governance_chain,
    "change_control_rules": change_control_rules,
    "required_markers": required_markers,
    "archive_readiness_state": archive_readiness_state,
    "change_control_invariants": change_control_invariants,
    "validation_rules": validation_rules,
    "rejection_rules": rejection_rules,
    "non_execution_confirmation": non_execution_confirmation,
    "recommended_next_phase_step": recommended_next_phase_step,
}
out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

md = [
    "# Фаза 36.4 — Governance Archive + Change-Control Packet v1",
    "",
    f"Сформировано: {now}",
    "",
    f"Маркер: `{marker}`",
    "",
    f"- Статус: **{status}**",
    f"- Причина: **{reason}**",
    "- Пакет фиксирует архив governance-цепочки и рамку change-control без открытия runtime.",
    "",
    "## archive_packet_status",
]
for k, v in archive_packet_status.items():
    md.append(f"- {k}: {v}")
for name, obj in [
    ("archive_scope", archive_scope),
    ("archived_governance_chain", archived_governance_chain),
    ("required_markers", required_markers),
    ("archive_readiness_state", archive_readiness_state),
    ("recommended_next_phase_step", recommended_next_phase_step),
]:
    md.extend(["", f"## {name}"])
    for k, v in obj.items():
        md.append(f"- {k}: {v}")
for name, vals in [
    ("change_control_rules", change_control_rules),
    ("change_control_invariants", change_control_invariants),
    ("validation_rules", validation_rules),
    ("rejection_rules", rejection_rules),
]:
    md.extend(["", f"## {name}"])
    for v in vals:
        md.append(f"- {v}")
md.extend(["", "## non_execution_confirmation"])
for k, v in non_execution_confirmation.items():
    md.append(f"- {k}: {v}")
out_md.write_text("\n".join(md) + "\n", encoding="utf-8")

print("Готово: сформирован governance archive + change-control packet в режиме read-only/design-only.")
print("Runtime activation/execution, graph writes и remediation остаются закрытыми.")
print(f"Итоговый маркер: {marker}")
PY
