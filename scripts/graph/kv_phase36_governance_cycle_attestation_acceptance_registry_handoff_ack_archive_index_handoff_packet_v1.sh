#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
for v in \
BLUEPRINT_JSON=phase35_validation_agent_design_blueprint_v1.json \
LAYER_CONTRACTS_JSON=phase35_validation_agent_layer_contracts_v1.json \
SLICE_ARTIFACT_POLICY_JSON=phase35_contract_slice_artifact_to_policy_v1.json \
SLICE_POLICY_DRYRUN_JSON=phase35_contract_slice_policy_to_dryrun_v1.json \
SLICE_DRYRUN_APPROVAL_JSON=phase35_contract_slice_dryrun_to_approval_v1.json \
SLICE_APPROVAL_AUDIT_JSON=phase35_contract_slice_approval_to_audit_v1.json \
SLICE_AUDIT_BOUNDARY_JSON=phase35_contract_slice_audit_to_runtime_boundary_v1.json \
BOUNDARY_GOV_JSON=phase35_future_runtime_boundary_governance_bundle_v1.json \
ENTRY_PACK_JSON=phase35_entry_pack_v1.json \
APPROVAL_CONTRACT_JSON=phase34_validation_agent_approval_contract_v1.json \
APPROVAL_RECORD_JSON=phase34_validation_agent_approval_record_v1.json \
OPERATOR_GATE_JSON=phase34_validation_agent_operator_gate_v1.json \
DECISION_MEMO_JSON=phase34_validation_agent_decision_memo_v1.json \
DECISION_MEMO_FALLBACK_JSON=phase34_validation_agent_gate_decision_memo_v1.json \
RUNTIME_ENTRY_JSON=phase34_validation_agent_runtime_entry_contract_v1.json \
RUNTIME_REQUEST_JSON=phase34_validation_agent_runtime_request_packet_v1.json \
RUNTIME_REVIEW_JSON=phase34_validation_agent_runtime_review_response_v1.json \
REVIEW_CYCLE_JSON=phase34_validation_agent_review_cycle_bundle_v1.json \
POLICY_JSON=phase33_operator_policy_v1.json \
BASELINE_JSON=phase33_baseline_freeze_v1.json \
HANDOFF_JSON=phase33_handoff_pack_v1.json \
HANDOFF_PACK_JSON=phase36_operator_handoff_governance_pack_v1.json \
BRIEFING_PACK_JSON=phase36_operator_briefing_signoff_prep_pack_v1.json \
FINAL_SIGNOFF_JSON=phase36_final_operator_signoff_packet_v1.json \
ARCHIVE_PACKET_JSON=phase36_governance_archive_change_control_packet_v1.json \
MAINTENANCE_PACKET_JSON=phase36_governance_maintenance_window_packet_v1.json \
SUCCESSOR_TEMPLATE_JSON=phase36_versioned_governance_successor_template_packet_v1.json \
SUCCESSOR_REVIEW_JSON=phase36_governance_successor_review_packet_v1.json \
RUNBOOK_JSON=phase36_successor_review_checklist_runbook_packet_v1.json \
OUTCOME_TEMPLATE_JSON=phase36_successor_review_outcome_template_packet_v1.json \
OUTCOME_RECORD_JSON=phase36_governance_outcome_record_packet_v1.json \
RETENTION_PACKET_JSON=phase36_governance_record_retention_packet_v1.json \
RETENTION_AUDIT_LOG_JSON=phase36_governance_retention_audit_log_packet_v1.json \
CLOSURE_MEMO_JSON=phase36_governance_closure_memo_packet_v1.json \
POST_CLOSURE_MONITORING_JSON=phase36_governance_post_closure_monitoring_packet_v1.json \
MONITORING_HANDOFF_JSON=phase36_governance_monitoring_handoff_packet_v1.json \
STEADY_STATE_MEMO_JSON=phase36_governance_steady_state_memo_packet_v1.json \
STEADY_STATE_WATCH_JSON=phase36_governance_steady_state_watch_packet_v1.json \
STEADY_STATE_WATCH_HANDOFF_JSON=phase36_governance_steady_state_watch_handoff_packet_v1.json \
STEADY_STATE_WATCH_ACK_JSON=phase36_governance_steady_state_watch_ack_packet_v1.json \
STEADY_STATE_CYCLE_JSON=phase36_governance_steady_state_cycle_packet_v1.json \
CYCLE_REVIEW_JSON=phase36_governance_cycle_review_packet_v1.json \
CYCLE_CLOSURE_ATTESTATION_JSON=phase36_governance_cycle_closure_attestation_packet_v1.json \
CYCLE_ATTESTATION_ARCHIVE_JSON=phase36_governance_cycle_attestation_archive_packet_v1.json \
CYCLE_ATTESTATION_HANDOVER_JSON=phase36_governance_cycle_attestation_handover_packet_v1.json \
CYCLE_ATTESTATION_ACCEPTANCE_JSON=phase36_governance_cycle_attestation_acceptance_packet_v1.json \
CYCLE_ATTESTATION_ACCEPTANCE_REGISTRY_JSON=phase36_governance_cycle_attestation_acceptance_registry_packet_v1.json \
CYCLE_ATTESTATION_ACCEPTANCE_REGISTRY_HANDOFF_JSON=phase36_governance_cycle_attestation_acceptance_registry_handoff_packet_v1.json \
CYCLE_ATTESTATION_ACCEPTANCE_REGISTRY_HANDOFF_ACK_JSON=phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_packet_v1.json \
CYCLE_ATTESTATION_ACCEPTANCE_REGISTRY_HANDOFF_ACK_ARCHIVE_JSON=phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_archive_packet_v1.json \
CYCLE_ATTESTATION_ACCEPTANCE_REGISTRY_HANDOFF_ACK_ARCHIVE_INDEX_JSON=phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_archive_index_packet_v1.json \
TRIAGE_JSON=phase34_operator_backlog_triage_v1.json \
OUT_JSON=phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_packet_v1.json \
OUT_MD=phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_packet_v1.md
 do
  k="${v%%=*}"; f="${v#*=}"; export "$k=${ROOT_DIR}/docs/$f"
