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

function TargetsPanel({
  targets,filteredTargets,targetSearch,setTargetSearch,
  targetTypeFilter,setTargetTypeFilter,archiveOnly,setArchiveOnly,
  form,setForm,hubRecon,
  handleSmartSave,handleDeleteTarget,handleGeocode,
  onNemesis,onMemoryRequest,onIsapiInfo,onIsapiSearch,
  onOnvifInfo,onOnvifRecordings,onArchiveEndpoints,onOpenHubArchive,
  labels,setLabels,onLabelClick,labelEditRequest,
  capture,nvr,auditResults,handlePortScan,handleSecurityAudit,
  handleDownloadIsapiPlayback,handleCaptureIsapiPlayback,handleDownloadOnvifToken,
  isPlayableRecord,isDownloadableRecord,
  handleCaptureArchive,handleDownloadHttp,activeTargetId,streamRtspUrl,activeCameraName,
}){
  const [tab2,setTab2]=useState('targets');

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
        {filteredTargets.map(t=>(
          <TargetCard key={t.id} target={t}
            onNemesis={onNemesis} onMemoryRequest={onMemoryRequest}
            onIsapiInfo={onIsapiInfo} onIsapiSearch={onIsapiSearch}
            onOnvifInfo={onOnvifInfo} onOnvifRecordings={onOnvifRecordings}
            onArchiveEndpoints={onArchiveEndpoints} onOpenHubArchive={onOpenHubArchive} onDelete={handleDeleteTarget}/>
        ))}

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
  handleStartNemesis,hubRecon,capture,hubConfig,fuzzPath,formatBytes,handleCaptureArchive,
}){
  const [showPb,setShowPb]=useState(false);
  const [showCamp,setShowCamp]=useState(false);
  const [showIot,setShowIot]=useState(false);
  const [showRadar,setShowRadar]=useState(false);
  const [campId,setCampId]=useState(null);

  return(
    <>
      <Section icon='🤖' title='Агент-конвейер' color={T.cyan}>
        <div style={{fontSize:'10px',color:T.muted,marginBottom:'3px'}}>Область сканирования</div>
        <input style={css.input} value={agentScope} onChange={e=>setAgentScope(e.target.value)} placeholder='Хост или подсеть: 192.168.1.0/24'/>
        <button style={css.btnFull(T.cyan)} onClick={handleRunReconAgent}>▶ Запустить разведчика</button>
        {agentStatus&&<div style={{fontSize:'11px',color:T.muted,marginTop:'4px'}}>{agentStatus}</div>}
        {agentPacket&&<AgentReport packet={agentPacket} nextAgent='ExploitVerifyAgent' onHandoff={handleAgentHandoff}/>}
      </Section>

      <Section icon='🛠' title='Инструменты' color={T.purp} defaultOpen={false}>
        <button style={css.btnFull(isSniffing?T.grn:T.cyan)} onClick={handleStartSniffer} disabled={isSniffing}>
          {isSniffing?'🎧 Перехват активен...':'🎧 Пассивный перехват'}</button>
        {interceptLogs.length>0&&<div style={{background:T.bg0,border:'1px solid '+T.grn+'30',padding:'8px',fontSize:'10px',color:T.grn,maxHeight:'80px',overflowY:'auto',borderRadius:'4px',marginBottom:'6px'}}>
          {interceptLogs.map((l,i)=><div key={i}>[{l.protocol}] {l.details}</div>)}</div>}
        <button style={css.btnFull(T.red)} onClick={handleStartNemesis}>☢ Nemesis — взлом архива</button>
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
    form,setForm,hubRecon,
    handleSmartSave,handleDeleteTarget,handleGeocode,
    onNemesis,onMemoryRequest,onIsapiInfo,onIsapiSearch,
    onOnvifInfo,onOnvifRecordings,onArchiveEndpoints,onOpenHubArchive,
    agentScope,setAgentScope,handleRunReconAgent,agentStatus,agentPacket,handleAgentHandoff,
    isSniffing,handleStartSniffer,interceptLogs,implementationStatus,onPlayCamera,
    handleStartNemesis,
    runtimeLogs,setRuntimeLogs,downloadTasks,resumeDownloads,setResumeDownloads,
    handleCancelDownloadTask,handleRetryDownloadTask,handleClearDownloads,
    labels,setLabels,onLabelClick,labelEditRequest,
    nvr,capture,auditResults,handlePortScan,handleSecurityAudit,
    handleDownloadIsapiPlayback,handleCaptureIsapiPlayback,handleDownloadOnvifToken,
    isPlayableRecord,isDownloadableRecord,handleCaptureArchive,
    handleDownloadHttp,activeTargetId,streamRtspUrl,activeCameraName,hubConfig,fuzzPath,formatBytes,
  }=props;

  const [tab,setTab]=useState('targets');

  useEffect(() => {
    if (labelEditRequest?.label) setTab('targets');
  }, [labelEditRequest]);

  return(
    <div style={css.panel}>
      <div style={{padding:'10px 14px 8px',borderBottom:'1px solid '+T.line,flexShrink:0,background:T.bg1}}>
        <div style={{display:'flex',alignItems:'baseline',gap:'8px'}}>
          <span style={{color:T.red,fontSize:'13px',fontWeight:800,letterSpacing:'.12em'}}>HYPERION</span>
          <span style={{color:T.dim,fontSize:'10px'}}>NODE</span>
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
          form={form} setForm={setForm} hubRecon={hubRecon}
          handleSmartSave={handleSmartSave} handleDeleteTarget={handleDeleteTarget}
          handleGeocode={handleGeocode}
          onNemesis={onNemesis} onMemoryRequest={onMemoryRequest}
          onIsapiInfo={onIsapiInfo} onIsapiSearch={onIsapiSearch}
          onOnvifInfo={onOnvifInfo} onOnvifRecordings={onOnvifRecordings}
          onArchiveEndpoints={onArchiveEndpoints} onOpenHubArchive={onOpenHubArchive}
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
          isSniffing={isSniffing} handleStartSniffer={handleStartSniffer}
          interceptLogs={interceptLogs} implementationStatus={implementationStatus}
          onPlayCamera={onPlayCamera} handleStartNemesis={handleStartNemesis}
          hubRecon={hubRecon} capture={capture} hubConfig={hubConfig}
          fuzzPath={fuzzPath} formatBytes={formatBytes} handleCaptureArchive={handleCaptureArchive}
        />}
        {tab==='intel'&&<IntelHub/>}
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
