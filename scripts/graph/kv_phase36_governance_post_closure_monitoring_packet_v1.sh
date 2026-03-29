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
TRIAGE_JSON=phase34_operator_backlog_triage_v1.json \
OUT_JSON=phase36_governance_post_closure_monitoring_packet_v1.json \
OUT_MD=phase36_governance_post_closure_monitoring_packet_v1.md
 do
  k="${v%%=*}"; f="${v#*=}"; export "$k=${ROOT_DIR}/docs/$f"
done
python - <<'PY'
import json, os
from pathlib import Path

def load(p,k,docs,miss,errs):
    if not p.exists(): docs[k]={}; miss.append(k); return
    try: docs[k]=json.loads(p.read_text(encoding='utf-8'))
    except json.JSONDecodeError as e: docs[k]={}; errs.append(f"{k}:invalid_json:{e.msg}")

now=os.environ['NOW_UTC']; outj=Path(os.environ['OUT_JSON']); outm=Path(os.environ['OUT_MD']); triage=Path(os.environ['TRIAGE_JSON'])
dec=Path(os.environ['DECISION_MEMO_JSON'])
if not dec.exists(): dec=Path(os.environ['DECISION_MEMO_FALLBACK_JSON'])
req={
'phase35_validation_agent_design_blueprint':Path(os.environ['BLUEPRINT_JSON']),'phase35_validation_agent_layer_contracts':Path(os.environ['LAYER_CONTRACTS_JSON']),'phase35_contract_slice_artifact_to_policy':Path(os.environ['SLICE_ARTIFACT_POLICY_JSON']),'phase35_contract_slice_policy_to_dryrun':Path(os.environ['SLICE_POLICY_DRYRUN_JSON']),'phase35_contract_slice_dryrun_to_approval':Path(os.environ['SLICE_DRYRUN_APPROVAL_JSON']),'phase35_contract_slice_approval_to_audit':Path(os.environ['SLICE_APPROVAL_AUDIT_JSON']),'phase35_contract_slice_audit_to_runtime_boundary':Path(os.environ['SLICE_AUDIT_BOUNDARY_JSON']),'phase35_future_runtime_boundary_governance_bundle':Path(os.environ['BOUNDARY_GOV_JSON']),'phase35_entry_pack':Path(os.environ['ENTRY_PACK_JSON']),'phase34_validation_agent_approval_contract':Path(os.environ['APPROVAL_CONTRACT_JSON']),'phase34_validation_agent_approval_record':Path(os.environ['APPROVAL_RECORD_JSON']),'phase34_validation_agent_operator_gate':Path(os.environ['OPERATOR_GATE_JSON']),'phase34_validation_agent_decision_memo':dec,'phase34_validation_agent_runtime_entry_contract':Path(os.environ['RUNTIME_ENTRY_JSON']),'phase34_validation_agent_runtime_request_packet':Path(os.environ['RUNTIME_REQUEST_JSON']),'phase34_validation_agent_runtime_review_response':Path(os.environ['RUNTIME_REVIEW_JSON']),'phase34_validation_agent_review_cycle_bundle':Path(os.environ['REVIEW_CYCLE_JSON']),'phase33_operator_policy':Path(os.environ['POLICY_JSON']),'phase33_baseline_freeze':Path(os.environ['BASELINE_JSON']),'phase33_handoff_pack':Path(os.environ['HANDOFF_JSON']),'phase36_operator_handoff_governance_pack':Path(os.environ['HANDOFF_PACK_JSON']),'phase36_operator_briefing_signoff_prep_pack':Path(os.environ['BRIEFING_PACK_JSON']),'phase36_final_operator_signoff_packet':Path(os.environ['FINAL_SIGNOFF_JSON']),'phase36_governance_archive_change_control_packet':Path(os.environ['ARCHIVE_PACKET_JSON']),'phase36_governance_maintenance_window_packet':Path(os.environ['MAINTENANCE_PACKET_JSON']),'phase36_versioned_governance_successor_template_packet':Path(os.environ['SUCCESSOR_TEMPLATE_JSON']),'phase36_governance_successor_review_packet':Path(os.environ['SUCCESSOR_REVIEW_JSON']),'phase36_successor_review_checklist_runbook_packet':Path(os.environ['RUNBOOK_JSON']),'phase36_successor_review_outcome_template_packet':Path(os.environ['OUTCOME_TEMPLATE_JSON']),'phase36_governance_outcome_record_packet':Path(os.environ['OUTCOME_RECORD_JSON']),'phase36_governance_record_retention_packet':Path(os.environ['RETENTION_PACKET_JSON']),'phase36_governance_retention_audit_log_packet':Path(os.environ['RETENTION_AUDIT_LOG_JSON']),'phase36_governance_closure_memo_packet':Path(os.environ['CLOSURE_MEMO_JSON'])
}
docs={}; miss=[]; errs=[]
for k,p in req.items(): load(p,k,docs,miss,errs)
triage_doc={}; triage_present=triage.exists()
if triage_present:
    try: triage_doc=json.loads(triage.read_text(encoding='utf-8'))
    except json.JSONDecodeError as e: errs.append(f"phase34_operator_backlog_triage:invalid_json:{e.msg}")
