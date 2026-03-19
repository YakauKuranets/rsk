import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

const S={
  wrap:{border:'1px solid #1a3a22',padding:'10px',backgroundColor:'#050f08',marginBottom:'8px'},
  h:{color:'#00cc66',marginTop:0,fontSize:'0.85rem',letterSpacing:'0.08em'},
  inp:{width:'100%',padding:'5px 8px',background:'#050f08',color:'#ccc',border:'1px solid #1a3a22',marginBottom:'6px',fontSize:'12px',boxSizing:'border-box'},
  btn:(c='#00cc66')=>({width:'100%',padding:'6px',cursor:'pointer',fontWeight:'bold',fontSize:'12px',marginBottom:'4px',background:c+'22',color:c,border:'1px solid '+c+'55'}),
};
const PC={IMMEDIATE:'#ff2222',HIGH:'#ff8800',MEDIUM:'#ffcc00',LOW:'#00cc66'};
const PL={IMMEDIATE:'НЕМЕДЛЕННО',HIGH:'ВЫСОКИЙ',MEDIUM:'СРЕДНИЙ',LOW:'НИЗКИЙ'};

export default function CVEPredictorPanel(){
  const [ids,setIds]=useState('CVE-2021-36260,CVE-2017-7921,CVE-2021-33045');
  const [loading,setLoad]=useState(false);
  const [syncing,setSync]=useState(false);
  const [report,setReport]=useState(null);

  const predict=async()=>{
    const cves=ids.split(',').map(s=>s.trim()).filter(Boolean);
    if(!cves.length)return;
    setLoad(true);setReport(null);
    try{setReport(await invoke('predict_cve_risk',{cveIds:cves}));}
    catch(e){alert('Ошибка: '+e);}
    setLoad(false);
  };

  const sync=async()=>{
    setSync(true);
    try{alert(await invoke('sync_epss_scores'));}
    catch(e){alert('Ошибка: '+e);}
    setSync(false);
  };

  return(
    <div style={S.wrap}>
      <h3 style={S.h}>🎯 CVE РИСКИ (EPSS + KEV)</h3>
      <p style={{fontSize:'11px',color:'#4a8a5a',marginBottom:'10px',lineHeight:1.5}}>
        Вероятность эксплуатации за 30 дней + статус KEV (CISA).</p>
      <textarea style={{...S.inp,height:'50px'}} value={ids} onChange={e=>setIds(e.target.value)}
        placeholder='CVE-ID через запятую: CVE-2021-36260, CVE-2017-7921'/>
      <div style={{display:'flex',gap:'6px',marginBottom:'6px'}}>
        <button style={{...S.btn(),flex:1,marginBottom:0}} onClick={predict} disabled={loading}>{loading?'⚙ Анализирую...':'▶ Оценить риск'}</button>
        <button style={{...S.btn('#0088cc'),flex:1,marginBottom:0}} onClick={sync} disabled={syncing}>{syncing?'⚙...':'🔄 Обновить EPSS'}</button>
      </div>
      {report&&(
        <div style={{marginTop:'8px'}}>
          <div style={{display:'flex',gap:'8px',flexWrap:'wrap',marginBottom:'8px'}}>
            <span style={{fontSize:'10px',background:'#ff222220',color:'#ff2222',border:'1px solid #ff222240',padding:'2px 8px',borderRadius:'10px'}}>Критично: {report.immediateCount}</span>
            <span style={{fontSize:'10px',background:'#ffcc0020',color:'#ffcc00',border:'1px solid #ffcc0040',padding:'2px 8px',borderRadius:'10px'}}>Патч: {report.patchWindowDays} дн.</span>
          </div>
          <div style={{maxHeight:'180px',overflowY:'auto'}}>
            {report.predictions?.map((pr,i)=>(
              <div key={i} style={{borderBottom:'1px solid #1a3a22',padding:'5px 0'}}>
                <div style={{display:'flex',alignItems:'center',gap:'6px',marginBottom:'2px'}}>
                  <span style={{fontSize:'9px',padding:'2px 6px',borderRadius:'8px',
                    background:(PC[pr.priority]||'#888')+'20',color:PC[pr.priority]||'#888',
                    border:'1px solid '+(PC[pr.priority]||'#888')+'40'}}>{PL[pr.priority]||pr.priority}</span>
                  <span style={{color:'#88ccaa',fontWeight:'bold',fontFamily:'monospace',fontSize:'11px'}}>{pr.cveId}</span>
                  {pr.inKev&&<span style={{fontSize:'9px',background:'#ff444420',color:'#ff4444',border:'1px solid #ff444440',padding:'2px 5px',borderRadius:'8px'}}>KEV</span>}
                </div>
                <div style={{display:'flex',gap:'12px',fontSize:'10px',color:'#4a8a5a'}}>
                  {pr.cvssScore!=null&&<span>CVSS: <b style={{color:'#88cc99'}}>{pr.cvssScore?.toFixed(1)}</b></span>}
                  {pr.epssScore!=null&&<span>EPSS: <b style={{color:'#88cc99'}}>{(pr.epssScore*100).toFixed(1)}%</b></span>}
                  <span>P30d: <b style={{color:pr.exploitProbability30d>0.5?'#ff8888':'#88cc99'}}>{(pr.exploitProbability30d*100).toFixed(1)}%</b></span>
                </div>
                <div style={{color:'#3a6a4a',fontSize:'10px'}}>{pr.action}</div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
