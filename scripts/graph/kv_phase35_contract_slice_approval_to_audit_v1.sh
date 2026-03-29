#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
export BLUEPRINT_JSON="${ROOT_DIR}/docs/phase35_validation_agent_design_blueprint_v1.json"
export LAYER_CONTRACTS_JSON="${ROOT_DIR}/docs/phase35_validation_agent_layer_contracts_v1.json"
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
import json, os
from pathlib import Path

now = os.environ['NOW_UTC']
required = {
  'phase35_validation_agent_design_blueprint': Path(os.environ['BLUEPRINT_JSON']),
  'phase35_validation_agent_layer_contracts': Path(os.environ['LAYER_CONTRACTS_JSON']),
  'phase35_contract_slice_dryrun_to_approval': Path(os.environ['SLICE_DRYRUN_APPROVAL_JSON']),
  'phase35_entry_pack': Path(os.environ['ENTRY_PACK_JSON']),
  'phase34_validation_agent_approval_contract': Path(os.environ['APPROVAL_CONTRACT_JSON']),
  'phase34_validation_agent_approval_record': Path(os.environ['APPROVAL_RECORD_JSON']),
  'phase34_validation_agent_operator_gate': Path(os.environ['OPERATOR_GATE_JSON']),
  'phase34_validation_agent_review_cycle_bundle': Path(os.environ['REVIEW_CYCLE_JSON']),
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

def load(p):
  return json.loads(p.read_text())

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
  'review_cycle_marker': docs['phase34_validation_agent_review_cycle_bundle'].get('marker',''),
  'policy_marker': docs['phase33_operator_policy'].get('marker',''),
  'baseline_marker': docs['phase33_baseline_freeze'].get('marker',''),
  'phase35_blueprint_marker': docs['phase35_validation_agent_design_blueprint'].get('marker',''),
  'layer_contracts_marker': docs['phase35_validation_agent_layer_contracts'].get('marker',''),
  'contract_slice_dryrun_to_approval_marker': docs['phase35_contract_slice_dryrun_to_approval'].get('marker',''),
  'handoff_marker': docs['phase33_handoff_pack'].get('marker',''),
}
if triage_present:
  required_markers['triage_marker_optional'] = triage.get('marker','')
missing_markers = [k for k,v in required_markers.items() if (k!='triage_marker_optional' and not v)]

states = [
 docs['phase35_validation_agent_design_blueprint'].get('status',''), docs['phase35_validation_agent_layer_contracts'].get('status',''), docs['phase35_contract_slice_dryrun_to_approval'].get('status',''), docs['phase35_entry_pack'].get('status',''),
 docs['phase34_validation_agent_approval_contract'].get('status',''), docs['phase34_validation_agent_approval_record'].get('status',''), docs['phase34_validation_agent_operator_gate'].get('status',''), docs['phase34_validation_agent_review_cycle_bundle'].get('status',''), docs['phase33_operator_policy'].get('status',''), docs['phase33_baseline_freeze'].get('baseline_status',''), docs['phase33_handoff_pack'].get('status','')
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
 'layer_id':'approval_interface_layer',
 'role_ru':'Готовит approval-facing и operator-facing пакет для аудита без запуска исполнения.',
 'approval_interface_outputs':['approval_review_context','approval_gate_requirements','operator_review_summary','decision_traceability_bundle','risk_visibility_notes'],
 'required_packet_fields':['approval_packet_id','approval_packet_status','recommendation_ref','approval_contract_ref','operator_gate_ref','approval_marker','approval_summary','approval_details','constraint_flags','evidence_refs','generated_at'],
 'not_approval_execution_signal':True,
 'forbidden_actions':['runtime_execution','graph_mutation','remediation_actions','implicit_runtime_open'],
}

target = {
 'layer_id':'audit_evidence_layer',
 'role_ru':'Принимает approval-интерфейсный пакет и строит трассируемую evidence-цепочку в read-only режиме.',
 'accepted_fields':source['required_packet_fields'],
 'allowed_audit_outputs':['audit_evidence_index','traceability_matrix','control_boundary_audit_notes','audit_chain_health_summary'],
 'forbidden_actions':['runtime_execution','graph_mutation','remediation_actions','silent_execution_fallback'],
 'no_runtime_open':True,
}

