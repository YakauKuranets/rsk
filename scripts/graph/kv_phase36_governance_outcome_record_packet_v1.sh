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
export ARCHIVE_PACKET_JSON="${ROOT_DIR}/docs/phase36_governance_archive_change_control_packet_v1.json"
export MAINTENANCE_PACKET_JSON="${ROOT_DIR}/docs/phase36_governance_maintenance_window_packet_v1.json"
export SUCCESSOR_TEMPLATE_JSON="${ROOT_DIR}/docs/phase36_versioned_governance_successor_template_packet_v1.json"
export SUCCESSOR_REVIEW_JSON="${ROOT_DIR}/docs/phase36_governance_successor_review_packet_v1.json"
export RUNBOOK_JSON="${ROOT_DIR}/docs/phase36_successor_review_checklist_runbook_packet_v1.json"
export OUTCOME_TEMPLATE_JSON="${ROOT_DIR}/docs/phase36_successor_review_outcome_template_packet_v1.json"

export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"

export OUT_JSON="${ROOT_DIR}/docs/phase36_governance_outcome_record_packet_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase36_governance_outcome_record_packet_v1.md"

python - <<'PY'
import json, os
from pathlib import Path

def safe_load(path,key,docs,missing,errors):
    if not path.exists(): docs[key]={}; missing.append(key); return
    try: docs[key]=json.loads(path.read_text(encoding='utf-8'))
    except json.JSONDecodeError as exc: docs[key]={}; errors.append(f"{key}:invalid_json:{exc.msg}")

now=os.environ['NOW_UTC']; out_json=Path(os.environ['OUT_JSON']); out_md=Path(os.environ['OUT_MD']); triage_path=Path(os.environ['TRIAGE_JSON'])
decision=Path(os.environ['DECISION_MEMO_JSON']);
if not decision.exists(): decision=Path(os.environ['DECISION_MEMO_FALLBACK_JSON'])
req={
 'phase35_validation_agent_design_blueprint':Path(os.environ['BLUEPRINT_JSON']),'phase35_validation_agent_layer_contracts':Path(os.environ['LAYER_CONTRACTS_JSON']),'phase35_contract_slice_artifact_to_policy':Path(os.environ['SLICE_ARTIFACT_POLICY_JSON']),'phase35_contract_slice_policy_to_dryrun':Path(os.environ['SLICE_POLICY_DRYRUN_JSON']),'phase35_contract_slice_dryrun_to_approval':Path(os.environ['SLICE_DRYRUN_APPROVAL_JSON']),'phase35_contract_slice_approval_to_audit':Path(os.environ['SLICE_APPROVAL_AUDIT_JSON']),'phase35_contract_slice_audit_to_runtime_boundary':Path(os.environ['SLICE_AUDIT_BOUNDARY_JSON']),'phase35_future_runtime_boundary_governance_bundle':Path(os.environ['BOUNDARY_GOV_JSON']),'phase35_entry_pack':Path(os.environ['ENTRY_PACK_JSON']),'phase34_validation_agent_approval_contract':Path(os.environ['APPROVAL_CONTRACT_JSON']),'phase34_validation_agent_approval_record':Path(os.environ['APPROVAL_RECORD_JSON']),'phase34_validation_agent_operator_gate':Path(os.environ['OPERATOR_GATE_JSON']),'phase34_validation_agent_decision_memo':decision,'phase34_validation_agent_runtime_entry_contract':Path(os.environ['RUNTIME_ENTRY_JSON']),'phase34_validation_agent_runtime_request_packet':Path(os.environ['RUNTIME_REQUEST_JSON']),'phase34_validation_agent_runtime_review_response':Path(os.environ['RUNTIME_REVIEW_JSON']),'phase34_validation_agent_review_cycle_bundle':Path(os.environ['REVIEW_CYCLE_JSON']),'phase33_operator_policy':Path(os.environ['POLICY_JSON']),'phase33_baseline_freeze':Path(os.environ['BASELINE_JSON']),'phase33_handoff_pack':Path(os.environ['HANDOFF_JSON']),'phase36_operator_handoff_governance_pack':Path(os.environ['HANDOFF_PACK_JSON']),'phase36_operator_briefing_signoff_prep_pack':Path(os.environ['BRIEFING_PACK_JSON']),'phase36_final_operator_signoff_packet':Path(os.environ['FINAL_SIGNOFF_JSON']),'phase36_governance_archive_change_control_packet':Path(os.environ['ARCHIVE_PACKET_JSON']),'phase36_governance_maintenance_window_packet':Path(os.environ['MAINTENANCE_PACKET_JSON']),'phase36_versioned_governance_successor_template_packet':Path(os.environ['SUCCESSOR_TEMPLATE_JSON']),'phase36_governance_successor_review_packet':Path(os.environ['SUCCESSOR_REVIEW_JSON']),'phase36_successor_review_checklist_runbook_packet':Path(os.environ['RUNBOOK_JSON']),'phase36_successor_review_outcome_template_packet':Path(os.environ['OUTCOME_TEMPLATE_JSON'])}
