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
export OUT_JSON="${ROOT_DIR}/docs/phase35_contract_slice_audit_to_runtime_boundary_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase35_contract_slice_audit_to_runtime_boundary_v1.md"

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
    try:
        docs[k] = load(p)
    except json.JSONDecodeError as exc:
        docs[k] = {}
        missing.append(k)
        errors.append(f"{k}:invalid_json:{exc.msg}")

triage = {}
if triage_present:
    try: triage = load(triage_path)
    except json.JSONDecodeError as exc: errors.append(f"phase34_operator_backlog_triage:invalid_json:{exc.msg}")

required_markers = {
'approval_contract_marker': docs['phase34_validation_agent_approval_contract'].get('marker',''),
'approval_record_marker': docs['phase34_validation_agent_approval_record'].get('marker',''),
'operator_gate_marker': docs['phase34_validation_agent_operator_gate'].get('marker',''),
'policy_marker': docs['phase33_operator_policy'].get('marker',''),
'baseline_marker': docs['phase33_baseline_freeze'].get('marker',''),
'phase35_blueprint_marker': docs['phase35_validation_agent_design_blueprint'].get('marker',''),
'layer_contracts_marker': docs['phase35_validation_agent_layer_contracts'].get('marker',''),
'contract_slice_approval_to_audit_marker': docs['phase35_contract_slice_approval_to_audit'].get('marker',''),
'runtime_entry_contract_marker': docs['phase34_validation_agent_runtime_entry_contract'].get('marker',''),
'runtime_request_packet_marker': docs['phase34_validation_agent_runtime_request_packet'].get('marker',''),
'runtime_review_response_marker': docs['phase34_validation_agent_runtime_review_response'].get('marker',''),
'handoff_marker': docs['phase33_handoff_pack'].get('marker',''),
}
if triage_present: required_markers['triage_marker_optional'] = triage.get('marker','')
missing_markers = [k for k,v in required_markers.items() if (k!='triage_marker_optional' and not v)]

states = [
 docs['phase35_validation_agent_design_blueprint'].get('status',''), docs['phase35_validation_agent_layer_contracts'].get('status',''), docs['phase35_contract_slice_artifact_to_policy'].get('status',''), docs['phase35_contract_slice_policy_to_dryrun'].get('status',''), docs['phase35_contract_slice_dryrun_to_approval'].get('status',''), docs['phase35_contract_slice_approval_to_audit'].get('status',''), docs['phase35_entry_pack'].get('status',''),
 docs['phase34_validation_agent_approval_contract'].get('status',''), docs['phase34_validation_agent_approval_record'].get('status',''), docs['phase34_validation_agent_operator_gate'].get('status',''), docs['phase34_validation_agent_review_cycle_bundle'].get('status',''), docs['phase34_validation_agent_runtime_entry_contract'].get('status',''), docs['phase34_validation_agent_runtime_request_packet'].get('status',''), docs['phase34_validation_agent_runtime_review_response'].get('status',''), docs['phase33_operator_policy'].get('status',''), docs['phase33_baseline_freeze'].get('baseline_status',''), docs['phase33_handoff_pack'].get('status','')
]
if triage_present: states.append(triage.get('status',''))
blocked = any(v=='blocked' or v.endswith('_blocked') for v in states if v)
notes = any('with_notes' in v for v in states if v)

if missing or errors or missing_markers:
    status, reason = 'contract_slice_blocked', 'safe_contract_slice_reference_missing'
elif blocked or notes:
    status, reason = 'contract_slice_ready_with_notes', 'safe_contract_slice_reference_ready_with_notes'
else:
    status, reason = 'contract_slice_ready', 'safe_contract_slice_reference_ready'

source = {
 'layer_id':'audit_evidence_layer',
 'role_ru':'Передаёт в boundary слой только audit-facing доказательную связность и ограничения.',
 'audit_facing_fields':['audit_evidence_index','traceability_matrix','audit_chain_health_summary','control_boundary_audit_notes'],
 'required_traceability_fields':['traceability_refs','evidence_chain_ref','approval_ref'],
 'required_archive_summary_fields':['archive_summary','archive_hash_ref','archive_window_ref'],
 'forbidden_actions':['runtime_execution','graph_mutation','remediation_actions'],
 'audit_scope_ru':'Только фиксация/связывание/проверка evidence chain.',
}

target = {
 'layer_id':'future_runtime_boundary_layer',
 'role_ru':'Принимает boundary packet и фиксирует только ограничения и readiness conditions без открытия runtime.',
 'accepted_boundary_packet_fields':['boundary_packet_id','audit_ref','approval_ref','policy_ref','baseline_ref','runtime_entry_ref','runtime_request_ref','review_response_ref','boundary_constraints','traceability_refs','operator_notes','generated_at'],
 'required_constraint_fields':['boundary_constraints','policy_ref','baseline_ref','runtime_entry_ref','runtime_request_ref','review_response_ref'],
 'allowed_runtime_boundary_summaries':['runtime_boundary_readiness_summary','boundary_constraint_summary','non_activation_guardrail_summary'],
 'runtime_activation_allowed':False,
 'runtime_execution_allowed':False,
}

