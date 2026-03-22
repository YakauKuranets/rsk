import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../store/appStore';
import MetaPanel from './MetaAgentPanel';
import ScoutPanel from './ScoutAgentPanel';
import CVEPredictorPanel from './CVEPredictorPanel';
import LLMPanel from './LLMPanel';
import BasPanel from './BASPanel';
import PayloadPanel from './PayloadPanel';
import ToolExecutorPanel from './ToolExecutorPanel';
import SiemPanel from './SIEMPanel';

const TABS=[
  {id:'meta',icon:'🧠',label:'Мета-агент',group:'recon',color:'#b06fff'},
  {id:'scout',icon:'🔭',label:'Авто-разведка',group:'recon',color:'#00aaff'},
  {id:'cve',icon:'🎯',label:'CVE-риски',group:'recon',color:'#00cc66'},
  {id:'llm',icon:'🤖',label:'Локальный ИИ',group:'recon',color:'#00ffcc'},
  {id:'bas',icon:'⚔',label:'BAS-симуляция',group:'attack',color:'#ff4444'},
  {id:'payload',icon:'🦠',label:'Пейлоады',group:'attack',color:'#ffaa00'},
  {id:'tools',icon:'🔧',label:'Инструменты',group:'attack',color:'#cccccc'},
  {id:'siem',icon:'📡',label:'SIEM/Отчёты',group:'attack',color:'#4488ff'},
];

const GROUPS={recon:{label:'Разведка',color:'#00aaff'},attack:{label:'Атака',color:'#ff4444'}};

function SharedContextBar(){
  const intelligenceTarget = useAppStore((s)=>s.intelligenceTarget);
  const setIntelligenceTarget = useAppStore((s)=>s.setIntelligenceTarget);
  const permitToken = useAppStore((s)=>s.permitToken);
  const setPermitToken = useAppStore((s)=>s.setPermitToken);
  const ollamaUrl = useAppStore((s)=>s.ollamaUrl);
  const setOllamaUrl = useAppStore((s)=>s.setOllamaUrl);
  const ollamaModel = useAppStore((s)=>s.ollamaModel);
  const setOllamaModel = useAppStore((s)=>s.setOllamaModel);
  const ollamaTemperature = useAppStore((s)=>s.ollamaTemperature);
  const setOllamaTemperature = useAppStore((s)=>s.setOllamaTemperature);
  const [llmStatus,setLlmStatus]=useState('Не проверено');
  const [toolStatus,setToolStatus]=useState('Не проверено');
  const [busy,setBusy]=useState('');

  const checkLlm = async () => {
    setBusy('llm');
    try {
      const ok = await invoke('llm_health_check', { ollamaUrl });
      setLlmStatus(ok ? 'Ollama доступен' : 'Ollama не отвечает');
    } catch (e) {
      setLlmStatus(`LLM ошибка: ${e}`);
    } finally {
      setBusy('');
    }
  };

  const checkTools = async () => {
    setBusy('tools');
    try {
      const tools = await invoke('check_tools_available');
      const available = Array.isArray(tools) ? tools.filter((t)=>t.available).length : 0;
      const total = Array.isArray(tools) ? tools.length : 0;
      setToolStatus(`Доступно ${available}/${total} CLI-инструментов`);
    } catch (e) {
      setToolStatus(`Проверка CLI не удалась: ${e}`);
    } finally {
      setBusy('');
    }
  };

  useEffect(() => {
    checkLlm();
    checkTools();
  }, []);

  return (
    <div style={{margin:'10px 12px 0',padding:'10px',border:'1px solid #1b2535',borderRadius:'6px',background:'#080d15'}}>
      <div style={{color:'#7aa2d8',fontSize:'10px',fontWeight:700,letterSpacing:'.08em',textTransform:'uppercase',marginBottom:'8px'}}>Операционный контекст</div>
      <input value={intelligenceTarget} onChange={(e)=>setIntelligenceTarget(e.target.value)} placeholder='Цель для модулей: IP, домен, URL, CIDR' style={{width:'100%',boxSizing:'border-box',padding:'7px 8px',background:'#09111b',color:'#d7e7ff',border:'1px solid #233247',borderRadius:'4px',marginBottom:'6px'}} />
      <input value={permitToken} onChange={(e)=>setPermitToken(e.target.value)} type='password' placeholder='Permit token для BAS / tools / payload / meta-agent' style={{width:'100%',boxSizing:'border-box',padding:'7px 8px',background:'#09111b',color:'#d7e7ff',border:'1px solid #233247',borderRadius:'4px',marginBottom:'6px'}} />
      <div style={{display:'flex',gap:'6px',marginBottom:'6px'}}>
        <input value={ollamaUrl} onChange={(e)=>setOllamaUrl(e.target.value)} placeholder='Ollama URL' style={{flex:2,padding:'7px 8px',background:'#09111b',color:'#d7e7ff',border:'1px solid #233247',borderRadius:'4px'}} />
        <select value={ollamaModel} onChange={(e)=>setOllamaModel(e.target.value)} style={{flex:1,padding:'7px 8px',background:'#09111b',color:'#d7e7ff',border:'1px solid #233247',borderRadius:'4px'}}>
          <option value='llama3'>llama3</option>
          <option value='deepseek-r1'>deepseek-r1</option>
          <option value='mistral'>mistral</option>
          <option value='phi3'>phi3</option>
          <option value='qwen2.5'>qwen2.5</option>
        </select>
        <input value={ollamaTemperature} onChange={(e)=>setOllamaTemperature(Number(e.target.value)||0)} type='number' min='0' max='1' step='0.1' style={{width:'80px',padding:'7px 8px',background:'#09111b',color:'#d7e7ff',border:'1px solid #233247',borderRadius:'4px'}} />
      </div>
      <div style={{display:'flex',gap:'6px',marginBottom:'8px'}}>
        <button onClick={checkLlm} disabled={busy==='llm'} style={{flex:1,padding:'6px 8px',background:'#10303a',color:'#79e4ff',border:'1px solid #2a5868',borderRadius:'4px',cursor:'pointer'}}>🩺 LLM</button>
        <button onClick={checkTools} disabled={busy==='tools'} style={{flex:1,padding:'6px 8px',background:'#191919',color:'#d0d0d0',border:'1px solid #444',borderRadius:'4px',cursor:'pointer'}}>🔧 CLI</button>
      </div>
      <div style={{display:'grid',gap:'4px',fontSize:'11px'}}>
        <div style={{color: llmStatus.includes('доступен') ? '#7ef0a8' : '#9fc6d5'}}>LLM: {llmStatus}</div>
        <div style={{color: toolStatus.includes('Доступно') ? '#d7d7d7' : '#ff9898'}}>Tools: {toolStatus}</div>
      </div>
    </div>
  );
}

