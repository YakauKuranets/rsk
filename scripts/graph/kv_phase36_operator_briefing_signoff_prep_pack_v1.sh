#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}" )/../.." && pwd)"
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
export DECISION_MEMO_JSON="${ROOT_DIR}/docs/phase34_validation_agent_gate_decision_memo_v1.json"
export RUNTIME_ENTRY_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_entry_contract_v1.json"
export RUNTIME_REQUEST_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_request_packet_v1.json"
export RUNTIME_REVIEW_JSON="${ROOT_DIR}/docs/phase34_validation_agent_runtime_review_response_v1.json"
export REVIEW_CYCLE_JSON="${ROOT_DIR}/docs/phase34_validation_agent_review_cycle_bundle_v1.json"
export POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
export BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
export HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
export PHASE36_HANDOFF_JSON="${ROOT_DIR}/docs/phase36_operator_handoff_governance_pack_v1.json"
export TRIAGE_JSON="${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json"
export OUT_JSON="${ROOT_DIR}/docs/phase36_operator_briefing_signoff_prep_pack_v1.json"
export OUT_MD="${ROOT_DIR}/docs/phase36_operator_briefing_signoff_prep_pack_v1.md"

python - <<'PY'
import json, os
from pathlib import Path

now=os.environ['NOW_UTC']
required={
'phase35_validation_agent_design_blueprint':Path(os.environ['BLUEPRINT_JSON']),
'phase35_validation_agent_layer_contracts':Path(os.environ['LAYER_CONTRACTS_JSON']),
'phase35_contract_slice_artifact_to_policy':Path(os.environ['SLICE_ARTIFACT_POLICY_JSON']),
'phase35_contract_slice_policy_to_dryrun':Path(os.environ['SLICE_POLICY_DRYRUN_JSON']),
'phase35_contract_slice_dryrun_to_approval':Path(os.environ['SLICE_DRYRUN_APPROVAL_JSON']),
'phase35_contract_slice_approval_to_audit':Path(os.environ['SLICE_APPROVAL_AUDIT_JSON']),
'phase35_contract_slice_audit_to_runtime_boundary':Path(os.environ['SLICE_AUDIT_BOUNDARY_JSON']),
'phase35_future_runtime_boundary_governance_bundle':Path(os.environ['BOUNDARY_GOV_JSON']),
'phase35_entry_pack':Path(os.environ['ENTRY_PACK_JSON']),
'phase34_validation_agent_approval_contract':Path(os.environ['APPROVAL_CONTRACT_JSON']),
'phase34_validation_agent_approval_record':Path(os.environ['APPROVAL_RECORD_JSON']),
'phase34_validation_agent_operator_gate':Path(os.environ['OPERATOR_GATE_JSON']),
'phase34_validation_agent_decision_memo':Path(os.environ['DECISION_MEMO_JSON']),
'phase34_validation_agent_runtime_entry_contract':Path(os.environ['RUNTIME_ENTRY_JSON']),
'phase34_validation_agent_runtime_request_packet':Path(os.environ['RUNTIME_REQUEST_JSON']),
'phase34_validation_agent_runtime_review_response':Path(os.environ['RUNTIME_REVIEW_JSON']),
'phase34_validation_agent_review_cycle_bundle':Path(os.environ['REVIEW_CYCLE_JSON']),
'phase33_operator_policy':Path(os.environ['POLICY_JSON']),
'phase33_baseline_freeze':Path(os.environ['BASELINE_JSON']),
'phase33_handoff_pack':Path(os.environ['HANDOFF_JSON']),
'phase36_operator_handoff_governance_pack':Path(os.environ['PHASE36_HANDOFF_JSON']),
}
triage_path=Path(os.environ['TRIAGE_JSON']); out_json=Path(os.environ['OUT_JSON']); out_md=Path(os.environ['OUT_MD'])
missing=[k for k,p in required.items() if not p.exists()]; errors=[]; triage_present=triage_path.exists()

def load(p): return json.loads(p.read_text())
docs={}
for k,p in required.items():
    if not p.exists(): docs[k]={}; continue
    try: docs[k]=load(p)
    except json.JSONDecodeError as exc: docs[k]={}; missing.append(k); errors.append(f"{k}:invalid_json:{exc.msg}")
triage={}
if triage_present:
    try: triage=load(triage_path)
    except json.JSONDecodeError as exc: errors.append(f"phase34_operator_backlog_triage:invalid_json:{exc.msg}")