docs={};missing=[];errors=[]
for k,p in req.items(): safe_load(p,k,docs,missing,errors)
triage_present=triage_path.exists(); triage={}
if triage_present:
    try: triage=json.loads(triage_path.read_text(encoding='utf-8'))
    except json.JSONDecodeError as exc: errors.append(f"phase34_operator_backlog_triage:invalid_json:{exc.msg}")
markers={
 'phase35_blueprint_marker':docs['phase35_validation_agent_design_blueprint'].get('marker',''),'layer_contracts_marker':docs['phase35_validation_agent_layer_contracts'].get('marker',''),'contract_slice_artifact_to_policy_marker':docs['phase35_contract_slice_artifact_to_policy'].get('marker',''),'contract_slice_policy_to_dryrun_marker':docs['phase35_contract_slice_policy_to_dryrun'].get('marker',''),'contract_slice_dryrun_to_approval_marker':docs['phase35_contract_slice_dryrun_to_approval'].get('marker',''),'contract_slice_approval_to_audit_marker':docs['phase35_contract_slice_approval_to_audit'].get('marker',''),'contract_slice_audit_to_runtime_boundary_marker':docs['phase35_contract_slice_audit_to_runtime_boundary'].get('marker',''),'future_runtime_boundary_governance_bundle_marker':docs['phase35_future_runtime_boundary_governance_bundle'].get('marker',''),'operator_handoff_governance_pack_marker':docs['phase36_operator_handoff_governance_pack'].get('marker',''),'operator_briefing_signoff_prep_pack_marker':docs['phase36_operator_briefing_signoff_prep_pack'].get('marker',''),'final_operator_signoff_packet_marker':docs['phase36_final_operator_signoff_packet'].get('marker',''),'governance_archive_change_control_packet_marker':docs['phase36_governance_archive_change_control_packet'].get('marker',''),'governance_maintenance_window_packet_marker':docs['phase36_governance_maintenance_window_packet'].get('marker',''),'versioned_governance_successor_template_packet_marker':docs['phase36_versioned_governance_successor_template_packet'].get('marker',''),'governance_successor_review_packet_marker':docs['phase36_governance_successor_review_packet'].get('marker',''),'successor_review_checklist_runbook_packet_marker':docs['phase36_successor_review_checklist_runbook_packet'].get('marker',''),'successor_review_outcome_template_packet_marker':docs['phase36_successor_review_outcome_template_packet'].get('marker',''),'approval_contract_marker':docs['phase34_validation_agent_approval_contract'].get('marker',''),'approval_record_marker':docs['phase34_validation_agent_approval_record'].get('marker',''),'operator_gate_marker':docs['phase34_validation_agent_operator_gate'].get('marker',''),'decision_memo_marker':docs['phase34_validation_agent_decision_memo'].get('marker',''),'runtime_entry_contract_marker':docs['phase34_validation_agent_runtime_entry_contract'].get('marker',''),'runtime_request_packet_marker':docs['phase34_validation_agent_runtime_request_packet'].get('marker',''),'runtime_review_response_marker':docs['phase34_validation_agent_runtime_review_response'].get('marker',''),'review_cycle_bundle_marker':docs['phase34_validation_agent_review_cycle_bundle'].get('marker',''),'policy_marker':docs['phase33_operator_policy'].get('marker',''),'baseline_marker':docs['phase33_baseline_freeze'].get('marker',''),'handoff_marker':docs['phase33_handoff_pack'].get('marker','')}
if triage_present: markers['triage_marker']=triage.get('marker','')
missing_markers=[k for k,v in markers.items() if not v]
states=[]
for d in docs.values():
    if isinstance(d,dict):
        if d.get('status'): states.append(d['status'])
        if d.get('baseline_status'): states.append(d['baseline_status'])
