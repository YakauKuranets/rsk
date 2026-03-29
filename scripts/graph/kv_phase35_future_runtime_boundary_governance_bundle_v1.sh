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
export ENTRY_PACK_JSON="${ROOT_DIR}/docs/phase35_entry_pack_v1.json"
export APPROVAL_CONTRACT_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_contract_v1.json"
export APPROVAL_RECORD_JSON="${ROOT_DIR}/docs/phase34_validation_agent_approval_record_v1.json"
export OPERATOR_GATE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_operator_gate_v1.json"
export REVIEW_CYCLE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_review_cycle_bundle_v1.json"
export RUNTIME_ENTRY_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_entry_contract_v1.json"
export RUNTIME_REQUEST_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_request_packet_v1.json"
export RUNTIME_REVIEW_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_review_response_v1.json"
export POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase35_future_runtime_boundary_governance_bundle_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase35_future_runtime_boundary_governance_bundle_v1.md"

python - <<'PY'
import json, os
from pathlib import Path

now = os.environ['NOW_UTC']
required = {
 'phase35_validation_agent_design_blueprint': Path(os.environ['BLUEPRINT_JSON']),
 'phase35_validation_agent_layer_contracts': Path(os.environ['LAYER_CONTRACTS_JSON']),
 'phase35_contract_slice_artifact_to_policy': Path(os.environ['SLICE_ARTIFACT_POLICY_JSON']),
 'phase35_contract_slice_policy_to_dryrun': Path(os.environ['SLICE_POLICY_DRYRUN_JSON']),
 'phase35_contract_slice_dryrun_to_approval': Path(os.environ['SLICE_DRYRUN_APPROVAL_JSON']),
 'phase35_contract_slice_approval_to_audit': Path(os.environ['SLICE_APPROVAL_AUDIT_JSON']),
 'phase35_contract_slice_audit_to_runtime_boundary': Path(os.environ['SLICE_AUDIT_BOUNDARY_JSON']),
 'phase35_entry_pack': Path(os.environ['ENTRY_PACK_JSON']),
 'phase34_validation_agent_approval_contract': Path(os.environ['APPROVAL_CONTRACT_JSON']),
 'phase34_validation_agent_approval_record': Path(os.environ['APPROVAL_RECORD_JSON']),
 'phase34_validation_agent_operator_gate': Path(os.environ['OPERATOR_GATE_JSON']),
 'phase34_validation_agent_review_cycle_bundle': Path(os.environ['REVIEW_CYCLE_JSON']),
 'phase34_validation_agent_runtime_entry_contract': Path(os.environ['RUNTIME_ENTRY_JSON']),
 'phase34_validation_agent_runtime_request_packet': Path(os.environ['RUNTIME_REQUEST_JSON']),
 'phase34_validation_agent_runtime_review_response': Path(os.environ['RUNTIME_REVIEW_JSON']),
 'phase33_operator_policy': Path(os.environ['POLICY_JSON']),
 'phase33_baseline_freeze': Path(os.environ['BASELINE_JSON']),
 'phase33_handoff_pack': Path(os.environ['HANDOFF_JSON']),
}
triage_path = Path(os.environ['TRIAGE_JSON'])
out_json = Path(os.environ['OUT_JSON'])
out_md = Path(os.environ['OUT_MD'])

missing = [k for k,p in required.items() if not p.exists()]
errors = []
triage_present = triage_path.exists()

def load(p): return json.loads(p.read_text())

docs = {}
for k,p in required.items():
    if not p.exists():
        docs[k] = {}
        continue
    try: docs[k] = load(p)
    except json.JSONDecodeError as exc:
        docs[k] = {}
        missing.append(k)
        errors.append(f"{k}:invalid_json:{exc.msg}")

triage = {}
if triage_present:
    try: triage = load(triage_path)
    except json.JSONDecodeError as exc: errors.append(f"phase34_operator_backlog_triage:invalid_json:{exc.msg}")