done

python - <<'PY'
import json, os
from pathlib import Path


def load_json(path: Path, key: str, docs: dict, missing: list, errors: list):
    if not path.exists():
        docs[key] = {}
        missing.append(key)
        return
    try:
        docs[key] = json.loads(path.read_text(encoding='utf-8'))
    except json.JSONDecodeError as e:
        docs[key] = {}
        errors.append(f"{key}:invalid_json:{e.msg}")


now = os.environ['NOW_UTC']
out_json = Path(os.environ['OUT_JSON'])
out_md = Path(os.environ['OUT_MD'])
triage_path = Path(os.environ['TRIAGE_JSON'])
decision_memo_path = Path(os.environ['DECISION_MEMO_JSON'])
if not decision_memo_path.exists():
    decision_memo_path = Path(os.environ['DECISION_MEMO_FALLBACK_JSON'])

required_inputs = {
    'phase35_validation_agent_design_blueprint': Path(os.environ['BLUEPRINT_JSON']),
    'phase35_validation_agent_layer_contracts': Path(os.environ['LAYER_CONTRACTS_JSON']),
    'phase35_contract_slice_artifact_to_policy': Path(os.environ['SLICE_ARTIFACT_POLICY_JSON']),
    'phase35_contract_slice_policy_to_dryrun': Path(os.environ['SLICE_POLICY_DRYRUN_JSON']),
    'phase35_contract_slice_dryrun_to_approval': Path(os.environ['SLICE_DRYRUN_APPROVAL_JSON']),
    'phase35_contract_slice_approval_to_audit': Path(os.environ['SLICE_APPROVAL_AUDIT_JSON']),
    'phase35_contract_slice_audit_to_runtime_boundary': Path(os.environ['SLICE_AUDIT_BOUNDARY_JSON']),
    'phase35_future_runtime_boundary_governance_bundle': Path(os.environ['BOUNDARY_GOV_JSON']),
    'phase35_entry_pack': Path(os.environ['ENTRY_PACK_JSON']),
    'phase34_validation_agent_approval_contract': Path(os.environ['APPROVAL_CONTRACT_JSON']),
    'phase34_validation_agent_approval_record': Path(os.environ['APPROVAL_RECORD_JSON']),
    'phase34_validation_agent_operator_gate': Path(os.environ['OPERATOR_GATE_JSON']),
    'phase34_validation_agent_decision_memo': decision_memo_path,
    'phase34_validation_agent_runtime_entry_contract': Path(os.environ['RUNTIME_ENTRY_JSON']),
    'phase34_validation_agent_runtime_request_packet': Path(os.environ['RUNTIME_REQUEST_JSON']),
    'phase34_validation_agent_runtime_review_response': Path(os.environ['RUNTIME_REVIEW_JSON']),
    'phase34_validation_agent_review_cycle_bundle': Path(os.environ['REVIEW_CYCLE_JSON']),
    'phase33_operator_policy': Path(os.environ['POLICY_JSON']),
    'phase33_baseline_freeze': Path(os.environ['BASELINE_JSON']),
    'phase33_handoff_pack': Path(os.environ['HANDOFF_JSON']),
    'phase36_operator_handoff_governance_pack': Path(os.environ['HANDOFF_PACK_JSON']),
    'phase36_operator_briefing_signoff_prep_pack': Path(os.environ['BRIEFING_PACK_JSON']),
    'phase36_final_operator_signoff_packet': Path(os.environ['FINAL_SIGNOFF_JSON']),
    'phase36_governance_archive_change_control_packet': Path(os.environ['ARCHIVE_PACKET_JSON']),
    'phase36_governance_maintenance_window_packet': Path(os.environ['MAINTENANCE_PACKET_JSON']),
    'phase36_versioned_governance_successor_template_packet': Path(os.environ['SUCCESSOR_TEMPLATE_JSON']),
    'phase36_governance_successor_review_packet': Path(os.environ['SUCCESSOR_REVIEW_JSON']),
    'phase36_successor_review_checklist_runbook_packet': Path(os.environ['RUNBOOK_JSON']),
    'phase36_successor_review_outcome_template_packet': Path(os.environ['OUTCOME_TEMPLATE_JSON']),
    'phase36_governance_outcome_record_packet': Path(os.environ['OUTCOME_RECORD_JSON']),
    'phase36_governance_record_retention_packet': Path(os.environ['RETENTION_PACKET_JSON']),
    'phase36_governance_retention_audit_log_packet': Path(os.environ['RETENTION_AUDIT_LOG_JSON']),
    'phase36_governance_closure_memo_packet': Path(os.environ['CLOSURE_MEMO_JSON']),
    'phase36_governance_post_closure_monitoring_packet': Path(os.environ['POST_CLOSURE_MONITORING_JSON']),
    'phase36_governance_monitoring_handoff_packet': Path(os.environ['MONITORING_HANDOFF_JSON']),
    'phase36_governance_steady_state_memo_packet': Path(os.environ['STEADY_STATE_MEMO_JSON']),
    'phase36_governance_steady_state_watch_packet': Path(os.environ['STEADY_STATE_WATCH_JSON']),
    'phase36_governance_steady_state_watch_handoff_packet': Path(os.environ['STEADY_STATE_WATCH_HANDOFF_JSON']),
    'phase36_governance_steady_state_watch_ack_packet': Path(os.environ['STEADY_STATE_WATCH_ACK_JSON']),
    'phase36_governance_steady_state_cycle_packet': Path(os.environ['STEADY_STATE_CYCLE_JSON']),
    'phase36_governance_cycle_review_packet': Path(os.environ['CYCLE_REVIEW_JSON']),
    'phase36_governance_cycle_closure_attestation_packet': Path(os.environ['CYCLE_CLOSURE_ATTESTATION_JSON']),
    'phase36_governance_cycle_attestation_archive_packet': Path(os.environ['CYCLE_ATTESTATION_ARCHIVE_JSON']),
    'phase36_governance_cycle_attestation_handover_packet': Path(os.environ['CYCLE_ATTESTATION_HANDOVER_JSON']),
    'phase36_governance_cycle_attestation_acceptance_packet': Path(os.environ['CYCLE_ATTESTATION_ACCEPTANCE_JSON']),
    'phase36_governance_cycle_attestation_acceptance_registry_packet': Path(os.environ['CYCLE_ATTESTATION_ACCEPTANCE_REGISTRY_JSON']),
    'phase36_governance_cycle_attestation_acceptance_registry_handoff_packet': Path(os.environ['CYCLE_ATTESTATION_ACCEPTANCE_REGISTRY_HANDOFF_JSON']),
    'phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_packet': Path(os.environ['CYCLE_ATTESTATION_ACCEPTANCE_REGISTRY_HANDOFF_ACK_JSON']),
    'phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_archive_packet': Path(os.environ['CYCLE_ATTESTATION_ACCEPTANCE_REGISTRY_HANDOFF_ACK_ARCHIVE_JSON']),
    'phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_archive_index_packet': Path(os.environ['CYCLE_ATTESTATION_ACCEPTANCE_REGISTRY_HANDOFF_ACK_ARCHIVE_INDEX_JSON']),
}