if triage_present and triage.get('status'): states.append(triage['status'])
blocked=any('blocked' in s for s in states); notes=any('with_notes' in s for s in states)
if missing or errors or missing_markers: status,reason='outcome_record_blocked','safe_outcome_record_reference_missing'
elif blocked or notes: status,reason='outcome_record_ready_with_notes','safe_outcome_record_reference_ready_with_notes'
else: status,reason='outcome_record_ready','safe_outcome_record_reference_ready'
outcome_record_status={'status':status,'reason':reason,'missing_required_inputs':sorted(set(missing)),'missing_required_markers':sorted(missing_markers),'parse_errors':errors,'triage_artifact_present':triage_present,'operator_message_ru':'Сформирован governance outcome record packet в reference-only режиме.'}
outcome_record_scope={'scope_target':'actual_governance_review_outcome_recording','governance_artifact_type':'outcome_record_reference_packet','is_runtime_authorization':False,'is_execution_permit':False,'opens_implicit_runtime_transition':False,'replaces_future_runtime_phase':False,'scope_ru':'Пакет относится только к фиксации фактического governance review outcome record.','governance_reference_only_ru':'Пакет является governance/reference артефактом.'}
actual_review_outcome_record=['outcome_record_id','review_subject_ref','outcome_template_ref','review_packet_ref','review_runbook_ref','predecessor_ref','change_control_ref','maintenance_window_ref','marker_set','recorded_outcome_class','recorded_outcome_summary','continuity_findings','traceability_findings','operator_notes','generated_at']
outcome_record_constraints=['recorded outcome must match outcome template classes','recorded outcome must preserve non-execution boundaries','recorded outcome must not imply runtime readiness','recorded outcome must not imply runtime authorization','recorded outcome must preserve predecessor/change-control/maintenance linkage','recorded outcome must preserve continuity findings','recorded outcome must preserve traceability findings','no silent outcome rewrite','no hidden governance fork via record packet']
record_traceability_requirements=['template linkage traceable','review packet linkage traceable','runbook linkage traceable','predecessor linkage traceable','marker transitions traceable','recorded findings operator-visible','outcome summary operator-visible','archived governance chain preserved','no silent replacement of recorded outcome']
record_invariants=['outcome-record-only governance flow','record-keeping-only interpretation','no runtime activation','no runtime execution','no graph mutation','no remediation','no hidden side effects','no policy bypass','no baseline bypass','no approval bypass','no audit bypass','no outcome-record-to-runtime shortcut','no silent execution fallback']
validation_rules=['outcome_record_packet_has_required_sections','all_required_markers_present','actual_review_outcome_record_is_complete_and_consistent','outcome_record_constraints_are_complete_and_consistent','record_traceability_requirements_are_complete_and_consistent','execution_related_flags_absent','runtime_open_flags_absent','outcome_record_packet_is_compatible_with_design_control_only_state']
rejection_rules=['missing_required_sections','missing_required_markers','malformed_actual_review_outcome_record','malformed_outcome_record_constraints','malformed_traceability_requirements','stale_governance_review_outcome_refs','execution_related_flags_present','runtime_open_fields_detected','hidden_action_fields_detected','implicit_runtime_activation_fields_detected']
non_execution_confirmation={'execution_authorized':False,'graph_write_authorized':False,'remediation_authorized':False,'runtime_phase_open':False,'outcome_record_packet_is_not_runtime_activation_or_execution_permission':True}
recommended_next_phase_step={'phase':'phase36_11_governance_record_retention_packet_v1','goal_ru':'Подготовить политику retention и аудита record-пакетов без открытия runtime.','runtime_authorization_change':False}
marker=f"KV_PHASE36_GOVERNANCE_OUTCOME_RECORD_PACKET_V1|status={status}|reason={reason}"
payload={'version':'phase36_governance_outcome_record_packet_v1','generated_at':now,'status':status,'reason':reason,'marker':marker,'outcome_record_status':outcome_record_status,'outcome_record_scope':outcome_record_scope,'actual_review_outcome_record':actual_review_outcome_record,'outcome_record_constraints':outcome_record_constraints,'required_markers':markers,'record_traceability_requirements':record_traceability_requirements,'record_invariants':record_invariants,'validation_rules':validation_rules,'rejection_rules':rejection_rules,'non_execution_confirmation':non_execution_confirmation,'recommended_next_phase_step':recommended_next_phase_step}
out_json.write_text(json.dumps(payload,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
md=['# Фаза 36.10 — Governance Outcome Record Packet v1','',f'Сформировано: {now}','',f'Маркер: `{marker}`','',f'- Статус: **{status}**',f'- Причина: **{reason}**','- Пакет фиксирует канонический формат фактического review outcome без открытия runtime.','','## outcome_record_status']
for k,v in outcome_record_status.items(): md.append(f'- {k}: {v}')
for n,o in [('outcome_record_scope',outcome_record_scope),('required_markers',markers),('recommended_next_phase_step',recommended_next_phase_step)]:
    md.extend(['',f'## {n}'])
    for k,v in o.items(): md.append(f'- {k}: {v}')
for n,vals in [('actual_review_outcome_record',actual_review_outcome_record),('outcome_record_constraints',outcome_record_constraints),('record_traceability_requirements',record_traceability_requirements),('record_invariants',record_invariants),('validation_rules',validation_rules),('rejection_rules',rejection_rules)]:
    md.extend(['',f'## {n}'])
    for v in vals: md.append(f'- {v}')
md.extend(['','## non_execution_confirmation'])
for k,v in non_execution_confirmation.items(): md.append(f'- {k}: {v}')
out_md.write_text('\n'.join(md)+'\n',encoding='utf-8')
print('Готово: сформирован governance outcome record packet в режиме read-only/design-only.')
print('Runtime activation/execution, graph writes и remediation остаются закрытыми.')
print(f'Итоговый маркер: {marker}')
PY