export default function IntelligenceHub({ onSessionAuditStatus }){
  const [active,setActive]=useState('meta');
  const tab=useMemo(()=>TABS.find(t=>t.id===active),[active]);

  const PANELS={
    meta:<MetaPanel/>,
    scout:<ScoutPanel/>,
    cve:<CVEPredictorPanel/>,
    llm:<LLMPanel/>,
    bas:<BasPanel/>,
    payload:<PayloadPanel/>,
    tools:<ToolExecutorPanel onSessionAuditStatus={onSessionAuditStatus}/>,
    siem:<SiemPanel/>,
  };

  return(
    <div style={{border:'1px solid #2a2a3a',background:'#0a0a12',borderRadius:'4px',overflow:'hidden',marginBottom:'16px'}}>
      <div style={{padding:'8px 12px',borderBottom:'1px solid #1a1a2a',background:'#0d0d1a'}}>
        <div style={{color:'#ff003c',fontSize:'11px',fontWeight:700,letterSpacing:'.1em',marginBottom:'2px'}}>HYPERION — РАЗВЕДКА И АТАКА</div>
        <div style={{color:'#444466',fontSize:'10px'}}>Панели используют общий target / permit token / LLM config ↓</div>
      </div>

      <SharedContextBar/>

      <div style={{background:'#0d0d1a',borderTop:'1px solid #1a1a2a',borderBottom:'1px solid #1a1a2a',marginTop:'10px'}}>
        {Object.entries(GROUPS).map(([gid,grp])=>(
          <div key={gid} style={{borderBottom:'1px solid #111'}}>
            <div style={{padding:'4px 10px 2px',fontSize:'9px',fontWeight:600,textTransform:'uppercase',letterSpacing:'.08em',color:grp.color+'88'}}>{grp.label}</div>
            <div style={{display:'flex',flexWrap:'wrap',gap:'3px',padding:'3px 8px 6px'}}>
              {TABS.filter(t=>t.group===gid).map(t=>(
                <button key={t.id} onClick={()=>setActive(t.id)} style={{
                  padding:'5px 10px',fontSize:'11px',fontWeight:active===t.id?700:400,cursor:'pointer',
                  border:'1px solid '+(active===t.id?t.color:'#2a2a3a'),
                  background:active===t.id?t.color+'22':'transparent',
                  color:active===t.id?t.color:'#888',borderRadius:'3px',transition:'all .15s',
                }}>
                  {t.icon} {t.label}
                </button>
              ))}
            </div>
          </div>
        ))}
      </div>

      {tab&&(
        <div style={{padding:'8px 12px 0',display:'flex',alignItems:'center',gap:'8px'}}>
          <span style={{fontSize:'18px'}}>{tab.icon}</span>
          <span style={{fontSize:'13px',fontWeight:600,color:tab.color}}>{tab.label}</span>
        </div>
      )}

      <div style={{padding:'10px 12px'}}>
        {PANELS[active]}
      </div>
    </div>
  );
}