approval_packet_schema = [
 {'field':'approval_packet_id','required':True,'type':'string','description_ru':'Идентификатор approval packet.'},
 {'field':'approval_packet_status','required':True,'type':'string','description_ru':'Статус approval packet.'},
 {'field':'recommendation_ref','required':True,'type':'string','description_ru':'Ссылка на recommendation packet dry-run слоя.'},
 {'field':'approval_contract_ref','required':True,'type':'string','description_ru':'Ссылка на approval contract marker/ref.'},
 {'field':'operator_gate_ref','required':True,'type':'string','description_ru':'Ссылка на operator gate marker/ref.'},
 {'field':'approval_marker','required':True,'type':'string','description_ru':'Маркер approval interface packet.'},
 {'field':'approval_summary','required':True,'type':'object','description_ru':'Сводка для operator/approval просмотра.'},
 {'field':'approval_details','required':True,'type':'array<object>','description_ru':'Детали approval интерфейса и условий.'},
 {'field':'constraint_flags','required':True,'type':'array<string>','description_ru':'Constraint-флаги policy/baseline/approval chain.'},
 {'field':'evidence_refs','required':True,'type':'array<string>','description_ru':'Ссылки на evidence chain.'},
 {'field':'generated_at','required':True,'type':'string(datetime)','description_ru':'UTC-время генерации packet.'},
]

invariants = [
 'recommendation-and-review-only flow','no runtime execution','no graph mutation','no remediation','no hidden side effects','no policy bypass','no baseline bypass','no implicit transition from approval interface to action','no silent execution fallback'
]
validation_rules = [
 'approval_packet_has_required_fields','required_markers_present','approval_summary_and_details_are_well_formed','evidence_refs_are_well_formed','output_is_compatible_with_audit_evidence_input','execution_related_flags_absent','constraint_flags_align_with_policy_baseline_approval_chain'
]
rejection_rules = [
 'missing_required_fields','missing_required_markers','stale_policy_or_baseline_refs','malformed_approval_summary','malformed_approval_details','malformed_evidence_refs','execution_related_flags_present','hidden_action_fields_detected','implicit_runtime_or_approval_execution_fields_detected'
]
recommended_next = {
 'slice_id':'audit_evidence_to_future_runtime_boundary_v1','goal_ru':'Зафиксировать финальный read-only интерфейс до будущей runtime boundary без её открытия.','depends_on_current_slice':True,'runtime_authorization_change':False
}
contract_status = {
 'status':status,'reason':reason,'missing_required_inputs':sorted(set(missing)),'missing_required_markers':missing_markers,'parse_errors':errors,'triage_artifact_present':triage_present,
 'operator_message_ru':'Сформирован только read-only контракт approval→audit без права на runtime execution.'
}
marker = f"KV_PHASE35_CONTRACT_SLICE_APPROVAL_TO_AUDIT_V1|status={status}|reason={reason}"
payload = {
 'version':'phase35_contract_slice_approval_to_audit_v1','generated_at':now,'status':status,'reason':reason,'marker':marker,
 'contract_slice_status':contract_status,'source_layer_contract':source,'target_layer_contract':target,'approval_packet_schema':approval_packet_schema,'required_markers':required_markers,
 'interface_invariants':invariants,'validation_rules':validation_rules,'rejection_rules':rejection_rules,
 'non_execution_confirmation':{'execution_authorized':False,'graph_write_authorized':False,'remediation_authorized':False,'runtime_phase_open':False,'contract_slice_is_not_runtime_permission':True},
 'recommended_next_contract_slice':recommended_next,
}
out_json.write_text(json.dumps(payload,ensure_ascii=False,indent=2)+'\n')

lines=[
 '# Фаза 35.6 — Contract Slice v4: approval_interface_layer -> audit_evidence_layer','',f'Сформировано: {now}','',f'Маркер: `{marker}`','',
 f'- Статус: **{status}**',f'- Причина: **{reason}**','- Этот документ описывает только read-only интерфейс approval->audit.','- Детальный contract slice не является разрешением на runtime execution.','',
 '## contract_slice_status'
]
for k,v in contract_status.items(): lines.append(f'- {k}: {v}')
for name,sect in [('source_layer_contract',source),('target_layer_contract',target),('required_markers',required_markers),('recommended_next_contract_slice',recommended_next)]:
 lines += ['',f'## {name}']
 for k,v in sect.items(): lines.append(f'- {k}: {v}')
lines += ['', '## approval_packet_schema']
for f in approval_packet_schema: lines.append(f"- {f['field']} | required={f['required']} | type={f['type']} | {f['description_ru']}")
for name,vals in [('interface_invariants',invariants),('validation_rules',validation_rules),('rejection_rules',rejection_rules)]:
 lines += ['',f'## {name}']
 for v in vals: lines.append(f'- {v}')
lines += ['', '## non_execution_confirmation']
for k,v in payload['non_execution_confirmation'].items(): lines.append(f'- {k}: {v}')
out_md.write_text('\n'.join(lines)+'\n')

print('Готово: сформирован contract slice v4 approval->audit в read-only режиме.')
print('Исполнение, запись в граф и remediation остаются запрещены.')
print(f'Маркер: {marker}')
PY