markers={
'phase35_blueprint_marker':docs['phase35_validation_agent_design_blueprint'].get('marker',''),'layer_contracts_marker':docs['phase35_validation_agent_layer_contracts'].get('marker',''),'contract_slice_artifact_to_policy_marker':docs['phase35_contract_slice_artifact_to_policy'].get('marker',''),'contract_slice_policy_to_dryrun_marker':docs['phase35_contract_slice_policy_to_dryrun'].get('marker',''),'contract_slice_dryrun_to_approval_marker':docs['phase35_contract_slice_dryrun_to_approval'].get('marker',''),'contract_slice_approval_to_audit_marker':docs['phase35_contract_slice_approval_to_audit'].get('marker',''),'contract_slice_audit_to_runtime_boundary_marker':docs['phase35_contract_slice_audit_to_runtime_boundary'].get('marker',''),'future_runtime_boundary_governance_bundle_marker':docs['phase35_future_runtime_boundary_governance_bundle'].get('marker',''),'operator_handoff_governance_pack_marker':docs['phase36_operator_handoff_governance_pack'].get('marker',''),'operator_briefing_signoff_prep_pack_marker':docs['phase36_operator_briefing_signoff_prep_pack'].get('marker',''),'final_operator_signoff_packet_marker':docs['phase36_final_operator_signoff_packet'].get('marker',''),'governance_archive_change_control_packet_marker':docs['phase36_governance_archive_change_control_packet'].get('marker',''),'governance_maintenance_window_packet_marker':docs['phase36_governance_maintenance_window_packet'].get('marker',''),'versioned_governance_successor_template_packet_marker':docs['phase36_versioned_governance_successor_template_packet'].get('marker',''),'governance_successor_review_packet_marker':docs['phase36_governance_successor_review_packet'].get('marker',''),'successor_review_checklist_runbook_packet_marker':docs['phase36_successor_review_checklist_runbook_packet'].get('marker',''),'successor_review_outcome_template_packet_marker':docs['phase36_successor_review_outcome_template_packet'].get('marker',''),'governance_outcome_record_packet_marker':docs['phase36_governance_outcome_record_packet'].get('marker',''),'governance_record_retention_packet_marker':docs['phase36_governance_record_retention_packet'].get('marker',''),'governance_retention_audit_log_packet_marker':docs['phase36_governance_retention_audit_log_packet'].get('marker',''),'governance_closure_memo_packet_marker':docs['phase36_governance_closure_memo_packet'].get('marker',''),'approval_contract_marker':docs['phase34_validation_agent_approval_contract'].get('marker',''),'approval_record_marker':docs['phase34_validation_agent_approval_record'].get('marker',''),'operator_gate_marker':docs['phase34_validation_agent_operator_gate'].get('marker',''),'decision_memo_marker':docs['phase34_validation_agent_decision_memo'].get('marker',''),'runtime_entry_contract_marker':docs['phase34_validation_agent_runtime_entry_contract'].get('marker',''),'runtime_request_packet_marker':docs['phase34_validation_agent_runtime_request_packet'].get('marker',''),'runtime_review_response_marker':docs['phase34_validation_agent_runtime_review_response'].get('marker',''),'review_cycle_bundle_marker':docs['phase34_validation_agent_review_cycle_bundle'].get('marker',''),'policy_marker':docs['phase33_operator_policy'].get('marker',''),'baseline_marker':docs['phase33_baseline_freeze'].get('marker',''),'handoff_marker':docs['phase33_handoff_pack'].get('marker','')}
if triage_present: markers['triage_marker']=triage_doc.get('marker','')
missm=[k for k,v in markers.items() if not v]
st=[]
for d in docs.values():
    if isinstance(d,dict):
        if d.get('status'): st.append(d['status'])
        if d.get('baseline_status'): st.append(d['baseline_status'])
