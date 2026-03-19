import { useState } from 'react';
import MetaPanel from './MetaAgentPanel';
import ScoutPanel from './ScoutAgentPanel';
import CVEPredictorPanel from './CVEPredictorPanel';
import LLMPanel from './LLMPanel';
import BasPanel from './BASPanel';
import PayloadPanel from './PayloadPanel';
import ToolExecutorPanel from './ToolExecutorPanel';
import SiemPanel from './SIEMPanel';

const TABS=[
  {id:'meta',  icon:'🧠',label:'Мета-агент',   group:'recon', color:'#b06fff'},
  {id:'scout', icon:'🔭',label:'Авто-разведка',group:'recon', color:'#00aaff'},
  {id:'cve',   icon:'🎯',label:'CVE-риски',    group:'recon', color:'#00cc66'},
  {id:'llm',   icon:'🤖',label:'Локальный ИИ', group:'recon', color:'#00ffcc'},
  {id:'bas',   icon:'⚔', label:'BAS-симуляция',group:'attack',color:'#ff4444'},
  {id:'payload',icon:'🦠',label:'Пейлоады',    group:'attack',color:'#ffaa00'},
  {id:'tools', icon:'🔧',label:'Инструменты',  group:'attack',color:'#cccccc'},
  {id:'siem',  icon:'📡',label:'SIEM/Отчёты',  group:'attack',color:'#4488ff'},
];

const GROUPS={recon:{label:'Разведка',color:'#00aaff'},attack:{label:'Атака',color:'#ff4444'}};

export default function IntelligenceHub(){
  const [active,setActive]=useState('meta');
  const tab=TABS.find(t=>t.id===active);

  const PANELS={
    meta:<MetaPanel/>,
    scout:<ScoutPanel/>,
    cve:<CVEPredictorPanel/>,
    llm:<LLMPanel/>,
    bas:<BasPanel/>,
    payload:<PayloadPanel/>,
    tools:<ToolExecutorPanel/>,
    siem:<SiemPanel/>,
  };

  return(
    <div style={{border:'1px solid #2a2a3a',background:'#0a0a12',borderRadius:'4px',overflow:'hidden',marginBottom:'16px'}}>
      <div style={{padding:'8px 12px',borderBottom:'1px solid #1a1a2a',background:'#0d0d1a'}}>
        <div style={{color:'#ff003c',fontSize:'11px',fontWeight:700,letterSpacing:'.1em',marginBottom:'2px'}}>HYPERION — РАЗВЕДКА И АТАКА</div>
        <div style={{color:'#444466',fontSize:'10px'}}>Выберите модуль ниже ↓</div>
      </div>

      <div style={{background:'#0d0d1a',borderBottom:'1px solid #1a1a2a'}}>
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