required_markers = {
 'phase35_blueprint_marker': docs['phase35_validation_agent_design_blueprint'].get('marker',''),
 'layer_contracts_marker': docs['phase35_validation_agent_layer_contracts'].get('marker',''),
 'contract_slice_artifact_to_policy_marker': docs['phase35_contract_slice_artifact_to_policy'].get('marker',''),
 'contract_slice_policy_to_dryrun_marker': docs['phase35_contract_slice_policy_to_dryrun'].get('marker',''),
 'contract_slice_dryrun_to_approval_marker': docs['phase35_contract_slice_dryrun_to_approval'].get('marker',''),
 'contract_slice_approval_to_audit_marker': docs['phase35_contract_slice_approval_to_audit'].get('marker',''),
 'contract_slice_audit_to_runtime_boundary_marker': docs['phase35_contract_slice_audit_to_runtime_boundary'].get('marker',''),
 'approval_contract_marker': docs['phase34_validation_agent_approval_contract'].get('marker',''),
 'approval_record_marker': docs['phase34_validation_agent_approval_record'].get('marker',''),
 'operator_gate_marker': docs['phase34_validation_agent_operator_gate'].get('marker',''),
 'runtime_entry_contract_marker': docs['phase34_validation_agent_runtime_entry_contract'].get('marker',''),
 'runtime_request_packet_marker': docs['phase34_validation_agent_runtime_request_packet'].get('marker',''),
 'runtime_review_response_marker': docs['phase34_validation_agent_runtime_review_response'].get('marker',''),
 'policy_marker': docs['phase33_operator_policy'].get('marker',''),
 'baseline_marker': docs['phase33_baseline_freeze'].get('marker',''),
 'handoff_marker': docs['phase33_handoff_pack'].get('marker',''),
}
if triage_present: required_markers['triage_marker_optional'] = triage.get('marker','')
missing_markers = [k for k,v in required_markers.items() if (k!='triage_marker_optional' and not v)]

states = [
 docs['phase35_validation_agent_design_blueprint'].get('status',''), docs['phase35_validation_agent_layer_contracts'].get('status',''), docs['phase35_contract_slice_artifact_to_policy'].get('status',''), docs['phase35_contract_slice_policy_to_dryrun'].get('status',''), docs['phase35_contract_slice_dryrun_to_approval'].get('status',''), docs['phase35_contract_slice_approval_to_audit'].get('status',''), docs['phase35_contract_slice_audit_to_runtime_boundary'].get('status',''), docs['phase35_entry_pack'].get('status',''),
 docs['phase34_validation_agent_approval_contract'].get('status',''), docs['phase34_validation_agent_approval_record'].get('status',''), docs['phase34_validation_agent_operator_gate'].get('status',''), docs['phase34_validation_agent_review_cycle_bundle'].get('status',''), docs['phase34_validation_agent_runtime_entry_contract'].get('status',''), docs['phase34_validation_agent_runtime_request_packet'].get('status',''), docs['phase34_validation_agent_runtime_review_response'].get('status',''), docs['phase33_operator_policy'].get('status',''), docs['phase33_baseline_freeze'].get('baseline_status',''), docs['phase33_handoff_pack'].get('status','')
]
if triage_present: states.append(triage.get('status',''))
blocked = any(v=='blocked' or v.endswith('_blocked') for v in states if v)
notes = any('with_notes' in v for v in states if v)

if missing or errors or missing_markers:
    status, reason = 'governance_bundle_blocked', 'safe_governance_reference_missing'
elif blocked or notes:
    status, reason = 'governance_bundle_ready_with_notes', 'safe_governance_reference_ready_with_notes'
else:
    status, reason = 'governance_bundle_ready', 'safe_governance_reference_ready'