docs = {}
missing_inputs = []
parse_errors = []
for key, path in required_inputs.items():
    load_json(path, key, docs, missing_inputs, parse_errors)

triage_present = triage_path.exists()
triage_doc = {}
if triage_present:
    try:
        triage_doc = json.loads(triage_path.read_text(encoding='utf-8'))
    except json.JSONDecodeError as e:
        parse_errors.append(f"phase34_operator_backlog_triage:invalid_json:{e.msg}")

required_markers = {
    'governance_phase35_blueprint_marker': docs['phase35_validation_agent_design_blueprint'].get('marker', ''),
    'governance_layer_contracts_marker': docs['phase35_validation_agent_layer_contracts'].get('marker', ''),
    'contract_slice_artifact_to_policy_marker': docs['phase35_contract_slice_artifact_to_policy'].get('marker', ''),
    'contract_slice_policy_to_dryrun_marker': docs['phase35_contract_slice_policy_to_dryrun'].get('marker', ''),
    'contract_slice_dryrun_to_approval_marker': docs['phase35_contract_slice_dryrun_to_approval'].get('marker', ''),
    'contract_slice_approval_to_audit_marker': docs['phase35_contract_slice_approval_to_audit'].get('marker', ''),
    'contract_slice_audit_to_runtime_boundary_marker': docs['phase35_contract_slice_audit_to_runtime_boundary'].get('marker', ''),
    'future_runtime_boundary_governance_bundle_marker': docs['phase35_future_runtime_boundary_governance_bundle'].get('marker', ''),
    'governance_entry_pack_marker': docs['phase35_entry_pack'].get('marker', ''),
    'governance_approval_contract_marker': docs['phase34_validation_agent_approval_contract'].get('marker', ''),
    'governance_approval_record_marker': docs['phase34_validation_agent_approval_record'].get('marker', ''),
    'governance_operator_gate_marker': docs['phase34_validation_agent_operator_gate'].get('marker', ''),
    'governance_decision_memo_marker': docs['phase34_validation_agent_decision_memo'].get('marker', ''),
    'governance_runtime_entry_contract_marker': docs['phase34_validation_agent_runtime_entry_contract'].get('marker', ''),
    'governance_runtime_request_packet_marker': docs['phase34_validation_agent_runtime_request_packet'].get('marker', ''),
    'governance_runtime_review_response_marker': docs['phase34_validation_agent_runtime_review_response'].get('marker', ''),
    'governance_review_cycle_bundle_marker': docs['phase34_validation_agent_review_cycle_bundle'].get('marker', ''),
    'governance_policy_marker': docs['phase33_operator_policy'].get('marker', ''),
    'governance_baseline_marker': docs['phase33_baseline_freeze'].get('marker', ''),
    'governance_handoff_marker': docs['phase33_handoff_pack'].get('marker', ''),
    'operator_handoff_governance_pack_marker': docs['phase36_operator_handoff_governance_pack'].get('marker', ''),
    'operator_briefing_signoff_prep_pack_marker': docs['phase36_operator_briefing_signoff_prep_pack'].get('marker', ''),
    'final_operator_signoff_packet_marker': docs['phase36_final_operator_signoff_packet'].get('marker', ''),
    'governance_archive_change_control_packet_marker': docs['phase36_governance_archive_change_control_packet'].get('marker', ''),
    'governance_maintenance_window_packet_marker': docs['phase36_governance_maintenance_window_packet'].get('marker', ''),
    'versioned_governance_successor_template_packet_marker': docs['phase36_versioned_governance_successor_template_packet'].get('marker', ''),
    'governance_successor_review_packet_marker': docs['phase36_governance_successor_review_packet'].get('marker', ''),
    'successor_review_checklist_runbook_packet_marker': docs['phase36_successor_review_checklist_runbook_packet'].get('marker', ''),
    'successor_review_outcome_template_packet_marker': docs['phase36_successor_review_outcome_template_packet'].get('marker', ''),
    'governance_outcome_record_packet_marker': docs['phase36_governance_outcome_record_packet'].get('marker', ''),
    'governance_record_retention_packet_marker': docs['phase36_governance_record_retention_packet'].get('marker', ''),
    'governance_retention_audit_log_packet_marker': docs['phase36_governance_retention_audit_log_packet'].get('marker', ''),
    'governance_closure_memo_packet_marker': docs['phase36_governance_closure_memo_packet'].get('marker', ''),
    'governance_post_closure_monitoring_packet_marker': docs['phase36_governance_post_closure_monitoring_packet'].get('marker', ''),
    'governance_monitoring_handoff_packet_marker': docs['phase36_governance_monitoring_handoff_packet'].get('marker', ''),
    'governance_steady_state_memo_packet_marker': docs['phase36_governance_steady_state_memo_packet'].get('marker', ''),
    'governance_steady_state_cycle_packet_marker': docs['phase36_governance_steady_state_cycle_packet'].get('marker', ''),
    'governance_steady_state_watch_packet_marker': docs['phase36_governance_steady_state_watch_packet'].get('marker', ''),
    'governance_steady_state_watch_handoff_packet_marker': docs['phase36_governance_steady_state_watch_handoff_packet'].get('marker', ''),
    'governance_steady_state_watch_ack_packet_marker': docs['phase36_governance_steady_state_watch_ack_packet'].get('marker', ''),
    'governance_cycle_review_packet_marker': docs['phase36_governance_cycle_review_packet'].get('marker', ''),
    'governance_cycle_closure_attestation_packet_marker': docs['phase36_governance_cycle_closure_attestation_packet'].get('marker', ''),
    'governance_cycle_attestation_archive_packet_marker': docs['phase36_governance_cycle_attestation_archive_packet'].get('marker', ''),
    'governance_cycle_attestation_handover_packet_marker': docs['phase36_governance_cycle_attestation_handover_packet'].get('marker', ''),
    'governance_cycle_attestation_acceptance_packet_marker': docs['phase36_governance_cycle_attestation_acceptance_packet'].get('marker', ''),
    'governance_cycle_attestation_acceptance_registry_packet_marker': docs['phase36_governance_cycle_attestation_acceptance_registry_packet'].get('marker', ''),
    'governance_cycle_attestation_acceptance_registry_handoff_packet_marker': docs['phase36_governance_cycle_attestation_acceptance_registry_handoff_packet'].get('marker', ''),
    'governance_cycle_attestation_acceptance_registry_handoff_ack_packet_marker': docs['phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_packet'].get('marker', ''),
    'governance_cycle_attestation_acceptance_registry_handoff_ack_archive_packet_marker': docs['phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_archive_packet'].get('marker', ''),
    'governance_cycle_attestation_acceptance_registry_handoff_ack_archive_index_packet_marker': docs['phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_archive_index_packet'].get('marker', ''),
}
if triage_present:
    required_markers['triage_marker'] = triage_doc.get('marker', '')