if triage_present and triage_doc.get('status'): st.append(triage_doc['status'])
blocked=any('blocked' in s for s in st); notes=any('with_notes' in s for s in st)
if miss or errs or missm: status,reason='post_closure_monitoring_blocked','safe_post_closure_monitoring_reference_missing'
elif blocked or notes: status,reason='post_closure_monitoring_ready_with_notes','safe_post_closure_monitoring_reference_ready_with_notes'
else: status,reason='post_closure_monitoring_ready','safe_post_closure_monitoring_reference_ready'
post_closure_monitoring_status={'status':status,'reason':reason,'missing_required_inputs':sorted(set(miss)),'missing_required_markers':sorted(missm),'parse_errors':errs,'triage_artifact_present':triage_present,'operator_message_ru':'Сформирован governance post-closure monitoring packet в reference-only режиме.'}
post_closure_monitoring_scope={'scope_target':'monitoring_of_closed_governance_chain_after_closure_memo','governance_artifact_type':'post_closure_monitoring_reference_packet','is_runtime_authorization':False,'is_execution_permit':False,'opens_implicit_runtime_transition':False,'replaces_future_runtime_phase':False,'scope_ru':'Пакет относится только к monitoring закрытого governance-контура после closure memo.','governance_reference_only_ru':'Пакет является governance/reference артефактом.'}
monitoring_targets=['closure memo consistency','required markers continuity','retained record continuity','retention audit trail continuity','successor governance chain stability','no drift in non-execution boundaries','no reinterpretation of governance closure as runtime readiness']
monitoring_signal_rules=['marker drift detection','closure-summary inconsistency detection','retention lineage break detection','post-closure semantics drift detection','non-execution flag drift detection','notes escalation if governance-only meaning weakens','no monitoring signal may imply runtime authorization','no monitoring signal may open runtime path']
monitoring_traceability_rules=['monitoring events traceable to closure state','marker transitions traceable','retained chain traceable','retention audit lineage traceable','operator-visible monitoring summary required','no silent monitoring state replacement','no hidden governance fork after closure','no implicit runtime meaning from monitoring outputs']
monitoring_invariants=['post-closure-monitoring-only governance flow','monitoring-only interpretation','no runtime activation','no runtime execution','no graph mutation','no remediation','no hidden side effects','no policy bypass','no baseline bypass','no approval bypass','no audit bypass','no monitoring-to-runtime shortcut','no silent execution fallback']
validation_rules=['monitoring_packet_has_required_sections','all_required_markers_present','monitoring_targets_are_complete_and_consistent','monitoring_signal_rules_are_complete_and_consistent','monitoring_traceability_rules_are_complete_and_consistent','execution_related_flags_absent','runtime_open_flags_absent','monitoring_packet_is_compatible_with_design_control_only_state']
rejection_rules=['missing_required_sections','missing_required_markers','malformed_monitoring_targets','malformed_monitoring_signal_rules','malformed_monitoring_traceability_rules','stale_closure_retention_refs','execution_related_flags_present','runtime_open_fields_detected','hidden_action_fields_detected','implicit_runtime_activation_fields_detected']
non_execution_confirmation={'execution_authorized':False,'graph_write_authorized':False,'remediation_authorized':False,'runtime_phase_open':False,'post_closure_monitoring_packet_is_not_runtime_activation_or_execution_permission':True}
recommended_next_phase_step={'phase':'phase36_15_governance_monitoring_handoff_packet_v1','goal_ru':'Подготовить governance handoff по post-closure monitoring без открытия runtime.','runtime_authorization_change':False}
marker=f"KV_PHASE36_GOVERNANCE_POST_CLOSURE_MONITORING_PACKET_V1|status={status}|reason={reason}"
payload={'version':'phase36_governance_post_closure_monitoring_packet_v1','generated_at':now,'status':status,'reason':reason,'marker':marker,'post_closure_monitoring_status':post_closure_monitoring_status,'post_closure_monitoring_scope':post_closure_monitoring_scope,'monitoring_targets':monitoring_targets,'monitoring_signal_rules':monitoring_signal_rules,'required_markers':markers,'monitoring_traceability_rules':monitoring_traceability_rules,'monitoring_invariants':monitoring_invariants,'validation_rules':validation_rules,'rejection_rules':rejection_rules,'non_execution_confirmation':non_execution_confirmation,'recommended_next_phase_step':recommended_next_phase_step}
outj.write_text(json.dumps(payload,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
md=['# Фаза 36.14 — Пакет governance post-closure monitoring (v1)','',f'Сформировано: {now}','',f'Маркер: `{marker}`','',f'- Статус: **{status}**',f'- Причина: **{reason}**','- Пакет задаёт monitoring-only рамку после closure memo без открытия runtime.','','## Статус post-closure monitoring (`post_closure_monitoring_status`)']
for k,v in post_closure_monitoring_status.items(): md.append(f'- `{k}`: {v}')
for title,obj in [('Область monitoring-пакета (`post_closure_monitoring_scope`)',post_closure_monitoring_scope),('Обязательные маркеры (`required_markers`)',markers),('Рекомендуемый следующий шаг (`recommended_next_phase_step`)',recommended_next_phase_step)]:
    md.extend(['',f'## {title}'])
    for k,v in obj.items(): md.append(f'- `{k}`: {v}')
for title,vals in [('Цели monitoring (`monitoring_targets`)',monitoring_targets),('Правила monitoring-сигналов (`monitoring_signal_rules`)',monitoring_signal_rules),('Правила traceability monitoring (`monitoring_traceability_rules`)',monitoring_traceability_rules),('Инварианты monitoring (`monitoring_invariants`)',monitoring_invariants),('Правила валидации (`validation_rules`)',validation_rules),('Правила отклонения (`rejection_rules`)',rejection_rules)]:
    md.extend(['',f'## {title}'])
    for v in vals: md.append(f'- `{v}`')
md.extend(['','## Подтверждение non-execution (`non_execution_confirmation`)'])
for k,v in non_execution_confirmation.items(): md.append(f'- `{k}`: {v}')
outm.write_text('\n'.join(md)+'\n',encoding='utf-8')
print('Готово: сформирован governance post-closure monitoring packet в режиме read-only/design-only.')
print('Runtime activation/execution, graph writes и remediation остаются закрытыми.')
print(f'Итоговый маркер: {marker}')
PY
