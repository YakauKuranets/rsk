import { useEffect, useState } from 'react';
import TargetCard from './TargetCard';
import LabelPanel from './LabelPanel';
import IntelHub from '../intelligence/IntelligenceHub';
import PlaybookRunner from '../playbook/PlaybookRunner';
import CampaignList from '../campaign/CampaignList';
import CampaignDashboard from '../campaign/CampaignDashboard';
import PassiveScanner from '../passive-scan/PassiveScanner';
import CameraScanPanel from '../scan/CameraScanPanel';
import MassAudit from '../mass-audit/MassAudit';
import RuntimeLogs from '../logs/RuntimeLogs';
import RelayPanel from '../relay/RelayPanel';
import AgentReport from '../agents/AgentReport';
import HubReconPanel from '../archive/HubReconPanel';
import CapturePanel from '../archive/CapturePanel';
import NvrProbePanel from '../archive/NvrProbePanel';
import SpiderControl from '../spider/SpiderControl';
import { canRunArchiveExport, canRunStreamVerification } from '../targets/cardKindAdapter';
import { normalizeTargetForLinkedAction } from '../targets/targetActionNormalizer';

const T={
  bg0:'#07070f',bg1:'#0c0c1a',bg2:'#111122',bg3:'#171730',
  line:'#1e1e35',dim:'#444466',muted:'#6666aa',text:'#c0c0e0',
  red:'#ff3355',cyan:'#00ccff',grn:'#00dd88',amb:'#ffaa00',purp:'#9966ff',blue:'#4488ff',
};

const TABS=[
  {id:'targets',icon:'📍',label:'Цели',   color:T.cyan},
  {id:'ops',    icon:'⚡',label:'Операции',color:T.purp},
  {id:'intel',  icon:'🧠',label:'Разведка',color:T.red},
  {id:'system', icon:'⚙', label:'Система', color:T.muted},
];

const css={
  panel:{width:'400px',background:T.bg0,borderLeft:'1px solid '+T.line,display:'flex',flexDirection:'column',height:'100%',overflow:'hidden',flexShrink:0,fontFamily:"'Inter','Segoe UI',system-ui,sans-serif"},
  tabBar:{display:'flex',borderBottom:'1px solid '+T.line,flexShrink:0,background:T.bg1},
  tab:(a,c)=>({flex:1,padding:'8px 4px',fontSize:'10px',fontWeight:a?700:400,textAlign:'center',cursor:'pointer',border:'none',borderBottom:a?'2px solid '+c:'2px solid transparent',background:a?c+'15':'transparent',color:a?c:T.muted,transition:'all .15s',letterSpacing:'.03em',fontFamily:'inherit'}),
  scroll:{flex:1,overflowY:'auto',padding:'10px'},
  sec:{background:T.bg2,border:'1px solid '+T.line,borderRadius:'6px',marginBottom:'8px',overflow:'hidden'},
  sHead:{display:'flex',alignItems:'center',gap:'7px',padding:'8px 12px',borderBottom:'1px solid '+T.line,background:T.bg3,cursor:'pointer',userSelect:'none'},
  sTitle:(c)=>({fontSize:'11px',fontWeight:700,color:c||T.text,flex:1,letterSpacing:'.05em',textTransform:'uppercase'}),
  sBody:{padding:'10px 12px'},
  input:{width:'100%',padding:'6px 9px',background:T.bg0,color:T.text,border:'1px solid '+T.line,borderRadius:'4px',fontSize:'12px',marginBottom:'6px',boxSizing:'border-box',fontFamily:'inherit',outline:'none'},
  btn:(c,fill=false)=>({padding:'6px 12px',background:fill?c:c+'18',color:fill?'#000':c,border:'1px solid '+c+'55',borderRadius:'4px',fontSize:'11px',fontWeight:600,cursor:'pointer',fontFamily:'inherit',transition:'all .12s'}),
  btnFull:(c,fill=false)=>({width:'100%',padding:'7px',background:fill?c:c+'18',color:fill?'#000':c,border:'1px solid '+c+'55',borderRadius:'4px',fontSize:'12px',fontWeight:700,cursor:'pointer',marginBottom:'6px',fontFamily:'inherit'}),
  row:{display:'flex',gap:'6px',marginBottom:'6px'},
};

function Section({icon,title,color,children,defaultOpen=true}){
  const [open,setOpen]=useState(defaultOpen);
  return(
    <div style={css.sec}>
      <div style={css.sHead} onClick={()=>setOpen(v=>!v)}>
        <span style={{fontSize:'14px'}}>{icon}</span>
        <span style={css.sTitle(color)}>{title}</span>
        <span style={{color:T.dim,fontSize:'10px'}}>{open?'▲':'▼'}</span>
      </div>
      {open&&<div style={css.sBody}>{children}</div>}
    </div>
  );
}

const POLICY_SIGNAL_LABELS = [
  { marker: 'TARGET_ENVELOPE_POLICY_WARNING', label: 'Предупреждение по политике', short: 'policy warning' },
  { marker: 'TARGET_ENVELOPE_POLICY_ESCALATION_WARNING', label: 'Усиление предупреждений политики', short: 'policy escalation' },
  { marker: 'TARGET_ENVELOPE_PRE_STRICTNESS_WARNING', label: 'Сигнал перед ужесточением', short: 'pre-strictness warning' },
  { marker: 'TARGET_ENVELOPE_STRICT_REJECT', label: 'Строгий отказ сохранения', short: 'strict reject' },
];

function getRuntimeLogMessage(line) {
  if (line == null) return '';
  if (typeof line === 'string') return line;
  if (typeof line === 'object') return String(line.message || '');
  return String(line);
}

