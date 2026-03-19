import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const S={
  wrap:{border:'1px solid #1a2a4a',padding:'10px',backgroundColor:'#080f1a',marginBottom:'8px'},
  h:{color:'#00aaff',marginTop:0,fontSize:'0.85rem',letterSpacing:'0.08em'},
  inp:{width:'100%',padding:'5px 8px',background:'#080f1a',color:'#ccc',border:'1px solid #1a2a4a',marginBottom:'6px',fontSize:'12px',boxSizing:'border-box'},
  btn:(c='#00aaff')=>({width:'100%',padding:'6px',cursor:'pointer',fontWeight:'bold',fontSize:'12px',marginBottom:'4px',background:c+'22',color:c,border:'1px solid '+c+'55'}),
};

export default function ScoutAgentPanel(){
  const [kw,setKw]=useState('hikvision,dahua,CVE-2021-36260');
  const [ivl,setIvl]=useState(30);
  const [sid,setSid]=useState('scout_'+Date.now());
  const [sources,setSrc]=useState({shodan:true,github:true,telegram:false});
  const [shodanKey,setShodan]=useState('');
  const [agents,setAgents]=useState([]);
  const [alerts,setAlerts]=useState([]);

  useEffect(()=>{
    let unlisten;
    listen('scout-alert',e=>{
      const a=Array.isArray(e.payload)?e.payload:[e.payload];
      setAlerts(p=>[...a,...p].slice(0,50));
    }).then(fn=>{unlisten=fn;});
    invoke('list_scout_agents').then(setAgents).catch(()=>{});
    return()=>unlisten?.();
  },[]);

  const start=async()=>{
    const kws=kw.split(',').map(s=>s.trim()).filter(Boolean);
    if(!kws.length)return alert('Введите ключевые слова');
    try{
      const id=await invoke('start_scout_agent',{config:{
        scoutId:sid.trim()||'scout_'+Date.now(),
        targets:[],keywords:kws,intervalMinutes:+ivl,
        sources:Object.keys(sources).filter(k=>sources[k]),
        shodanKey:shodanKey.trim()||null,
        telegramBotToken:null,telegramChannels:[],
      }});
      setAgents(p=>[...p,id]);
    }catch(e){alert('Ошибка: '+e);}
  };

  const stop=async(id)=>{
    try{await invoke('stop_scout_agent',{scoutId:id});setAgents(p=>p.filter(a=>a!==id));}
    catch(e){alert('Ошибка: '+e);}
  };

  return(
    <div style={S.wrap}>
      <h3 style={S.h}>🔭 АВТО-РАЗВЕДКА 24/7</h3>
      <p style={{fontSize:'11px',color:'#4a7a9a',marginBottom:'10px',lineHeight:1.5}}>
        Мониторит Shodan, GitHub PoC и Telegram. При находке — алерт.</p>
      <input style={S.inp} value={kw} onChange={e=>setKw(e.target.value)} placeholder='Ключевые слова: hikvision, CVE-2021-36260' />
      <div style={{display:'flex',gap:'6px',marginBottom:'6px'}}>
        <div style={{flex:1}}><div style={{fontSize:'10px',color:'#555',marginBottom:'2px'}}>Интервал (мин)</div>
          <input style={{...S.inp,marginBottom:0}} type='number' min='5' value={ivl} onChange={e=>setIvl(e.target.value)}/></div>
        <div style={{flex:1}}><div style={{fontSize:'10px',color:'#555',marginBottom:'2px'}}>ID агента</div>
          <input style={{...S.inp,marginBottom:0}} value={sid} onChange={e=>setSid(e.target.value)}/></div>
      </div>
      <div style={{display:'flex',gap:'12px',marginBottom:'8px',flexWrap:'wrap'}}>
        {Object.keys(sources).map(s=>(
          <label key={s} style={{fontSize:'11px',color:'#8ab',cursor:'pointer',display:'flex',gap:'4px',alignItems:'center'}}>
            <input type='checkbox' checked={sources[s]} onChange={e=>setSrc(p=>({...p,[s]:e.target.checked}))}/>
            {s==='shodan'?'Shodan':s==='github'?'GitHub PoC':'Telegram'}</label>
        ))}
      </div>
      {sources.shodan&&<input style={S.inp} value={shodanKey} onChange={e=>setShodan(e.target.value)} placeholder='Shodan API ключ (необязательно)'/>}
      <button style={S.btn()} onClick={start}>▶ Запустить агента</button>
      {agents.map(id=>(
        <div key={id} style={{display:'flex',justifyContent:'space-between',alignItems:'center',background:'#080f1a',padding:'4px 8px',marginBottom:'2px',border:'1px solid #1a3a5a',borderRadius:'3px'}}>
          <span style={{fontSize:'11px',color:'#7abcdf',fontFamily:'monospace'}}>{id}</span>
          <button style={{...S.btn('#ff7070'),padding:'2px 8px',marginBottom:0}} onClick={()=>stop(id)}>Стоп</button>
        </div>
      ))}
      {alerts.length>0
        ?<div style={{maxHeight:'160px',overflowY:'auto',marginTop:'6px'}}>
           {alerts.map((a,i)=>(
             <div key={i} style={{borderLeft:'3px solid '+(a.severity==='HIGH'?'#ff4444':'#ffaa00'),
               padding:'6px 8px',marginBottom:'4px',background:'#080f1a',fontSize:'10px',
               color:a.severity==='HIGH'?'#ff8888':'#ffcc88',borderRadius:'0 3px 3px 0'}}>
               <b>[{a.source?.toUpperCase()}] {a.alertType}</b><br/>{a.description}
             </div>
           ))}
         </div>
        :<div style={{fontSize:'11px',color:'#2a4a6a',padding:'8px',textAlign:'center'}}>Нет алертов — агенты ждут...</div>
      }
    </div>
  );
}