governance_bundle_status = {
 'status':status,'reason':reason,'missing_required_inputs':sorted(set(missing)),'missing_required_markers':missing_markers,'parse_errors':errors,'triage_artifact_present':triage_present,
 'operator_message_ru':'Сформирован финальный governance bundle для future_runtime_boundary_layer без runtime activation.'
}
boundary_governance_scope = {
 'governance_target_layer':'future_runtime_boundary_layer',
 'governance_artifact_type':'reference_only_governance_bundle',
 'is_runtime_authorization':False,
 'is_execution_permit':False,
 'opens_implicit_runtime_transition':False,
 'scope_ru':'Артефакт только для governance/reference фиксации boundary-ограничений.'
}
boundary_constraints_registry = {
 'policy_constraints':['policy_marker_required','no_policy_bypass'],
 'baseline_constraints':['baseline_marker_required','no_baseline_bypass'],
 'approval_chain_constraints':['approval_contract_marker_required','approval_record_marker_required','operator_gate_marker_required'],
 'audit_evidence_chain_constraints':['contract_slice_approval_to_audit_marker_required','contract_slice_audit_to_runtime_boundary_marker_required','traceability_chain_must_be_consistent'],
 'runtime_entry_dependency_constraints':['runtime_entry_contract_marker_required','runtime_request_packet_marker_required'],
 'review_response_dependency_constraints':['runtime_review_response_marker_required','review_response_consistency_required'],
 'handoff_dependency_constraints':['handoff_marker_required'],
 'operator_control_constraints':['operator_gate_marker_required','operator_notes_traceability_required'],
 'no_execution_constraints':['execution_authorized_must_be_false','no_runtime_execution'],
 'no_graph_write_constraints':['graph_write_authorized_must_be_false','no_graph_mutation'],
 'no_remediation_constraints':['remediation_authorized_must_be_false','no_remediation_actions'],
}
interface_chain_summary = [
 'artifact_intake_layer -> policy_reasoning_layer',
 'policy_reasoning_layer -> dry_run_recommendation_layer',
 'dry_run_recommendation_layer -> approval_interface_layer',
 'approval_interface_layer -> audit_evidence_layer',
 'audit_evidence_layer -> future_runtime_boundary_layer',
]
governance_invariants = [
 'governance-only consolidation','no runtime activation','no runtime execution','no graph mutation','no remediation','no hidden side effects','no policy bypass','no baseline bypass','no approval bypass','no audit bypass','no silent execution fallback','no implicit boundary-to-runtime transition'
]
validation_rules = [
 'governance_bundle_has_required_sections','all_required_markers_present','interface_chain_summary_is_complete_and_consistent','boundary_constraints_registry_is_complete_and_consistent','upstream_contract_slice_chain_is_consistent','execution_related_flags_absent','runtime_open_flags_absent','governance_bundle_is_compatible_with_design_control_boundary_state'
]
rejection_rules = [
 'missing_required_sections','missing_required_markers','malformed_chain_summary','malformed_boundary_constraints_registry','stale_policy_or_baseline_refs','stale_approval_or_audit_refs','execution_related_flags_present','runtime_open_fields_detected','hidden_action_fields_detected','implicit_runtime_activation_fields_detected'
]
recommended_next_phase = {
 'phase':'phase36_design_control_handoff_preparation_v1','goal_ru':'Подготовить handoff governance пакет для оператора, сохраняя runtime закрытым.','runtime_authorization_change':False
}
marker = f"KV_PHASE35_FUTURE_RUNTIME_BOUNDARY_GOVERNANCE_BUNDLE_V1|status={status}|reason={reason}"
payload = {
 'version':'phase35_future_runtime_boundary_governance_bundle_v1','generated_at':now,'status':status,'reason':reason,'marker':marker,
 'governance_bundle_status':governance_bundle_status,
 'boundary_governance_scope':boundary_governance_scope,
 'boundary_constraints_registry':boundary_constraints_registry,
 'required_markers':required_markers,
 'interface_chain_summary':interface_chain_summary,
 'governance_invariants':governance_invariants,
 'validation_rules':validation_rules,
 'rejection_rules':rejection_rules,
 'non_execution_confirmation':{'execution_authorized':False,'graph_write_authorized':False,'remediation_authorized':False,'runtime_phase_open':False,'governance_bundle_is_not_runtime_activation_or_execution_permission':True},
 'recommended_next_phase':recommended_next_phase,
}
out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2)+'\n')

md=['# Фаза 35.8 — Future Runtime Boundary Governance Bundle v1','',f'Сформировано: {now}','',f'Маркер: `{marker}`','',f'- Статус: **{status}**',f'- Причина: **{reason}**','- Документ только для governance/reference консолидации.','- Runtime activation/execution остаются запрещены.','', '## governance_bundle_status']
for k,v in governance_bundle_status.items(): md.append(f'- {k}: {v}')
for name,section in [('boundary_governance_scope',boundary_governance_scope),('boundary_constraints_registry',boundary_constraints_registry),('required_markers',required_markers),('recommended_next_phase',recommended_next_phase)]:
 md += ['',f'## {name}']
 for k,v in section.items(): md.append(f'- {k}: {v}')
md += ['', '## interface_chain_summary']
for i in interface_chain_summary: md.append(f'- {i}')
for name,vals in [('governance_invariants',governance_invariants),('validation_rules',validation_rules),('rejection_rules',rejection_rules)]:
 md += ['',f'## {name}']
 for v in vals: md.append(f'- {v}')
md += ['', '## non_execution_confirmation']
for k,v in payload['non_execution_confirmation'].items(): md.append(f'- {k}: {v}')
out_md.write_text('\n'.join(md)+'\n')

print('Готово: сформирован governance bundle future_runtime_boundary_layer в read-only режиме.')
print('Исполнение, запись в граф и remediation остаются запрещены.')
print(f'Маркер: {marker}')
PY