runtime_boundary_packet_schema = [
 {'field':'boundary_packet_id','required':True,'type':'string','description_ru':'Идентификатор boundary packet.'},
 {'field':'audit_ref','required':True,'type':'string','description_ru':'Ссылка на audit evidence пакет.'},
 {'field':'approval_ref','required':True,'type':'string','description_ru':'Ссылка на approval интерфейсный пакет.'},
 {'field':'policy_ref','required':True,'type':'string','description_ru':'Ссылка на policy marker/ref.'},
 {'field':'baseline_ref','required':True,'type':'string','description_ru':'Ссылка на baseline marker/ref.'},
 {'field':'runtime_entry_ref','required':True,'type':'string','description_ru':'Ссылка на runtime entry contract marker/ref.'},
 {'field':'runtime_request_ref','required':True,'type':'string','description_ru':'Ссылка на runtime request packet marker/ref.'},
 {'field':'review_response_ref','required':True,'type':'string','description_ru':'Ссылка на runtime review response marker/ref.'},
 {'field':'boundary_constraints','required':True,'type':'array<string>','description_ru':'Boundary constraints для runtime boundary слоя.'},
 {'field':'traceability_refs','required':True,'type':'array<string>','description_ru':'Ссылки трассируемости цепочки артефактов.'},
 {'field':'operator_notes','required':True,'type':'array<string>','description_ru':'Операторские заметки по boundary contract.'},
 {'field':'generated_at','required':True,'type':'string(datetime)','description_ru':'UTC-время формирования boundary packet.'},
]

invariants = [
 'constraint-only flow','no runtime activation','no runtime execution','no graph mutation','no remediation','no hidden side effects','no policy bypass','no baseline bypass','no implicit transition from boundary contract to action','no silent execution fallback'
]
validation_rules = [
 'runtime_boundary_packet_has_required_fields','required_markers_present','traceability_refs_are_well_formed','boundary_constraints_are_well_formed','output_is_compatible_with_future_runtime_boundary_input','execution_related_flags_absent','runtime_open_flags_absent','evidence_chain_aligns_with_policy_baseline_approval_audit_chain'
]
rejection_rules = [
 'missing_required_fields','missing_required_markers','stale_policy_or_baseline_refs','malformed_traceability_refs','malformed_boundary_constraints','execution_related_flags_present','runtime_open_fields_detected','hidden_action_fields_detected','implicit_runtime_activation_fields_detected'
]
recommended_next = {
 'slice_id':'future_runtime_boundary_governance_bundle_v1','goal_ru':'Сформировать финальный design-control governance пакет boundary-ограничений без runtime activation.','depends_on_current_slice':True,'runtime_authorization_change':False
}
contract_status = {
 'status':status,'reason':reason,'missing_required_inputs':sorted(set(missing)),'missing_required_markers':missing_markers,'parse_errors':errors,'triage_artifact_present':triage_present,
 'operator_message_ru':'Сформирован read-only boundary контракт audit->future runtime boundary без права на runtime activation.'
}
marker = f"KV_PHASE35_CONTRACT_SLICE_AUDIT_TO_RUNTIME_BOUNDARY_V1|status={status}|reason={reason}"
payload = {
 'version':'phase35_contract_slice_audit_to_runtime_boundary_v1','generated_at':now,'status':status,'reason':reason,'marker':marker,
 'contract_slice_status':contract_status,'source_layer_contract':source,'target_layer_contract':target,'runtime_boundary_packet_schema':runtime_boundary_packet_schema,
 'required_markers':required_markers,'interface_invariants':invariants,'validation_rules':validation_rules,'rejection_rules':rejection_rules,
 'non_execution_confirmation':{'execution_authorized':False,'graph_write_authorized':False,'remediation_authorized':False,'runtime_phase_open':False,'contract_slice_is_not_runtime_activation_permission':True},
 'recommended_next_contract_slice':recommended_next,
}
out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2)+'\n')

md=[
'# Фаза 35.7 — Contract Slice v5: audit_evidence_layer -> future_runtime_boundary_layer','',f'Сформировано: {now}','',f'Маркер: `{marker}`','',
f'- Статус: **{status}**',f'- Причина: **{reason}**','- Это только read-only/design-only boundary contract.','- Runtime activation и execution запрещены.','',
'## contract_slice_status'
]
for k,v in contract_status.items(): md.append(f'- {k}: {v}')
for name,section in [('source_layer_contract',source),('target_layer_contract',target),('required_markers',required_markers),('recommended_next_contract_slice',recommended_next)]:
 md += ['',f'## {name}']
 for k,v in section.items(): md.append(f'- {k}: {v}')
md += ['', '## runtime_boundary_packet_schema']
for f in runtime_boundary_packet_schema: md.append(f"- {f['field']} | required={f['required']} | type={f['type']} | {f['description_ru']}")
for name,vals in [('interface_invariants',invariants),('validation_rules',validation_rules),('rejection_rules',rejection_rules)]:
 md += ['',f'## {name}']
 for v in vals: md.append(f'- {v}')
md += ['', '## non_execution_confirmation']
for k,v in payload['non_execution_confirmation'].items(): md.append(f'- {k}: {v}')
out_md.write_text('\n'.join(md)+'\n')

print('Готово: сформирован contract slice v5 audit->future runtime boundary в read-only режиме.')
print('Исполнение, запись в граф и remediation остаются запрещены.')
print(f'Маркер: {marker}')
PY