missing_markers = sorted([k for k, v in required_markers.items() if not v])

upstream_statuses = []
for doc in docs.values():
    if isinstance(doc, dict):
        if doc.get('status'):
            upstream_statuses.append(str(doc['status']))
        if doc.get('baseline_status'):
            upstream_statuses.append(str(doc['baseline_status']))
if triage_present and triage_doc.get('status'):
    upstream_statuses.append(str(triage_doc['status']))

has_blocked = any('blocked' in s for s in upstream_statuses)
has_notes = any('with_notes' in s for s in upstream_statuses)
if missing_inputs or parse_errors or missing_markers:
    status = 'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_blocked'
    reason = 'safe_registry_handoff_ack_archive_index_handoff_reference_missing_or_invalid'
elif has_blocked or has_notes:
    status = 'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_ready_with_notes'
    reason = 'safe_registry_handoff_ack_archive_index_handoff_reference_ready_with_notes'
else:
    status = 'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_ready'
    reason = 'safe_registry_handoff_ack_archive_index_handoff_reference_ready'

cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_status = {
    'status': status,
    'reason': reason,
    'missing_required_inputs': sorted(set(missing_inputs)),
    'missing_required_markers': missing_markers,
    'parse_errors': parse_errors,
    'triage_artifact_present': triage_present,
    'operator_message_ru': 'Сформирован governance cycle attestation acceptance registry handoff acknowledgement archive index handoff packet в режиме только чтения.',
}

cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_scope = {
    'scope_target': 'handoff_of_indexed_archive_acknowledgement_for_registry_handoff_acceptance_governance_cycle',
    'governance_reference_only': True,
    'is_runtime_authorization': False,
    'is_execution_permit': False,
    'opens_implicit_runtime_transition': False,
    'replaces_future_runtime_phase': False,
    'scope_ru': 'Передача индексированного archive acknowledgement для registry handoff acceptance governance cycle.',
    'governance_reference_only_ru': 'Артефакт относится только к governance/reference-only режиму.',
    'not_runtime_authorization_ru': 'Артефакт не является runtime authorization.',
    'not_execution_permit_ru': 'Артефакт не является execution permit.',
    'no_implicit_runtime_transition_ru': 'Артефакт не открывает implicit transition к runtime.',
    'does_not_replace_future_runtime_phase_ru': 'Артефакт не заменяет будущую runtime-фазу.',
}

cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_summary = [
    'cycle review completed',
    'cycle closure attestation archived, handed over, accepted, registered, handed over, acknowledged, archived, indexed and handed over',
    'handoff preserves archive-index guardrails',
    'handoff preserves archive-index interpretation',
    'operator-visible handoff summary required',
    'no handoff summary may imply runtime readiness',
]

cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_guardrails = [
    'no runtime activation',
    'no runtime execution',
    'no graph mutation',
    'no remediation',
    'no reinterpretation of archive-index handoff as operational approval',
    'no hidden side effects',
    'no policy bypass',
    'no baseline bypass',
    'no approval bypass',
    'no audit bypass',
    'no archive-index-handoff-to-runtime shortcut',
]

cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_traceability_rules = [
    'handoff linked to cycle attestation acceptance registry handoff ack archive index packet',
    'handoff linked to cycle attestation acceptance registry handoff ack archive packet',
    'handoff linked to cycle attestation acceptance registry handoff ack packet',
    'handoff linked to cycle attestation acceptance registry handoff packet',
    'handoff linked to cycle attestation acceptance registry packet',
    'handoff linked to cycle attestation acceptance packet',
    'handoff linked to cycle attestation handover packet',
    'handoff linked to cycle attestation archive packet',
    'handoff linked to cycle closure attestation packet',
    'handoff linked to cycle review packet',
    'handoff linked to steady-state cycle packet',
    'handoff linked to watch/handoff/ack chain',
    'marker transitions traceable',
    'retained lineage traceable',
    'operator-visible handoff summaries required',
    'no silent handoff replacement',
    'no hidden governance fork via archive-index handoff',
    'no implicit runtime meaning through handoff outputs',
]

cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_invariants = [
    'cycle-attestation-acceptance-registry-handoff-ack-archive-index-handoff-only governance flow',
    'handoff-index-only interpretation',
    'no runtime activation',
    'no runtime execution',
    'no graph mutation',
    'no remediation',
    'no hidden side effects',
    'no policy bypass',
    'no baseline bypass',
    'no approval bypass',
    'no audit bypass',
    'no archive-index-handoff-to-runtime shortcut',
    'no silent execution fallback',
]

validation_rules = [
    'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_packet_has_required_sections',
    'all_required_markers_present',
    'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_summary_is_complete_and_consistent',
    'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_guardrails_are_complete_and_consistent',
    'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_traceability_rules_are_complete_and_consistent',
    'execution_related_flags_absent',
    'runtime_open_flags_absent',
    'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_packet_is_compatible_with_design_control_only_state',
]