required_markers={
 'phase35_blueprint_marker':docs['phase35_validation_agent_design_blueprint'].get('marker',''),
 'layer_contracts_marker':docs['phase35_validation_agent_layer_contracts'].get('marker',''),
 'contract_slice_artifact_to_policy_marker':docs['phase35_contract_slice_artifact_to_policy'].get('marker',''),
 'contract_slice_policy_to_dryrun_marker':docs['phase35_contract_slice_policy_to_dryrun'].get('marker',''),
 'contract_slice_dryrun_to_approval_marker':docs['phase35_contract_slice_dryrun_to_approval'].get('marker',''),
 'contract_slice_approval_to_audit_marker':docs['phase35_contract_slice_approval_to_audit'].get('marker',''),
 'contract_slice_audit_to_runtime_boundary_marker':docs['phase35_contract_slice_audit_to_runtime_boundary'].get('marker',''),
 'future_runtime_boundary_governance_bundle_marker':docs['phase35_future_runtime_boundary_governance_bundle'].get('marker',''),
 'operator_handoff_governance_pack_marker':docs['phase36_operator_handoff_governance_pack'].get('marker',''),
 'approval_contract_marker':docs['phase34_validation_agent_approval_contract'].get('marker',''),
 'approval_record_marker':docs['phase34_validation_agent_approval_record'].get('marker',''),
 'operator_gate_marker':docs['phase34_validation_agent_operator_gate'].get('marker',''),
 'decision_memo_marker':docs['phase34_validation_agent_decision_memo'].get('marker',''),
 'runtime_entry_contract_marker':docs['phase34_validation_agent_runtime_entry_contract'].get('marker',''),
 'runtime_request_packet_marker':docs['phase34_validation_agent_runtime_request_packet'].get('marker',''),
 'runtime_review_response_marker':docs['phase34_validation_agent_runtime_review_response'].get('marker',''),
 'review_cycle_bundle_marker':docs['phase34_validation_agent_review_cycle_bundle'].get('marker',''),
 'policy_marker':docs['phase33_operator_policy'].get('marker',''),
 'baseline_marker':docs['phase33_baseline_freeze'].get('marker',''),
 'handoff_marker':docs['phase33_handoff_pack'].get('marker',''),
}
if triage_present: required_markers['triage_marker_optional']=triage.get('marker','')
missing_markers=[k for k,v in required_markers.items() if (k!='triage_marker_optional' and not v)]
states=[d.get('status','') for d in docs.values()] + [docs['phase33_baseline_freeze'].get('baseline_status','')]
if triage_present: states.append(triage.get('status',''))
blocked=any(v=='blocked' or v.endswith('_blocked') for v in states if v)
notes=any('with_notes' in v for v in states if v)
if missing or errors or missing_markers: status,reason='briefing_pack_blocked','safe_briefing_reference_missing'
elif blocked or notes: status,reason='briefing_pack_ready_with_notes','safe_briefing_reference_ready_with_notes'
else: status,reason='briefing_pack_ready','safe_briefing_reference_ready'

