import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

const S={
  wrap:{border:'1px solid #1a2a4a',padding:'10px',backgroundColor:'#060a14',marginBottom:'8px'},
  h:{color:'#4488ff',marginTop:0,fontSize:'0.85rem',letterSpacing:'0.08em'},
  inp:{width:'100%',padding:'5px 8px',background:'#060a14',color:'#ccc',border:'1px solid #1a2a4a',marginBottom:'6px',fontSize:'12px',boxSizing:'border-box'},
  btn:(c='#4488ff')=>({width:'100%',padding:'6px',cursor:'pointer',fontWeight:'bold',fontSize:'12px',marginBottom:'4px',background:c+'22',color:c,border:'1px solid '+c+'55'}),
};
const SIEMS=[{id:'splunk',label:'Splunk HEC',color:'#ff6600',cmd:'send_to_splunk_hec'},{id:'elastic',label:'Elastic ECS',color:'#00aaff',cmd:'send_to_elastic'},{id:'qradar',label:'QRadar LEEF',color:'#cc3300',cmd:'send_to_qradar'}];

export default function SIEMPanel(){
  const [sel,setSel]=useState('splunk');
  const [host,setHost]=useState('');
  const [port,setPort]=useState('8088');
  const [token,setToken]=useState('');
  const [index,setIndex]=useState('');
  const [json,setJson]=useState('');
  const [load,setLoad]=useState(false);
  const [msg,setMsg]=useState('');

  const siem=SIEMS.find(s=>s.id===sel);

  const send=async()=>{
    if(!host.trim())return alert('Введите адрес SIEM');
    if(!json.trim())return alert('Вставьте JSON');
    setLoad(true);setMsg('');
    try{setMsg(await invoke(siem.cmd,{findingsJson:json,config:{target:sel,host:host.trim(),port:+port,token:token.trim()||null,index:index.trim()||null}}));}
    catch(e){setMsg('Ошибка: '+e);}
    setLoad(false);
  };

  const genReport=async()=>{
    if(!json.trim())return alert('Вставьте JSON');
    setLoad(true);
    try{
      const path=await invoke('generate_html_report',{findingsJson:json,nlpReportJson:null,config:{title:'Отчёт Hyperion PTES',clientName:'Клиент',operatorName:'Оператор',includeExecutive:true,includeTechnical:true,includeMitreHeatmap:true,classification:'КОНФИДЕНЦИАЛЬНО'}});
      setMsg('HTML-отчёт сохранён: '+path);
    }catch(e){setMsg('Ошибка: '+e);}
    setLoad(false);
  };

  return(
    <div style={S.wrap}>
      <h3 style={S.h}>📡 SIEM / ОТЧЁТЫ</h3>
      <div style={{display:'flex',gap:'4px',marginBottom:'8px'}}>
        {SIEMS.map(s=>(
          <button key={s.id} onClick={()=>setSel(s.id)}
            style={{flex:1,padding:'5px',background:sel===s.id?s.color+'22':'transparent',
              color:sel===s.id?s.color:'#555',border:'1px solid '+(sel===s.id?s.color:'#1a1a2a'),
              cursor:'pointer',borderRadius:'3px',fontSize:'10px'}}>{s.label}</button>
        ))}
      </div>
      <div style={{display:'flex',gap:'6px',marginBottom:'6px'}}>
        <input style={{...S.inp,marginBottom:0,flex:1}} value={host} onChange={e=>setHost(e.target.value)} placeholder='Адрес SIEM'/>
        <input style={{...S.inp,marginBottom:0,flex:0,width:'70px'}} value={port} onChange={e=>setPort(e.target.value)} placeholder='Порт' type='number'/>
      </div>
      <div style={{height:'6px'}}/>
      {sel!=='qradar'&&<input style={S.inp} value={token} onChange={e=>setToken(e.target.value)} placeholder={sel==='splunk'?'HEC-токен Splunk':'API-ключ (необязательно)'}/>}
      {sel!=='qradar'&&<input style={S.inp} value={index} onChange={e=>setIndex(e.target.value)} placeholder='Индекс (main, hyperion)'/>}
      <textarea style={{...S.inp,height:'55px'}} value={json} onChange={e=>setJson(e.target.value)} placeholder='JSON findings: [{"host":"192.168.1.1","severity":"Critical",...}]'/>
      <div style={{display:'flex',gap:'6px'}}>
        <button style={{...S.btn(siem.color),flex:1,marginBottom:0}} onClick={send} disabled={load}>{load?'⚙...':'▶ '+siem.label}</button>
        <button style={{...S.btn('#8888ff'),flex:1,marginBottom:0}} onClick={genReport} disabled={load}>📄 HTML-отчёт</button>
      </div>
      {msg&&<div style={{marginTop:'8px',background:'#060a14',border:'1px solid #1a2a4a',padding:'8px',fontSize:'11px',color:msg.includes('Ошибка')?'#ff6666':'#66aaff',fontFamily:'monospace',borderRadius:'3px'}}>{msg}</div>}
    </div>
  );
}