rejection_rules = [
    'missing_required_sections',
    'missing_required_markers',
    'malformed_cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_summary',
    'malformed_cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_guardrails',
    'malformed_cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_traceability_rules',
    'stale_acceptance_registry_handoff_ack_archive_index_handoff_refs',
    'execution_related_flags_present',
    'runtime_open_fields_detected',
    'hidden_action_fields_detected',
    'implicit_runtime_activation_fields_detected',
]

non_execution_confirmation = {
    'execution_authorized': False,
    'graph_write_authorized': False,
    'remediation_authorized': False,
    'runtime_phase_open': False,
}

recommended_next_phase_step = {
    'phase': 'phase36_32_governance_cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_ack_packet_v1',
    'goal_ru': 'Подготовить подтверждение получения handoff index-пакета без открытия runtime.',
    'runtime_authorization_change': False,
}

marker = f"KV_PHASE36_GOVERNANCE_CYCLE_ATTESTATION_ACCEPTANCE_REGISTRY_HANDOFF_ACK_ARCHIVE_INDEX_HANDOFF_PACKET_V1|status={status}|reason={reason}"

payload = {
    'version': 'phase36_governance_cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_packet_v1',
    'generated_at': now,
    'status': status,
    'reason': reason,
    'marker': marker,
    'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_status': cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_status,
    'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_scope': cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_scope,
    'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_summary': cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_summary,
    'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_guardrails': cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_guardrails,
    'required_markers': required_markers,
    'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_traceability_rules': cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_traceability_rules,
    'cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_invariants': cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_invariants,
    'validation_rules': validation_rules,
    'rejection_rules': rejection_rules,
    'non_execution_confirmation': non_execution_confirmation,
    'recommended_next_phase_step': recommended_next_phase_step,
}

out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')

md = [
    '# Фаза 36.31 — Пакет передачи индекса архивированного подтверждения registry handoff acceptance governance cycle attestation (v1)',
    '',
    f'Сформировано: {now}',
    '',
    f'Маркер: `{marker}`',
    '',
    f'- Статус: **{status}**',
    f'- Причина: **{reason}**',
    '- Пакет формализует передачу индексированного архивного подтверждения получения handoff зарегистрированного accepted handed-over archived attest-нутого steady-state governance cycle как governance/reference-only артефакт без runtime-активации.',
]

sections_obj = [
    ('Статус передачи индекса архивированного подтверждения (`cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_status`)', cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_status),
    ('Область передачи индекса архивированного подтверждения (`cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_scope`)', cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_scope),
    ('Обязательные маркеры (`required_markers`)', required_markers),
    ('Рекомендуемый следующий шаг (`recommended_next_phase_step`)', recommended_next_phase_step),
    ('Подтверждение non-execution (`non_execution_confirmation`)', non_execution_confirmation),
]
for title, obj in sections_obj:
    md.extend(['', f'## {title}'])
    for k, v in obj.items():
        md.append(f'- `{k}`: {v}')

sections_list = [
    ('Сводка передачи индекса архивированного подтверждения (`cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_summary`)', cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_summary),
    ('Ограничители передачи индекса архивированного подтверждения (`cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_guardrails`)', cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_guardrails),
    ('Правила трассируемости передачи индекса архивированного подтверждения (`cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_traceability_rules`)', cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_traceability_rules),
    ('Инварианты передачи индекса архивированного подтверждения (`cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_invariants`)', cycle_attestation_acceptance_registry_handoff_ack_archive_index_handoff_invariants),
    ('Правила валидации (`validation_rules`)', validation_rules),
    ('Правила отклонения (`rejection_rules`)', rejection_rules),
]
for title, values in sections_list:
    md.extend(['', f'## {title}'])
    for value in values:
        md.append(f'- `{value}`')

out_md.write_text('\n'.join(md) + '\n', encoding='utf-8')

print('Готово: сформирован пакет передачи индекса архивированного подтверждения реестра acceptance registry в режиме только чтения и только дизайна.')
print('Активация runtime, выполнение runtime, изменение графа и remediation не открываются.')
print(f'Итоговый маркер: {marker}')
PY
