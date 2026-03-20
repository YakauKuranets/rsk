import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../store/appStore';

const S={
  wrap:{border:'1px solid #2a2a2a',padding:'10px',backgroundColor:'#0a0a0a',marginBottom:'8px'},
  h:{color:'#cccccc',marginTop:0,fontSize:'0.85rem',letterSpacing:'0.08em'},
  inp:{width:'100%',padding:'5px 8px',background:'#0a0a0a',color:'#ccc',border:'1px solid #2a2a2a',marginBottom:'6px',fontSize:'12px',boxSizing:'border-box'},
  btn:(c='#cccccc')=>({width:'100%',padding:'6px',cursor:'pointer',fontWeight:'bold',fontSize:'12px',marginBottom:'4px',background:c+'22',color:c,border:'1px solid '+c+'55'}),
};

const TOOLS=['nmap','nikto','nuclei','hydra','sqlmap','amass','gobuster','masscan','ffuf'];
const PRESETS={nmap:'-sV -sC -p 80,443,554',nikto:'-h',nuclei:'-t cves/',hydra:'-l admin -P wordlist.txt http-get',sqlmap:'-u',amass:'enum -passive -d',masscan:'-p 80,443,554 --rate 1000'};

export default function ToolExecutorPanel(){
  const intelligenceTarget = useAppStore((s)=>s.intelligenceTarget);
  const setIntelligenceTarget = useAppStore((s)=>s.setIntelligenceTarget);
  const permit = useAppStore((s)=>s.permitToken);
  const setPerm = useAppStore((s)=>s.setPermitToken);
  const [tool,setTool]=useState('nmap');
  const [args,setArgs]=useState('-sV -sC -p 80,443,554');
  const [timeout,setTo]=useState(120);
  const [load,setLoad]=useState(false);
  const [result,setResult]=useState(null);
  const [avail,setAvail]=useState([]);

  const run=async()=>{
    if(!intelligenceTarget.trim())return alert('Введите цель');
    if(permit.trim().length<8)return alert('Нужен токен');
    setLoad(true);setResult(null);
    try{setResult(await invoke('execute_tool',{req:{tool,target:intelligenceTarget.trim(),args:args.trim().split(/\s+/).filter(Boolean),timeoutSecs:+timeout,permitToken:permit.trim()}}));}
    catch(e){alert('Ошибка: '+e);}
    setLoad(false);
  };

  return(
    <div style={S.wrap}>
      <h3 style={S.h}>🔧 ИНСТРУМЕНТЫ (UNIFIED API)</h3>
      <div style={{display:'flex',gap:'4px',flexWrap:'wrap',marginBottom:'8px'}}>
        {TOOLS.map(t=>(
          <button key={t} onClick={()=>{setTool(t);setArgs(PRESETS[t]||'');}}
            style={{padding:'3px 8px',background:tool===t?'#1a1a1a':'transparent',
              color:tool===t?'#eee':'#555',border:'1px solid '+(tool===t?'#555':'#1a1a1a'),
              cursor:'pointer',fontSize:'10px',borderRadius:'3px'}}>{t}</button>
        ))}
      </div>
      <input style={S.inp} value={intelligenceTarget} onChange={e=>setIntelligenceTarget(e.target.value)} placeholder='192.168.1.0/24 или example.com'/>
      <input style={S.inp} value={args} onChange={e=>setArgs(e.target.value)} placeholder='Аргументы: -sV -sC -p 80,443'/>
      <input style={S.inp} value={permit} onChange={e=>setPerm(e.target.value)} placeholder='Разрешительный токен' type='password'/>
      <div style={{display:'flex',gap:'6px',marginBottom:'6px'}}>
        <div style={{flex:1}}><div style={{fontSize:'10px',color:'#555',marginBottom:'2px'}}>Таймаут (сек)</div>
          <input style={{...S.inp,marginBottom:0}} type='number' value={timeout} onChange={e=>setTo(e.target.value)}/></div>
        <div style={{flex:2,display:'flex',flexDirection:'column',gap:'3px'}}>
          <button style={{...S.btn(),flex:1,marginBottom:0}} onClick={run} disabled={load}>{load?'⚙...':'▶ '+tool}</button>
          <button style={{...S.btn('#555'),flex:1,marginBottom:0,fontSize:'10px'}} onClick={()=>invoke('check_tools_available').then(setAvail).catch(()=>{})}>Проверить доступность</button>
        </div>
      </div>
      {avail.length>0&&<div style={{display:'flex',flexWrap:'wrap',gap:'4px',margin:'6px 0'}}>
        {avail.map(t=><span key={t.tool} style={{fontSize:'9px',background:(t.available?'#00aa44':'#aa3333')+'20',color:t.available?'#00aa44':'#aa3333',border:'1px solid '+(t.available?'#00aa44':'#aa3333')+'40',padding:'2px 6px',borderRadius:'8px'}}>{t.tool}: {t.available?'✓':'✗'}</span>)}
      </div>}
      {result&&<div style={{marginTop:'6px'}}>
        <div style={{display:'flex',gap:'8px',fontSize:'10px',color:'#666',marginBottom:'4px'}}>
          <span>Код: <b style={{color:result.exitCode===0?'#00aa44':'#ff4444'}}>{result.exitCode}</b></span>
          <span>Время: <b style={{color:'#aaa'}}>{((result.durationMs||0)/1000).toFixed(1)}с</b></span>
          <span>Findings: <b style={{color:result.findingsExtracted?.length>0?'#ffaa00':'#444'}}>{result.findingsExtracted?.length||0}</b></span>
        </div>
        {result.findingsExtracted?.length>0&&<div style={{background:'#0a1205',border:'1px solid #1a3a1a',padding:'6px',marginBottom:'4px',fontSize:'10px',color:'#00aa44',maxHeight:'80px',overflowY:'auto',fontFamily:'monospace',borderRadius:'3px'}}>
          {result.findingsExtracted.map((f,i)=><div key={i}>{f}</div>)}</div>}
        <div style={{background:'#0a0a0a',border:'1px solid #1a1a1a',padding:'6px',fontSize:'10px',color:'#666',maxHeight:'100px',overflowY:'auto',fontFamily:'monospace',whiteSpace:'pre-wrap',wordBreak:'break-all',borderRadius:'3px'}}>
          {(result.stdout||'').slice(0,2000)||(result.stderr||'').slice(0,500)}</div>
      </div>}
    </div>
  );
}