briefing_pack_status={'status':status,'reason':reason,'missing_required_inputs':sorted(set(missing)),'missing_required_markers':missing_markers,'parse_errors':errors,'triage_artifact_present':triage_present,'operator_message_ru':'Сформирован operator briefing/signoff prep pack без runtime activation.'}
briefing_scope={'scope_target':'operator_briefing_signoff_preparation','governance_artifact_type':'briefing_signoff_reference_pack','is_runtime_authorization':False,'is_execution_permit':False,'opens_implicit_runtime_transition':False,'replaces_future_runtime_phase':False,'scope_ru':'Пакет задаёт рамку интерпретации readiness и guardrails без открытия runtime.'}
operator_readiness_interpretation={'ready_with_notes_interpretation_ru':'ready_with_notes означает допустимость governance-подготовки при наличии фиксированных notes; не runtime permission.','completeness_chain_interpretation_ru':'полная chain подтверждает связность артефактов, но не разрешение исполнения.','marker_completeness_interpretation_ru':'полный набор markers подтверждает трассируемость, но не активацию runtime.','non_execution_flags_interpretation_ru':'flags=false подтверждают жёсткое закрытие execution/runtime path.','governance_readiness_interpretation_ru':'готовность относится только к governance/reference состоянию.','not_runtime_permission_ru':'любая readiness в этой фазе не означает runtime permission.'}
signoff_prep_summary={'signoff_prep_readiness_definition_ru':'готовность signoff-prep = целостная chain + полные markers + подтверждённые guardrails + runtime_closed.','visible_boundary_guardrails_ru':['policy','baseline','approval','audit/evidence','runtime-boundary','operator-control','no-execution','no-graph-write','no-remediation'],'required_continuity_points_ru':['artifact->policy','policy->dryrun','dryrun->approval','approval->audit','audit->runtime-boundary','governance bundle'],'dependencies_to_confirm_ru':['approval contract/record','operator gate','decision memo','runtime entry/request/review refs','review cycle bundle'],'allowed_unresolved_notes_ru':'notes допускаются, если они явно задокументированы и не нарушают non-execution guardrails.','not_runtime_signoff_ru':'signoff prep не является runtime signoff.'}
guardrail_interpretation_rules={'policy_interpretation_rules':['policy marker обязателен','policy bypass запрещён'],'baseline_interpretation_rules':['baseline marker обязателен','baseline bypass запрещён'],'approval_interpretation_rules':['approval markers обязательны','approval не означает execution'],'audit_evidence_interpretation_rules':['audit chain должна быть согласована','evidence traceability обязательна'],'runtime_boundary_interpretation_rules':['runtime boundary refs обязательны','runtime остаётся закрытым'],'governance_bundle_interpretation_rules':['governance bundle обязателен','bundle не открывает runtime'],'operator_control_interpretation_rules':['operator gate/review cycle обязательны','контроль через sequence проверки'],'no_execution_interpretation_rules':['execution_authorized=false обязателен'],'no_graph_write_interpretation_rules':['graph_write_authorized=false обязателен'],'no_remediation_interpretation_rules':['remediation_authorized=false обязателен']}
operator_do_not_assume_rules=['do not assume runtime is open','do not assume approval implies execution','do not assume governance readiness implies activation','do not assume boundary completeness implies runtime permission','do not assume handoff completion implies execution signoff','do not assume hidden fallback path exists']
validation_rules=['briefing_pack_has_required_sections','all_required_markers_present','readiness_interpretation_is_complete_and_consistent','signoff_prep_summary_is_complete_and_consistent','operator_do_not_assume_rules_are_complete','execution_related_flags_absent','runtime_open_flags_absent','briefing_pack_is_compatible_with_design_control_only_state']
rejection_rules=['missing_required_sections','missing_required_markers','malformed_readiness_interpretation','malformed_signoff_prep_summary','malformed_guardrail_interpretation_rules','malformed_do_not_assume_rules','stale_governance_or_handoff_refs','execution_related_flags_present','runtime_open_fields_detected','hidden_action_fields_detected','implicit_runtime_activation_fields_detected']
recommended_next_phase_step={'phase':'phase36_3_operator_signoff_packet_v1','goal_ru':'Подготовить финальный signoff packet в design/control режиме без runtime activation.','runtime_authorization_change':False}
marker=f"KV_PHASE36_OPERATOR_BRIEFING_SIGNOFF_PREP_PACK_V1|status={status}|reason={reason}"
payload={'version':'phase36_operator_briefing_signoff_prep_pack_v1','generated_at':now,'status':status,'reason':reason,'marker':marker,
'briefing_pack_status':briefing_pack_status,'briefing_scope':briefing_scope,'operator_readiness_interpretation':operator_readiness_interpretation,'signoff_prep_summary':signoff_prep_summary,'required_markers':required_markers,'guardrail_interpretation_rules':guardrail_interpretation_rules,'operator_do_not_assume_rules':operator_do_not_assume_rules,'validation_rules':validation_rules,'rejection_rules':rejection_rules,'non_execution_confirmation':{'execution_authorized':False,'graph_write_authorized':False,'remediation_authorized':False,'runtime_phase_open':False,'briefing_pack_is_not_runtime_activation_or_execution_permission':True},'recommended_next_phase_step':recommended_next_phase_step}
out_json.write_text(json.dumps(payload,ensure_ascii=False,indent=2)+'\n')

md=['# Фаза 36.2 — Operator Briefing / Signoff Prep Pack v1','',f'Сформировано: {now}','',f'Маркер: `{marker}`','',f'- Статус: **{status}**',f'- Причина: **{reason}**','- Документ только для operator briefing/signoff prep в design/control контуре.','- Runtime activation/execution остаются запрещены.','', '## briefing_pack_status']
for k,v in briefing_pack_status.items(): md.append(f'- {k}: {v}')
for name,section in [('briefing_scope',briefing_scope),('operator_readiness_interpretation',operator_readiness_interpretation),('signoff_prep_summary',signoff_prep_summary),('required_markers',required_markers),('guardrail_interpretation_rules',guardrail_interpretation_rules),('recommended_next_phase_step',recommended_next_phase_step)]:
 md += ['',f'## {name}']
 for k,v in section.items(): md.append(f'- {k}: {v}')
md += ['', '## operator_do_not_assume_rules']
for r in operator_do_not_assume_rules: md.append(f'- {r}')
for name,vals in [('validation_rules',validation_rules),('rejection_rules',rejection_rules)]:
 md += ['',f'## {name}']
 for v in vals: md.append(f'- {v}')
md += ['', '## non_execution_confirmation']
for k,v in payload['non_execution_confirmation'].items(): md.append(f'- {k}: {v}')
out_md.write_text('\n'.join(md)+'\n')

print('Готово: сформирован operator briefing/signoff prep pack в read-only режиме.')
print('Исполнение, запись в граф и remediation остаются запрещены.')
print(f'Маркер: {marker}')
PY