function PolicyRuntimeStatusBlock({ runtimeLogs }) {
  const safeLogs = runtimeLogs || [];
  const recentWindow = safeLogs.slice(-80);
  const stats = POLICY_SIGNAL_LABELS.map(({ marker, label, short }) => {
    const hits = recentWindow.filter((line) => getRuntimeLogMessage(line).includes(marker));
    return { marker, label, short, count: hits.length, last: hits[hits.length - 1] || null };
  });
  const strictRejectCount = stats.find((s) => s.marker === 'TARGET_ENVELOPE_STRICT_REJECT')?.count || 0;
  const warningCount = stats.find((s) => s.marker === 'TARGET_ENVELOPE_POLICY_WARNING')?.count || 0;
  const escalationCount = stats.find((s) => s.marker === 'TARGET_ENVELOPE_POLICY_ESCALATION_WARNING')?.count || 0;
  const preStrictnessCount = stats.find((s) => s.marker === 'TARGET_ENVELOPE_PRE_STRICTNESS_WARNING')?.count || 0;

  let statusText = 'Спокойно';
  let statusColor = T.grn;
  if (strictRejectCount > 0) {
    statusText = 'Был строгий отказ';
    statusColor = T.red;
  } else if (escalationCount >= 2 || preStrictnessCount >= 2 || warningCount >= 4) {
    statusText = 'Есть устойчивый плохой тренд';
    statusColor = '#ff8844';
  } else if (warningCount > 0 || escalationCount > 0 || preStrictnessCount > 0) {
    statusText = 'Есть предупреждение';
    statusColor = T.amb;
  }

  const recentSignals = [];
  for (let i = safeLogs.length - 1; i >= 0; i -= 1) {
    const line = safeLogs[i];
    const message = getRuntimeLogMessage(line);
    const match = POLICY_SIGNAL_LABELS.find(({ marker }) => message.includes(marker));
    if (!match) continue;
    recentSignals.push({
      label: match.label,
      short: match.short,
      time: typeof line === 'object' ? (line.time || '??:??:??') : '??:??:??',
    });
    if (recentSignals.length >= 4) break;
  }

  return (
    <div style={{
      marginBottom: '8px',
      borderRadius: '6px',
      border: '1px solid ' + statusColor + '55',
      background: statusColor + '12',
      padding: '7px 8px',
    }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px', marginBottom: '4px' }}>
        <span style={{ fontSize: '10px', color: T.muted }}>Сигналы политики и системы</span>
        <span style={{ fontSize: '10px', color: statusColor, fontWeight: 700 }}>{statusText}</span>
      </div>
      <div style={{ fontSize: '10px', color: T.text, marginBottom: recentSignals.length ? '6px' : '0px' }}>
        Обзор: последние {recentWindow.length} записей · предупреждения: {warningCount} · усиления: {escalationCount} · pre-strict: {preStrictnessCount} · отказы: {strictRejectCount}
      </div>
      {recentSignals.length > 0 && (
        <div style={{ display: 'grid', gap: '3px' }}>
          {recentSignals.map((item, idx) => (
            <div key={`${item.short}_${item.time}_${idx}`} style={{ fontSize: '10px', color: T.muted }}>
              [{item.time}] {item.label}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function buildPolicyRuntimeSummary(runtimeLogs) {
  const safeLogs = runtimeLogs || [];
  const recentWindow = safeLogs.slice(-80);
  const getCount = (marker) => recentWindow.filter((line) => getRuntimeLogMessage(line).includes(marker)).length;
  const warningCount = getCount('TARGET_ENVELOPE_POLICY_WARNING');
  const escalationCount = getCount('TARGET_ENVELOPE_POLICY_ESCALATION_WARNING');
  const preStrictnessCount = getCount('TARGET_ENVELOPE_PRE_STRICTNESS_WARNING');
  const strictRejectCount = getCount('TARGET_ENVELOPE_STRICT_REJECT');

  let mode = 'calm';
  let text = 'Спокойно';
  if (strictRejectCount > 0) {
    mode = 'reject';
    text = 'Строгий отказ';
  } else if (warningCount > 0 || escalationCount > 0 || preStrictnessCount > 0) {
    mode = 'warning';
    text = 'Предупреждение';
  }

  return { mode, text, warningCount, escalationCount, preStrictnessCount, strictRejectCount };
}

function UnifiedTargetStatusBlock({ targetSaveStatus, sessionAuditStatus, runtimeLogs, form, activeTargetId, selectedTarget }) {
  const formTarget = [form?.name, form?.host].map((x) => String(x || '').trim()).filter(Boolean).join(' / ');
  const selectedTargetLabel = selectedTarget
    ? [selectedTarget?.name, selectedTarget?.host].map((x) => String(x || '').trim()).filter(Boolean).join(' / ')
    : '';
  const targetContext = selectedTargetLabel
    ? `Выбранная цель: ${selectedTargetLabel}`
    : activeTargetId
    ? `Последняя активная цель: ${activeTargetId}`
    : formTarget
    ? `Черновик цели: ${formTarget}`
      : targetSaveStatus?.text
        ? 'Последняя операция сохранения (цель определена по сохранению)'
        : 'Цель ещё не выбрана';
  const saveText = !targetSaveStatus
    ? 'Данных пока нет'
    : targetSaveStatus.level === 'error'
      ? 'Ошибка'
      : targetSaveStatus.level === 'warn'
        ? 'Сохранено с мягкой обработкой'
        : 'Сохранено';
  const saveColor = !targetSaveStatus
    ? T.dim
    : targetSaveStatus.level === 'error'
      ? T.red
      : targetSaveStatus.level === 'warn'
        ? T.amb
        : T.grn;

  const sessionText = !sessionAuditStatus
    ? 'Аудит ещё не запускался'
    : sessionAuditStatus.mode === 'inconclusive'
      ? 'Неопределённо'
      : sessionAuditStatus.mode === 'fallback'
        ? 'Резервный путь'
        : 'Основной путь';
  const sessionColor = !sessionAuditStatus
    ? T.dim
    : sessionAuditStatus.mode === 'inconclusive'
      ? T.amb
      : sessionAuditStatus.mode === 'fallback'
        ? '#ff8844'
        : T.grn;

  const policy = buildPolicyRuntimeSummary(runtimeLogs);
  const policyColor = policy.mode === 'reject' ? T.red : policy.mode === 'warning' ? T.amb : T.grn;

  return (
    <div style={{ marginTop: '6px', border: '1px solid #24303f', background: '#0a1018', borderRadius: '4px', padding: '6px 8px' }}>
      <div style={{ fontSize: '10px', color: '#7f93a4', marginBottom: '4px' }}>Сводный статус цели и системы</div>
      <div style={{ fontSize: '10px', color: T.dim, marginBottom: '6px' }}>{targetContext}</div>
      <div style={{ fontSize: '10px', color: '#7f93a4', marginBottom: '3px' }}>Состояние по выбранной цели</div>
      <div style={{ display: 'grid', gap: '3px', marginBottom: '6px' }}>
        <div style={{ fontSize: '10px', color: T.muted }}>Сохранение: <b style={{ color: saveColor }}>{saveText}</b></div>
        <div style={{ fontSize: '10px', color: T.muted }}>Сессия: <b style={{ color: sessionColor }}>{sessionText}</b></div>
      </div>
      <div style={{ fontSize: '10px', color: '#7f93a4', marginBottom: '3px' }}>Системный контекст</div>
      <div style={{ display: 'grid', gap: '3px' }}>
        <div style={{ fontSize: '10px', color: T.muted }}>Политика и runtime: <b style={{ color: policyColor }}>{policy.text}</b></div>
      </div>
    </div>
  );
}

function getSelectedTargetActionAvailability(target) {
  const isHub = String(target?.type || '').toLowerCase() === 'hub';
  const canStream = !isHub && canRunStreamVerification(target);
  const canArchive = isHub ? true : canRunArchiveExport(target);
  return {
    stream: canStream,
    archive: canArchive,
    isapi: canStream,
    onvif: canStream,
  };
}

function hasWebEndpoint(target) {
  const value = String(target?.url || target?.endpoint || '').trim();
  if (!value) return false;
  return /^https?:\/\//i.test(value);
}

function hasLikelyWebTargetInput(target) {
  const raw = `${target?.url || ''} ${target?.endpoint || ''} ${target?.host || ''}`.toLowerCase();
  if (!raw.trim()) return false;
  return /https?:\/\/|:80\b|:443\b|\bwww\./.test(raw);
}

function buildLinkedActionStatuses(target, availability) {
  const isHub = String(target?.type || '').toLowerCase() === 'hub';
  const streamTarget = normalizeTargetForLinkedAction(target, 'stream');
  const webIsapiTarget = normalizeTargetForLinkedAction(target, 'isapi_info');
  const webOnvifTarget = normalizeTargetForLinkedAction(target, 'onvif_info');
  const archiveSearchTarget = normalizeTargetForLinkedAction(target, 'archive_search');
  const archiveEndpointTarget = normalizeTargetForLinkedAction(target, isHub ? 'hub_archive' : 'archive_endpoints');
  const hasHost = (t) => String(t?.host || '').trim().length > 0;
  const hasWebHint = hasLikelyWebTargetInput(target) || hasWebEndpoint(target);

  return {
    stream: isHub
      ? 'Ограничено для HUB'
      : !hasHost(streamTarget)
        ? 'Нет host/ip'
        : availability?.stream
          ? 'Готово'
          : 'Недостаточно данных для запуска',
    isapi: !hasHost(webIsapiTarget)
      ? 'Нет host/ip'
      : !hasWebHint
        ? 'Нужен web-endpoint'
        : availability?.isapi
          ? 'Готово'
          : 'Недостаточно данных для запуска',
    onvif: !hasHost(webOnvifTarget)
      ? 'Нет host/ip'
      : !hasWebHint
        ? 'Нужен web-endpoint'
        : availability?.onvif
          ? 'Готово'
          : 'Недостаточно данных для запуска',
    archiveSearch: !hasHost(archiveSearchTarget)
      ? 'Нет host/ip'
      : !hasWebHint
        ? 'Нужен web-endpoint'
        : 'Готово',
    archive: !hasHost(archiveEndpointTarget)
      ? 'Нет host/ip'
      : isHub
        ? 'Готово'
        : availability?.archive
          ? 'Готово'
          : 'Недостаточно данных для запуска',
  };
}

function buildLinkedActionAggregate(statuses) {
  const entries = Object.entries(statuses || {});
  const labels = {
    stream: 'Поток',
    isapi: 'ISAPI',
    onvif: 'ONVIF',
    archiveSearch: 'Поиск архива',
    archive: 'Архив',
  };
  const values = entries.map(([, status]) => status).filter(Boolean);
  const readyCount = values.filter((s) => s === 'Готово').length;
  const limitedCount = values.filter((s) => s === 'Ограничено для HUB').length;
  const blockedCount = values.length - readyCount - limitedCount;
  const priorityReasons = ['Нет host/ip', 'Нужен web-endpoint', 'Недостаточно данных для запуска', 'Ограничено для HUB'];
  const mainReason = priorityReasons.find((reason) => values.includes(reason)) || (readyCount > 0 ? 'Готово' : 'Нет данных');
  const readyActions = entries.filter(([, status]) => status === 'Готово').map(([key]) => labels[key] || key);
  const limitedActions = entries.filter(([, status]) => status === 'Ограничено для HUB').map(([key]) => labels[key] || key);
  const blockedActions = entries
    .filter(([, status]) => status && status !== 'Готово' && status !== 'Ограничено для HUB')
    .map(([key]) => labels[key] || key);
  return { readyCount, limitedCount, blockedCount, mainReason, total: values.length, readyActions, limitedActions, blockedActions };
}

function buildTargetCompatibilityProfile(target, availability, actionStatuses) {
  const safeTarget = target || {};
  const typeText = String(safeTarget?.type || '').toLowerCase();
  const hostText = String(safeTarget?.host || safeTarget?.ip || '').toLowerCase();
  const endpointText = `${safeTarget?.url || ''} ${safeTarget?.endpoint || ''}`.toLowerCase();
  const nameText = String(safeTarget?.name || '').toLowerCase();
  const text = `${nameText} ${hostText} ${endpointText}`.toLowerCase();
  const isHub = typeText === 'hub';
  const looksCamera = /(cam|camera|nvr|dvr|rtsp|onvif|554|hik|xmeye|ipcam)/.test(`${typeText} ${text}`);
  const looksWeb = /https?:\/\/|\bwww\.|:80\b|:443\b|web|portal|admin/.test(text);
  const hasHost = hostText.trim().length > 0;
  const streamReady = actionStatuses?.stream === 'Готово' || Boolean(availability?.stream);
  const webReady = actionStatuses?.isapi === 'Готово' || actionStatuses?.onvif === 'Готово' || Boolean(availability?.isapi) || Boolean(availability?.onvif);
  const archiveReady = actionStatuses?.archive === 'Готово' || actionStatuses?.archiveSearch === 'Готово' || Boolean(availability?.archive);
  const buildSignalsText = (extra = []) => {
    const signals = [];
    if (typeText) signals.push(`тип: ${typeText}`);
    if (streamReady) signals.push('поток доступен');
    if (webReady) signals.push('web-проверки доступны');
    if (archiveReady) signals.push('архив доступен');
    extra.filter(Boolean).forEach((item) => signals.push(item));
    return signals.join(', ');
  };

  if (isHub) {
    return {
      label: 'HUB',
      stream: 'ограничен',
      web: 'частично уместны',
      archive: 'уместны',
      note: actionStatuses?.archive || 'Готово',
      basis: buildSignalsText(['класс: HUB']),
    };
  }
  if (looksCamera) {
    return {
      label: 'Камера / NVR',
      stream: streamReady ? 'уместен' : 'ограничен',
      web: webReady ? 'уместны' : 'ограничены',
      archive: archiveReady ? 'уместны' : 'ограничены',
      note: actionStatuses?.stream || 'Готово',
      basis: buildSignalsText(['класс: камера/NVR']),
    };
  }
  if (looksWeb) {
    return {
      label: 'Web-цель',
      stream: streamReady ? 'возможен' : 'не приоритет',
      web: webReady ? 'уместны' : 'частично уместны',
      archive: archiveReady ? 'уместны' : 'зависит от цели',
      note: actionStatuses?.isapi || actionStatuses?.archiveSearch || 'Готово',
      basis: buildSignalsText(['класс: web-цель']),
    };
  }
  if (hasHost) {
    return {
      label: 'Сетевой узел',
      stream: streamReady ? 'возможен' : 'неочевиден',
      web: webReady ? 'по ситуации' : 'неочевидны',
      archive: archiveReady ? 'возможны' : 'ограничены',
      note: actionStatuses?.stream || actionStatuses?.archive || 'Недостаточно данных',
      basis: buildSignalsText(['класс: сетевой узел']),
    };
  }
  return {
    label: 'Неопределённая цель',
    stream: 'неочевиден',
    web: 'неочевидны',
    archive: 'неочевидны',
    note: 'Недостаточно данных для запуска',
    basis: buildSignalsText(['класс: неопределённый']),
  };
}

function TargetsPanel({
  targets,filteredTargets,targetSearch,setTargetSearch,
  targetTypeFilter,setTargetTypeFilter,archiveOnly,setArchiveOnly,
  form,setForm,targetSaveStatus,sessionAuditStatus,runtimeLogs,selectedTarget,setSelectedTarget,hubRecon,
  handleSmartSave,handleDeleteTarget,handleGeocode,
  onNemesis,onMemoryRequest,onIsapiInfo,onIsapiSearch,
  onOnvifInfo,onOnvifRecordings,onArchiveEndpoints,onOpenHubArchive,
  onQuickStartStream,
  labels,setLabels,onLabelClick,labelEditRequest,
  capture,nvr,auditResults,handlePortScan,handleSecurityAudit,
  handleDownloadIsapiPlayback,handleCaptureIsapiPlayback,handleDownloadOnvifToken,
  isPlayableRecord,isDownloadableRecord,
  handleCaptureArchive,handleDownloadHttp,activeTargetId,streamRtspUrl,activeCameraName,
}){
  const [tab2,setTab2]=useState('targets');
  const selectedTargetAvailability = selectedTarget ? getSelectedTargetActionAvailability(selectedTarget) : null;
  const selectedTargetActionStatuses = selectedTarget ? buildLinkedActionStatuses(selectedTarget, selectedTargetAvailability) : null;
  const selectedTargetActionAggregate = selectedTargetActionStatuses ? buildLinkedActionAggregate(selectedTargetActionStatuses) : null;
  const selectedTargetCompatibilityProfile = selectedTarget
    ? buildTargetCompatibilityProfile(selectedTarget, selectedTargetAvailability, selectedTargetActionStatuses)
    : null;
  const withNormalizedTarget = (actionType, handler, target) => {
    if (typeof handler !== 'function') return;
    handler(normalizeTargetForLinkedAction(target, actionType));
  };

  useEffect(() => {
    if (labelEditRequest?.label) setTab2('labels');
  }, [labelEditRequest]);

  return(
    <>
      <div style={{display:'flex',gap:'4px',marginBottom:'10px'}}>
        {[['targets','📍 Цели',T.cyan],['labels','🏷 Метки',T.amb]].map(([id,label,col])=>(
          <button key={id} onClick={()=>setTab2(id)} style={{
            flex:1,padding:'5px',fontSize:'11px',cursor:'pointer',fontFamily:'inherit',
            background:tab2===id?col+'20':'transparent',color:tab2===id?col:T.muted,
            border:'1px solid '+(tab2===id?col+'60':T.line),borderRadius:'4px',fontWeight:tab2===id?700:400,
          }}>{label}</button>
        ))}
      </div>

      {tab2==='targets'&&<>
        <Section icon='➕' title='Добавить цель' color={T.cyan} defaultOpen={false}>
          <div style={css.row}>
            <div style={{flex:1}}><div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>Название</div>
              <input style={{...css.input,marginBottom:0}} value={form.name} onChange={e=>setForm({...form,name:e.target.value})} placeholder='ул. Ленина 5'/></div>
            <div style={{flex:1}}><div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>IP:порт</div>
              <input style={{...css.input,marginBottom:0}} value={form.host} onChange={e=>setForm({...form,host:e.target.value})} placeholder='93.125.3.58:554'/></div>
          </div>
          <div style={{height:'6px'}}/>
          <div style={css.row}>
            <input style={{...css.input,flex:1,marginBottom:0}} value={form.login} onChange={e=>setForm({...form,login:e.target.value})} placeholder='Логин'/>
            <input style={{...css.input,flex:1,marginBottom:0}} type='password' value={form.password} onChange={e=>setForm({...form,password:e.target.value})} placeholder='Пароль'/>
            <input style={{...css.input,width:'56px',marginBottom:0}} type='number' value={form.channelCount} onChange={e=>setForm({...form,channelCount:+e.target.value})} placeholder='Кам.'/>
          </div>
          <div style={{height:'6px'}}/>
          <div style={{display:'flex',gap:'6px'}}>
            <input style={{...css.input,flex:1,marginBottom:0}} value={hubRecon.addressQuery} onChange={e=>hubRecon.setAddressQuery(e.target.value)} placeholder='Адрес для геокодирования...'/>
            <button style={css.btn(T.cyan)} onClick={handleGeocode}>📍 ГЕО</button>
          </div>
          <div style={{height:'8px'}}/>
          <button style={css.btnFull(T.cyan,true)} onClick={handleSmartSave}>💾 СОХРАНИТЬ ЦЕЛЬ</button>
          {targetSaveStatus?.text && (
            <div style={{
              marginTop:'6px',
              fontSize:'10px',
              borderRadius:'4px',
              padding:'6px 8px',
              border:'1px solid ' + (targetSaveStatus.level === 'error' ? '#7a2a2a' : targetSaveStatus.level === 'warn' ? '#6c5a24' : '#245a3c'),
              background: targetSaveStatus.level === 'error' ? '#1a0b0b' : targetSaveStatus.level === 'warn' ? '#17130a' : '#0b1710',
              color: targetSaveStatus.level === 'error' ? '#ff9b9b' : targetSaveStatus.level === 'warn' ? '#ffd27d' : '#9fe0b7',
            }}>
              {targetSaveStatus.text}
            </div>
          )}
          <UnifiedTargetStatusBlock
            targetSaveStatus={targetSaveStatus}
            sessionAuditStatus={sessionAuditStatus}
            runtimeLogs={runtimeLogs}
            form={form}
            activeTargetId={activeTargetId}
            selectedTarget={selectedTarget}
          />
        </Section>

        <div style={{marginBottom:'8px'}}>
          <input style={css.input} value={targetSearch} onChange={e=>setTargetSearch(e.target.value)} placeholder='🔍 Поиск по имени или IP...'/>
          <div style={{display:'flex',gap:'4px',marginBottom:'6px'}}>
            {[['all','Все',T.text],['hub','HUB',T.purp],['local','LOCAL',T.amb]].map(([v,l,c])=>(
              <button key={v} onClick={()=>setTargetTypeFilter(v)} style={{
                flex:1,padding:'5px',fontSize:'10px',fontWeight:600,cursor:'pointer',fontFamily:'inherit',
                background:targetTypeFilter===v?c+'20':'transparent',color:targetTypeFilter===v?c:T.dim,
                border:'1px solid '+(targetTypeFilter===v?c+'60':T.line),borderRadius:'4px',}}>{l}</button>
            ))}
          </div>
          <label style={{fontSize:'10px',color:T.muted,display:'flex',gap:'6px',alignItems:'center',cursor:'pointer'}}>
            <input type='checkbox' checked={archiveOnly} onChange={e=>setArchiveOnly(e.target.checked)}/>
            Только с архивом
          </label>
        </div>
        <div style={{fontSize:'10px',color:T.muted,marginBottom:'8px'}}>Показано: {filteredTargets.length} из {targets.length}</div>
        {selectedTarget && (
          <div style={{marginBottom:'8px',padding:'6px',border:'1px solid '+T.line,borderRadius:'4px',background:T.bg1}}>
            <div style={{border:'1px solid '+T.line,borderRadius:'4px',padding:'6px',background:T.bg0,marginBottom:'6px'}}>
              <div style={{fontSize:'10px',color:T.muted,marginBottom:'4px'}}>Краткий контекст цели</div>
              <div style={{fontSize:'11px',color:T.text,fontWeight:700,marginBottom:'3px'}}>
                {selectedTarget.name || selectedTarget.host || selectedTarget.id || 'Без имени'}
              </div>
              <div style={{display:'grid',gridTemplateColumns:'repeat(2,minmax(0,1fr))',gap:'4px',fontSize:'10px',color:T.dim}}>
                <div>IP/хост: <span style={{color:T.text}}>{selectedTarget.host || selectedTarget.ip || 'не указан'}</span></div>
                <div>Тип: <span style={{color:T.text}}>{String(selectedTarget.type || 'локальная').toUpperCase()}</span></div>
                <div>Каналы: <span style={{color:T.text}}>{Array.isArray(selectedTarget.channels) ? selectedTarget.channels.length : Number(selectedTarget.channelCount || selectedTarget.cameraCount || 0)}</span></div>
                <div>Поток: <span style={{color:selectedTargetAvailability?.stream ? T.grn : T.amb}}>{selectedTargetAvailability?.stream ? 'доступен' : 'ограничен'}</span></div>
                <div>Архив: <span style={{color:selectedTargetAvailability?.archive ? T.grn : T.amb}}>{selectedTargetAvailability?.archive ? 'доступен' : 'ограничен'}</span></div>
                <div>ISAPI: <span style={{color:selectedTargetAvailability?.isapi ? T.grn : T.amb}}>{selectedTargetAvailability?.isapi ? 'доступен' : 'ограничен'}</span></div>
                <div>ONVIF: <span style={{color:selectedTargetAvailability?.onvif ? T.grn : T.amb}}>{selectedTargetAvailability?.onvif ? 'доступен' : 'ограничен'}</span></div>
              </div>
            </div>
            <div style={{fontSize:'10px',color:T.cyan,marginBottom:'6px'}}>
              В работе: <b>{selectedTarget.name || selectedTarget.host || selectedTarget.id}</b>
            </div>
            {selectedTargetCompatibilityProfile && (
              <div style={{marginBottom:'6px',border:'1px solid #2a3a4f',background:'#0c1520',borderRadius:'4px',padding:'6px 8px'}}>
                <div style={{fontSize:'10px',color:'#7f93a4',marginBottom:'3px'}}>Профиль совместимости цели</div>
                <div style={{fontSize:'10px',color:T.text,marginBottom:'3px'}}>Профиль: <b>{selectedTargetCompatibilityProfile.label}</b></div>
                <div style={{fontSize:'10px',color:T.muted}}>
                  Поток: <b style={{color:'#9ec58f'}}>{selectedTargetCompatibilityProfile.stream}</b> ·
                  Web-проверки: <b style={{color:'#9ec58f'}}> {selectedTargetCompatibilityProfile.web}</b> ·
                  Архив: <b style={{color:'#9ec58f'}}> {selectedTargetCompatibilityProfile.archive}</b>
                </div>
                <div style={{fontSize:'10px',color:T.muted,marginTop:'3px'}}>
                  Ключевой сигнал сейчас: <b style={{color:T.amb}}>{selectedTargetCompatibilityProfile.note}</b>
                </div>
                <div style={{fontSize:'10px',color:'#6f8394',marginTop:'2px'}}>
                  Основание профиля: <span>{selectedTargetCompatibilityProfile.basis || 'сигналов недостаточно'}</span>
                </div>
              </div>
            )}
            <div style={{fontSize:'10px',color:T.muted,marginBottom:'2px'}}>Быстрые действия по цели</div>
            {selectedTargetCompatibilityProfile && (
              <div style={{fontSize:'10px',color:'#7f93a4',marginBottom:'5px'}}>
                Контур: <b style={{color:T.text}}>{selectedTargetCompatibilityProfile.label}</b> · ключевой сигнал: <b style={{color:T.amb}}>{selectedTargetCompatibilityProfile.note}</b>
              </div>
            )}
            <div style={{display:'grid',gridTemplateColumns:'repeat(2,minmax(0,1fr))',gap:'4px'}}>
              {String(selectedTarget?.type || '').toLowerCase() !== 'hub' && (
                <button style={css.btn(T.cyan)} onClick={()=>withNormalizedTarget('stream', onQuickStartStream, selectedTarget)}>Открыть поток (1-й канал)</button>
              )}
              <button style={css.btn(T.cyan)} onClick={()=>withNormalizedTarget('isapi_info', onIsapiInfo, selectedTarget)}>Проверить ISAPI</button>
              <button style={css.btn(T.grn)} onClick={()=>withNormalizedTarget('onvif_info', onOnvifInfo, selectedTarget)}>Проверить ONVIF</button>
              <button style={css.btn(T.amb)} onClick={()=>withNormalizedTarget('archive_search', onIsapiSearch, selectedTarget)}>Поиск в архиве</button>
              {String(selectedTarget?.type || '').toLowerCase() === 'hub'
                ? <button style={css.btn(T.blue)} onClick={()=>withNormalizedTarget('hub_archive', onOpenHubArchive, selectedTarget)}>Открыть архив HUB</button>
                : <button style={css.btn(T.purp)} onClick={()=>withNormalizedTarget('archive_endpoints', onArchiveEndpoints, selectedTarget)}>Открыть точки архива</button>}
            </div>
            {selectedTargetActionAggregate && (
              <div style={{marginTop:'6px',border:'1px solid #253449',background:'#0b121c',borderRadius:'4px',padding:'6px 8px'}}>
                <div style={{fontSize:'10px',color:'#7f93a4',marginBottom:'3px'}}>Сводка готовности действий</div>
                <div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>
                  Готово: <b style={{color:T.grn}}>{selectedTargetActionAggregate.readyCount}</b> ·
                  Ограничено: <b style={{color:T.amb}}> {selectedTargetActionAggregate.limitedCount}</b> ·
                  Блокирует: <b style={{color:T.red}}> {selectedTargetActionAggregate.blockedCount}</b>
                </div>
                <div style={{fontSize:'10px',color:T.muted}}>
                  Ключевая причина: <b style={{color:selectedTargetActionAggregate.mainReason === 'Готово' ? T.grn : T.amb}}>{selectedTargetActionAggregate.mainReason}</b>
                </div>
                {(selectedTargetActionAggregate.readyActions?.length > 0 || selectedTargetActionAggregate.limitedActions?.length > 0 || selectedTargetActionAggregate.blockedActions?.length > 0) && (
                  <div style={{fontSize:'10px',color:T.muted,marginTop:'3px'}}>
                    {selectedTargetActionAggregate.readyActions?.length > 0 && (
                      <div>Готовы: <b style={{color:T.grn}}>{selectedTargetActionAggregate.readyActions.join(', ')}</b></div>
                    )}
                    {selectedTargetActionAggregate.limitedActions?.length > 0 && (
                      <div>Ограничены: <b style={{color:T.amb}}>{selectedTargetActionAggregate.limitedActions.join(', ')}</b></div>
                    )}
                    {selectedTargetActionAggregate.blockedActions?.length > 0 && (
                      <div>Блокирует запуск: <b style={{color:T.red}}>{selectedTargetActionAggregate.blockedActions.join(', ')}</b></div>
                    )}
                  </div>
                )}
              </div>
            )}
            <div style={{marginTop:'6px',display:'grid',gap:'3px'}}>
              {String(selectedTarget?.type || '').toLowerCase() !== 'hub' && (
                <div style={{fontSize:'10px',color:T.muted}}>Поток: <b style={{color:selectedTargetActionStatuses?.stream === 'Готово' ? T.grn : T.amb}}>{selectedTargetActionStatuses?.stream || '—'}</b></div>
              )}
              <div style={{fontSize:'10px',color:T.muted}}>ISAPI: <b style={{color:selectedTargetActionStatuses?.isapi === 'Готово' ? T.grn : T.amb}}>{selectedTargetActionStatuses?.isapi || '—'}</b></div>
              <div style={{fontSize:'10px',color:T.muted}}>ONVIF: <b style={{color:selectedTargetActionStatuses?.onvif === 'Готово' ? T.grn : T.amb}}>{selectedTargetActionStatuses?.onvif || '—'}</b></div>
              <div style={{fontSize:'10px',color:T.muted}}>Поиск в архиве: <b style={{color:selectedTargetActionStatuses?.archiveSearch === 'Готово' ? T.grn : T.amb}}>{selectedTargetActionStatuses?.archiveSearch || '—'}</b></div>
              <div style={{fontSize:'10px',color:T.muted}}>
                {String(selectedTarget?.type || '').toLowerCase() === 'hub' ? 'Архив HUB' : 'Точки архива'}: <b style={{color:selectedTargetActionStatuses?.archive === 'Готово' ? T.grn : T.amb}}>{selectedTargetActionStatuses?.archive || '—'}</b>
              </div>
            </div>
          </div>
        )}
        {filteredTargets.map(t=>{
          const cardAvailability = getSelectedTargetActionAvailability(t);
          const cardActionStatuses = buildLinkedActionStatuses(t, cardAvailability);
          const cardCompatibilityProfile = buildTargetCompatibilityProfile(t, cardAvailability, cardActionStatuses);
          return (
            <TargetCard key={t.id} target={t}
              selected={selectedTarget?.id === t.id}
              compatibilityProfile={cardCompatibilityProfile}
              onSelect={setSelectedTarget}
              onNemesis={onNemesis} onMemoryRequest={onMemoryRequest}
              onIsapiInfo={(x)=>withNormalizedTarget('isapi_info', onIsapiInfo, x)}
              onIsapiSearch={(x)=>withNormalizedTarget('archive_search', onIsapiSearch, x)}
              onOnvifInfo={(x)=>withNormalizedTarget('onvif_info', onOnvifInfo, x)}
              onOnvifRecordings={(x)=>withNormalizedTarget('onvif_recordings', onOnvifRecordings, x)}
              onArchiveEndpoints={(x)=>withNormalizedTarget('archive_endpoints', onArchiveEndpoints, x)}
              onOpenHubArchive={(x)=>withNormalizedTarget('hub_archive', onOpenHubArchive, x)}
              onDelete={handleDeleteTarget}/>
          );
        })}

        <Section icon='📦' title='Захват архива' color={T.amb} defaultOpen={false}>
          <CapturePanel
            capture={capture}
            handleCaptureArchive={handleCaptureArchive}
            handleDownloadHttp={handleDownloadHttp}
            activeTargetId={activeTargetId}
            streamRtspUrl={streamRtspUrl}
            activeCameraName={activeCameraName}
          />
        </Section>

        <NvrProbePanel nvr={nvr} capture={capture} auditResults={auditResults||[]}
          handlePortScan={handlePortScan} handleSecurityAudit={handleSecurityAudit}
          isPlayableRecord={isPlayableRecord} isDownloadableRecord={isDownloadableRecord}
          handleDownloadIsapiPlayback={handleDownloadIsapiPlayback}
          handleCaptureIsapiPlayback={handleCaptureIsapiPlayback}
          handleDownloadOnvifToken={handleDownloadOnvifToken}/>
      </>}

      {tab2==='labels'&&<LabelPanel labels={labels} setLabels={setLabels} onLabelClick={onLabelClick} requestedEditRequest={labelEditRequest}/>}
    </>
  );
}

function OpsPanel({
  agentScope,setAgentScope,handleRunReconAgent,agentStatus,agentPacket,handleAgentHandoff,
  isSniffing,handleStartSniffer,interceptLogs,implementationStatus,onPlayCamera,
  handleStartNemesis,handleAnalyzeSources,handlePlayFuzzedLink,
  hubRecon,capture,hubConfig,fuzzPath,formatBytes,handleCaptureArchive,
  labMode,
}){
  const [showPb,setShowPb]=useState(false);
  const [showCamp,setShowCamp]=useState(false);
  const [showIot,setShowIot]=useState(false);
  const [showRadar,setShowRadar]=useState(false);
  const [campId,setCampId]=useState(null);

  return(
    <>
      {labMode && <Section icon='🤖' title='LAB: Агент-конвейер' color={T.cyan}>
        <div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>Область сканирования</div>
        <input style={css.input} value={agentScope} onChange={e=>setAgentScope(e.target.value)} placeholder='Хост или подсеть: 192.168.1.0/24'/>
        <button style={css.btnFull(T.cyan)} onClick={handleRunReconAgent}>▶ Запустить разведчика</button>
        {agentStatus&&<div style={{fontSize:'11px',color:T.muted,marginTop:'4px'}}>{agentStatus}</div>}
        {agentPacket&&<AgentReport packet={agentPacket} nextAgent='ExploitVerifyAgent' onHandoff={handleAgentHandoff}/>}
      </Section>}

      <Section icon='🛠' title='Инструменты' color={T.purp} defaultOpen={false}>
        <button style={css.btnFull(T.purp)} onClick={()=>setShowPb(v=>!v)}>📋 {showPb?'Скрыть плейбуки':'Плейбуки'}</button>
        {showPb&&<PlaybookRunner/>}
        <button style={css.btnFull(T.amb)} onClick={()=>setShowCamp(v=>!v)}>📁 {showCamp?'Скрыть кампании':'Кампании'}</button>
        {showCamp&&<><CampaignList onOpen={setCampId}/><CampaignDashboard campaignId={campId}/></>}
        <button style={css.btnFull(T.muted)} onClick={()=>setShowIot(v=>!v)}>🛡 IoT-аудит</button>
        {showIot&&<PassiveScanner/>}
        <button style={{...css.btnFull(T.cyan),background:showRadar?T.cyan:T.cyan+'18',color:showRadar?'#000':T.cyan}} onClick={()=>setShowRadar(v=>!v)}>📡 Радар камер</button>
        {showRadar&&<CameraScanPanel onPlayCamera={onPlayCamera}/>}        
        <MassAudit/>
      </Section>

      {labMode && <Section icon='🧪' title='LAB: Экспериментальные операции' color={T.red} defaultOpen={false}>
        <button style={css.btnFull(isSniffing?T.grn:T.cyan)} onClick={handleStartSniffer} disabled={isSniffing}>
          {isSniffing?'🎧 Перехват активен...':'🎧 Пассивный перехват'}</button>
        {interceptLogs.length>0&&<div style={{background:T.bg0,border:'1px solid '+T.grn+'30',padding:'8px',fontSize:'10px',color:T.grn,maxHeight:'80px',overflowY:'auto',borderRadius:'4px',marginBottom:'6px'}}>
          {interceptLogs.map((l,i)=><div key={i}>[{l.protocol}] {l.details}</div>)}</div>}
        <SpiderControl
          handleStartNemesis={handleStartNemesis}
          handleAnalyzeSources={handleAnalyzeSources}
          handlePlayFuzzedLink={handlePlayFuzzedLink}
        />
      </Section>}

      <Section icon='🔍' title='Разведка архива (HUB)' color={T.grn} defaultOpen={false}>
        <HubReconPanel hubRecon={hubRecon} capture={capture} hubConfig={hubConfig} fuzzPath={fuzzPath} formatBytes={formatBytes} handleCaptureArchive={handleCaptureArchive}/>
      </Section>

      {implementationStatus&&<Section icon='📊' title='Статус системы' color={T.grn} defaultOpen={false}>
        <div style={{fontSize:'11px',color:T.text,marginBottom:'6px'}}>
          Выполнено: <b style={{color:T.grn}}>{implementationStatus.completed}</b>/{implementationStatus.total}</div>
        <div style={{maxHeight:'100px',overflowY:'auto'}}>
          {(implementationStatus.items||[]).map((item,i)=>(
            <div key={i} style={{fontSize:'10px',color:item.status==='completed'?T.grn:T.muted,marginBottom:'2px'}}>
              {item.status==='completed'?'✓':'○'} {item.name}</div>
          ))}
        </div>
      </Section>}
    </>
  );
}

function SystemPanel({runtimeLogs,setRuntimeLogs,downloadTasks,resumeDownloads,setResumeDownloads,handleCancelDownloadTask,handleRetryDownloadTask,handleClearDownloads}){
  return(
    <>
      <Section icon='📟' title='Логи ядра' color={T.grn}>
        <PolicyRuntimeStatusBlock runtimeLogs={runtimeLogs}/>
        <RuntimeLogs runtimeLogs={runtimeLogs} setRuntimeLogs={setRuntimeLogs}/>
      </Section>
      <Section icon='🔗' title='FTP Relay' color={T.blue} defaultOpen={false}>
        <RelayPanel/>
      </Section>
      <Section icon='⬇' title='Загрузки' color={T.amb}>
        <label style={{fontSize:'11px',color:T.muted,display:'flex',gap:'6px',alignItems:'center',marginBottom:'8px',cursor:'pointer'}}>
          <input type='checkbox' checked={resumeDownloads} onChange={e=>setResumeDownloads(e.target.checked)}/>
          Докачка при обрыве
        </label>
        {downloadTasks.length>0&&<button style={{...css.btnFull(T.red),marginBottom:'8px'}} onClick={handleClearDownloads}>Очистить завершённые</button>}
        {downloadTasks.length===0
          ?<div style={{fontSize:'11px',color:T.dim,textAlign:'center',padding:'12px 0'}}>Загрузок нет</div>
          :downloadTasks.map(task=>(
            <div key={task.id} style={{background:T.bg0,border:'1px solid '+T.line,borderRadius:'4px',padding:'8px',marginBottom:'6px'}}>
              <div style={{display:'flex',justifyContent:'space-between',marginBottom:'4px'}}>
                <span style={{fontSize:'11px',color:T.text,flex:1,overflow:'hidden',textOverflow:'ellipsis',whiteSpace:'nowrap'}}>{task.filename}</span>
                <span style={{fontSize:'10px',color:task.status==='done'?T.grn:task.status==='error'?T.red:T.amb,marginLeft:'8px',flexShrink:0}}>
                  {task.status==='done'?'✓ Готово':task.status==='error'?'✗ Ошибка':(task.percent??0)+'%'}</span>
              </div>
              {task.status==='running'&&<div style={{height:'3px',background:T.line,borderRadius:'2px',overflow:'hidden'}}>
                <div style={{height:'100%',width:(task.percent??5)+'%',background:T.cyan,transition:'width .3s'}}/></div>}
              {task.status==='error'&&task.error&&<div style={{fontSize:'10px',color:T.red,marginTop:'3px'}}>{task.error.slice(0,80)}</div>}
              <div style={{display:'flex',gap:'4px',marginTop:'6px'}}>
                {task.status==='running'&&<button style={css.btn(T.red)} onClick={()=>handleCancelDownloadTask(task)}>Отмена</button>}
                {task.status==='error'&&<button style={css.btn(T.amb)} onClick={()=>handleRetryDownloadTask(task)}>Повтор</button>}
              </div>
            </div>
          ))
        }
      </Section>
    </>
  );
}

export default function Sidebar(props){
  const {
    targets,filteredTargets,targetSearch,setTargetSearch,
    targetTypeFilter,setTargetTypeFilter,archiveOnly,setArchiveOnly,
    form,setForm,targetSaveStatus,hubRecon,
    handleSmartSave,handleDeleteTarget,handleGeocode,
    onNemesis,onMemoryRequest,onIsapiInfo,onIsapiSearch,
    onOnvifInfo,onOnvifRecordings,onArchiveEndpoints,onOpenHubArchive,
    onQuickStartStream,
    agentScope,setAgentScope,handleRunReconAgent,agentStatus,agentPacket,handleAgentHandoff,
    fuzzPath,handleAnalyzeSources,handlePlayFuzzedLink,
    isSniffing,handleStartSniffer,interceptLogs,implementationStatus,onPlayCamera,
    handleStartNemesis,
    runtimeLogs,setRuntimeLogs,downloadTasks,resumeDownloads,setResumeDownloads,
    handleCancelDownloadTask,handleRetryDownloadTask,handleClearDownloads,
    labels,setLabels,onLabelClick,labelEditRequest,
    nvr,capture,auditResults,handlePortScan,handleSecurityAudit,
    handleDownloadIsapiPlayback,handleCaptureIsapiPlayback,handleDownloadOnvifToken,
    isPlayableRecord,isDownloadableRecord,handleCaptureArchive,
    handleDownloadHttp,activeTargetId,streamRtspUrl,activeCameraName,hubConfig,formatBytes,
  }=props;

  const [tab,setTab]=useState('targets');
  const [labMode, setLabMode] = useState(() => {
    try {
      return localStorage.getItem('hyperion_lab_mode_v1') === '1';
    } catch {
      return false;
    }
  });
  const [sessionAuditStatus, setSessionAuditStatus] = useState(null);
  const [selectedTarget, setSelectedTarget] = useState(null);

  useEffect(() => {
    if (labelEditRequest?.label) setTab('targets');
  }, [labelEditRequest]);

  useEffect(() => {
    if (!selectedTarget?.id) return;
    const stillExists = (targets || []).some((t) => t?.id === selectedTarget.id);
    if (!stillExists) setSelectedTarget(null);
  }, [targets, selectedTarget]);

  useEffect(() => {
    try {
      localStorage.setItem('hyperion_lab_mode_v1', labMode ? '1' : '0');
    } catch {}
  }, [labMode]);

  return(
    <div style={css.panel}>
      <div style={{padding:'10px 14px 8px',borderBottom:'1px solid '+T.line,flexShrink:0,background:T.bg1}}>
        <div style={{display:'flex',alignItems:'baseline',gap:'8px'}}>
          <span style={{color:T.red,fontSize:'13px',fontWeight:800,letterSpacing:'.12em'}}>HYPERION</span>
          <span style={{color:T.dim,fontSize:'10px'}}>NODE</span>
          <button
            style={{...css.btn(labMode ? T.amb : T.muted),padding:'3px 7px',fontSize:'9px',marginLeft:'8px'}}
            onClick={() => setLabMode((v) => !v)}
            title='Переключить экспериментальные панели'
          >
            {labMode ? 'LAB ON' : 'LAB OFF'}
          </button>
          <span style={{marginLeft:'auto',fontSize:'10px',color:T.grn}}>● онлайн</span>
        </div>
      </div>

      <div style={css.tabBar}>
        {TABS.map(t=>(
          <button key={t.id} style={css.tab(tab===t.id,t.color)} onClick={()=>setTab(t.id)}>
            <div style={{fontSize:'14px',marginBottom:'2px'}}>{t.icon}</div>
            {t.label}
          </button>
        ))}
      </div>

      <div style={css.scroll}>
        {tab==='targets'&&<TargetsPanel
          targets={targets} filteredTargets={filteredTargets}
          targetSearch={targetSearch} setTargetSearch={setTargetSearch}
          targetTypeFilter={targetTypeFilter} setTargetTypeFilter={setTargetTypeFilter}
          archiveOnly={archiveOnly} setArchiveOnly={setArchiveOnly}
          form={form} setForm={setForm} targetSaveStatus={targetSaveStatus} sessionAuditStatus={sessionAuditStatus} runtimeLogs={runtimeLogs}
          selectedTarget={selectedTarget} setSelectedTarget={setSelectedTarget}
          hubRecon={hubRecon}
          handleSmartSave={handleSmartSave} handleDeleteTarget={handleDeleteTarget}
          handleGeocode={handleGeocode}
          onNemesis={onNemesis} onMemoryRequest={onMemoryRequest}
          onIsapiInfo={onIsapiInfo} onIsapiSearch={onIsapiSearch}
          onOnvifInfo={onOnvifInfo} onOnvifRecordings={onOnvifRecordings}
          onArchiveEndpoints={onArchiveEndpoints} onOpenHubArchive={onOpenHubArchive}
          onQuickStartStream={onQuickStartStream}
          labels={labels} setLabels={setLabels} onLabelClick={onLabelClick} labelEditRequest={labelEditRequest}
          capture={capture} nvr={nvr} auditResults={auditResults}
          handlePortScan={handlePortScan} handleSecurityAudit={handleSecurityAudit}
          handleDownloadIsapiPlayback={handleDownloadIsapiPlayback}
          handleCaptureIsapiPlayback={handleCaptureIsapiPlayback}
          handleDownloadOnvifToken={handleDownloadOnvifToken}
          isPlayableRecord={isPlayableRecord} isDownloadableRecord={isDownloadableRecord}
          handleCaptureArchive={handleCaptureArchive}
          handleDownloadHttp={handleDownloadHttp}
          activeTargetId={activeTargetId}
          streamRtspUrl={streamRtspUrl}
          activeCameraName={activeCameraName}
        />}
        {tab==='ops'&&<OpsPanel
          agentScope={agentScope} setAgentScope={setAgentScope}
          handleRunReconAgent={handleRunReconAgent} agentStatus={agentStatus}
          agentPacket={agentPacket} handleAgentHandoff={handleAgentHandoff}
          handleAnalyzeSources={handleAnalyzeSources} handlePlayFuzzedLink={handlePlayFuzzedLink}
          isSniffing={isSniffing} handleStartSniffer={handleStartSniffer}
          interceptLogs={interceptLogs} implementationStatus={implementationStatus}
          onPlayCamera={onPlayCamera} handleStartNemesis={handleStartNemesis}
          hubRecon={hubRecon} capture={capture} hubConfig={hubConfig}
          fuzzPath={fuzzPath} formatBytes={formatBytes} handleCaptureArchive={handleCaptureArchive}
          labMode={labMode}
        />}
        {tab==='intel'&&<IntelHub onSessionAuditStatus={setSessionAuditStatus} selectedTarget={selectedTarget} labMode={labMode}/>}
        {tab==='system'&&<SystemPanel
          runtimeLogs={runtimeLogs} setRuntimeLogs={setRuntimeLogs}
          downloadTasks={downloadTasks} resumeDownloads={resumeDownloads}
          setResumeDownloads={setResumeDownloads}
          handleCancelDownloadTask={handleCancelDownloadTask}
          handleRetryDownloadTask={handleRetryDownloadTask}
          handleClearDownloads={handleClearDownloads}
        />}
      </div>
    </div>
  );
}
