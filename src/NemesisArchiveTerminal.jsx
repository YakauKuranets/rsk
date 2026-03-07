import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

/*
 * NEMESIS ARCHIVE TERMINAL
 * Layout: точная копия Hikvision download.asp
 * Left panel: камера, тип файла, тип потока, время начала, время окончания, кнопка Поиск
 * Right panel: таблица (№, имя файла, время начала, время окончания, размер, прогресс)
 * Style: cyberpunk (scan-lines, neon, dark)
 *
 * Props: target = { host, login, password, name, channels }, onClose = fn
 */
export default function NemesisArchiveTerminal({ target, onClose }) {
  const [phase, setPhase] = useState('connecting');
  const [statusText, setStatusText] = useState('УСТАНОВКА СВЯЗИ...');
  const [camera, setCamera] = useState('101');
  const [fileType, setFileType] = useState('all');
  const [streamType, setStreamType] = useState('main');
  const [timeFrom, setTimeFrom] = useState('');
  const [timeTo, setTimeTo] = useState('');
  const [records, setRecords] = useState([]);
  const [scanning, setScanning] = useState(false);
  const [activeDownloads, setActiveDownloads] = useState({});
  const [logs, setLogs] = useState([]);
  const logRef = useRef(null);
  const bootRef = useRef(false);

  const log = (msg, type = 'info') => {
    const ts = new Date().toLocaleTimeString('ru-RU', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    setLogs(p => [...p.slice(-60), { ts, msg, type }]);
  };

  useEffect(() => { if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight; }, [logs]);

  useEffect(() => {
    const now = new Date();
    const fmt = d => `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
    setTimeFrom(`${fmt(now)} 00:00:00`);
    setTimeTo(`${fmt(now)} 23:59:59`);
    if (target.channels?.length) setCamera(String(target.channels[0]?.index || target.channels[0]?.id || '101'));
  }, []);

  useEffect(() => {
    if (bootRef.current) return; bootRef.current = true;
    (async () => {
      log('\u2622 NEMESIS ARCHIVE LINK INITIATED', 'sys');
      log(`\u0426\u0435\u043b\u044c: ${target.name || target.host}`, 'sys');
      await new Promise(r => setTimeout(r, 400));
      log('\u0417\u043e\u043d\u0434\u0438\u0440\u043e\u0432\u0430\u043d\u0438\u0435 \u043f\u043e\u0440\u0442\u0430 2019...', 'info');
      try {
        const info = await invoke('fetch_nvr_device_info', { host: target.host, login: target.login || 'admin', pass: target.password || '' });
        log(`\u0423\u0437\u0435\u043b: ${info?.bodyPreview?.substring(0, 50) || 'OK'}`, 'ok');
      } catch (e) { log(`Device info: ${e}`, 'warn'); }
      setPhase('ready'); setStatusText('\u041a\u0410\u041d\u0410\u041b \u0410\u041a\u0422\u0418\u0412\u0415\u041d');
      log('\ud83d\udd17 \u0410\u0420\u0425\u0418\u0412\u041d\u042b\u0419 \u041a\u041e\u041d\u0422\u0423\u0420 \u0413\u041e\u0422\u041e\u0412', 'ok');
    })();
  }, []);

  const handleSearch = async () => {
    setScanning(true); setRecords([]); setPhase('scanning'); setStatusText('\u0421\u041a\u0410\u041d\u0418\u0420\u041e\u0412\u0410\u041d\u0418\u0415...');
    const from = timeFrom.replace(' ', 'T') + (timeFrom.includes('Z') ? '' : 'Z');
    const to = timeTo.replace(' ', 'T') + (timeTo.includes('Z') ? '' : 'Z');
    log(`\ud83d\udd0e \u041f\u041e\u0418\u0421\u041a: ${timeFrom} \u2192 ${timeTo} | cam:${camera}`, 'info');
    try {
      const items = await invoke('search_isapi_recordings', { host: target.host, login: target.login || 'admin', pass: target.password || '', fromTime: from, toTime: to });
      const filtered = (items || []).filter(i => i.playbackUri || i.startTime);
      setRecords(filtered); setPhase('ready'); setStatusText(`\u041d\u0410\u0419\u0414\u0415\u041d\u041e: ${filtered.length}`);
      log(`\u2705 \u0420\u0435\u0437\u0443\u043b\u044c\u0442\u0430\u0442: ${filtered.length} \u0437\u0430\u043f\u0438\u0441\u0435\u0439`, 'ok');
    } catch (err) {
      setPhase('ready'); setStatusText('\u041e\u0428\u0418\u0411\u041a\u0410'); log(`\u274c ${err}`, 'err');
    } finally { setScanning(false); }
  };

  const handleDownload = async (item, idx) => {
    const k = `dl_${idx}`; setActiveDownloads(p => ({ ...p, [k]: 'working' }));
    log(`⬇ ЗАГРУЗКА #${idx+1}...`, 'info');
    const taskId = `nem_${Date.now()}_${idx}`;
    try {
      const r = await invoke('download_isapi_playback_uri', {
        playbackUri: item.playbackUri,
        login: target.login || 'admin',
        pass: target.password || '',
        filenameHint: `${target.host.replace(/\./g,'_')}_cam${camera}_${idx}.mp4`,
        taskId,
      });
      setActiveDownloads(p => ({ ...p, [k]: 'done' }));
      log(`✅ ${r.filename} (${(r.bytesWritten/1048576).toFixed(1)} MB)`, 'ok');
    } catch (e) {
      try {
        const fallback = await invoke('capture_archive_segment', {
          sourceUrl: (item.playbackUri || '').replace(/&amp;/g, '&'),
          filenameHint: `${target.host.replace(/\./g,'_')}_cam${camera}_${idx}_fallback.mp4`,
          durationSeconds: 180,
          taskId,
        });
        setActiveDownloads(p => ({ ...p, [k]: 'done' }));
        log(`⚠ ISAPI отказал, fallback OK: ${fallback.filename} (${(fallback.bytesWritten/1048576).toFixed(1)} MB)`, 'warn');
      } catch (fallbackErr) {
        setActiveDownloads(p => ({ ...p, [k]: 'error' }));
        log(`❌ ${e} | fallback: ${fallbackErr}`, 'err');
      }
    }
  };

  const handleCapture = async (item) => {
    log('\ud83c\udfaf \u0417\u0410\u0425\u0412\u0410\u0422 \u0421\u0415\u0413\u041c\u0415\u041d\u0422\u0410...', 'info');
    try {
      const r = await invoke('capture_archive_segment', { sourceUrl: (item.playbackUri||'').replace(/&amp;/g,'&'), filenameHint: `capture_${Date.now()}.mp4`, durationSeconds: 120, taskId: `cap_${Date.now()}` });
      log(`\u2705 ${r.filename} (${(r.bytesWritten/1048576).toFixed(1)} MB)`, 'ok');
    } catch (e) { log(`\u274c ${e}`, 'err'); }
  };

  const chOpts = target.channels?.length
    ? target.channels.map(c => ({ v: String(c.index ?? c.id ?? '101'), l: c.name||`\u041a\u0430\u043d\u0430\u043b ${c.index ?? c.id}` }))
    : [{v:'101',l:'[A1] pod 1'},{v:'201',l:'[A2] pod 2'},{v:'301',l:'[A3] pod 3'},{v:'401',l:'[A4] pod 4'}];

  const pc = phase==='ready'?'#00ff9c':phase==='scanning'?'#00f0ff':phase==='error'?'#ff003c':'#ff9900';

  return (
    <div style={{ position:'fixed',inset:0,background:'rgba(0,0,0,.88)',backdropFilter:'blur(3px)',zIndex:10000,display:'flex',alignItems:'center',justifyContent:'center',animation:'nemIn .25s ease-out' }}>
      <style>{`
        @keyframes nemIn{from{opacity:0;transform:scale(.97)}to{opacity:1;transform:scale(1)}}
        @keyframes nemP{0%,100%{opacity:.5}50%{opacity:1}}
        @keyframes nemS{0%{top:-2px}100%{top:100%}}
        .ni{background:#000;color:#00f0ff;border:1px solid #1a2a33;padding:6px 8px;font:11px/1.3 Consolas,monospace;outline:none;box-sizing:border-box;width:100%}
        .ni:focus{border-color:#00f0ff;box-shadow:0 0 6px #00f0ff33}
        .ns{background:#000;color:#00f0ff;border:1px solid #1a2a33;padding:6px 8px;font:11px/1.3 Consolas,monospace;outline:none;box-sizing:border-box;width:100%;cursor:pointer}
        .nr{display:flex;align-items:center;gap:10px;padding:7px 14px;border-bottom:1px solid #0e0e12;transition:background .12s}
        .nr:hover{background:#0a1218}
        .nb{background:#0a1a0a;color:#00ff9c;border:1px solid #00ff9c44;padding:2px 8px;font:9px Consolas,monospace;cursor:pointer;transition:all .12s}
        .nb:hover{background:#00ff9c;color:#000}
        .nc{background:#0a0a1a;color:#00f0ff;border:1px solid #00f0ff44;padding:2px 8px;font:9px Consolas,monospace;cursor:pointer;transition:all .12s}
        .nc:hover{background:#00f0ff;color:#000}
      `}</style>

      <div style={{ width:880,maxHeight:'92vh',background:'#08080c',border:`1px solid ${pc}55`,boxShadow:`0 0 40px ${pc}15`,display:'flex',flexDirection:'column',overflow:'hidden',position:'relative' }}>
        {/* scanlines */}
        <div style={{ position:'absolute',inset:0,pointerEvents:'none',zIndex:1,background:'repeating-linear-gradient(0deg,transparent,transparent 3px,rgba(0,240,255,.008) 3px,rgba(0,240,255,.008) 4px)' }}/>
        <div style={{ position:'absolute',left:0,right:0,height:1,background:`linear-gradient(90deg,transparent,${pc}44,transparent)`,animation:'nemS 4s linear infinite',pointerEvents:'none',zIndex:1 }}/>

        {/* HEADER */}
        <div style={{ background:`linear-gradient(90deg,${pc}0a,transparent 30%,transparent 70%,${pc}0a)`,borderBottom:`1px solid ${pc}33`,padding:'10px 16px',display:'flex',justifyContent:'space-between',alignItems:'center',zIndex:2 }}>
          <div style={{ display:'flex',alignItems:'center',gap:10 }}>
            <div style={{ width:7,height:7,borderRadius:'50%',background:pc,boxShadow:`0 0 8px ${pc}`,animation:scanning?'nemP .6s infinite':'none' }}/>
            <div>
              <div style={{ color:pc,fontSize:12,fontWeight:'bold',fontFamily:'Consolas,monospace',letterSpacing:2 }}>{'\u2622'} ЗАГРУЗКА ИЗ СЕТИ</div>
              <div style={{ color:'#444',fontSize:9,fontFamily:'monospace',letterSpacing:1 }}>NEMESIS ARCHIVE TERMINAL // {target.host}:2019</div>
            </div>
          </div>
          <div style={{ display:'flex',alignItems:'center',gap:10 }}>
            <span style={{ color:pc,fontSize:10,fontFamily:'monospace' }}>{statusText}</span>
            <button onClick={onClose} style={{ background:'none',border:'1px solid #ff003c44',color:'#ff003c',padding:'3px 10px',cursor:'pointer',fontSize:11,fontFamily:'monospace',transition:'all .15s' }}
              onMouseEnter={e=>{e.target.style.background='#ff003c';e.target.style.color='#000'}}
              onMouseLeave={e=>{e.target.style.background='none';e.target.style.color='#ff003c'}}>{'\u2716'}</button>
          </div>
        </div>

        {/* TABS */}
        <div style={{ borderBottom:'1px solid #151518',display:'flex',zIndex:2 }}>
          <div style={{ padding:'7px 18px',fontSize:11,fontFamily:'monospace',color:'#ff003c',borderBottom:'2px solid #ff003c',cursor:'pointer',letterSpacing:1 }}>СКАЧАТЬ ПО ФАЙЛАМ</div>
          <div style={{ padding:'7px 18px',fontSize:11,fontFamily:'monospace',color:'#333',cursor:'pointer',letterSpacing:1 }}>СКАЧИВАТЬ ПО ДАТЕ</div>
        </div>

        {/* BODY */}
        <div style={{ display:'flex',flex:1,overflow:'hidden',zIndex:2 }}>

          {/* LEFT PANEL */}
          <div style={{ width:195,borderRight:'1px solid #151518',padding:'14px 12px',background:'#06060a',display:'flex',flexDirection:'column',gap:14,flexShrink:0 }}>
            <div style={{ color:'#555',fontSize:9,fontFamily:'monospace',letterSpacing:1,borderBottom:'1px solid #151518',paddingBottom:6 }}>УСЛОВИЕ ПОИСКА</div>

            <div>
              <label style={{ color:'#4a5a66',fontSize:9,fontFamily:'monospace',display:'block',marginBottom:3 }}>Камера</label>
              <select className="ns" value={camera} onChange={e=>setCamera(e.target.value)}>
                {chOpts.map(o=><option key={o.v} value={o.v}>{o.l}</option>)}
              </select>
            </div>

            <div>
              <label style={{ color:'#4a5a66',fontSize:9,fontFamily:'monospace',display:'block',marginBottom:3 }}>Тип файла</label>
              <select className="ns" value={fileType} onChange={e=>setFileType(e.target.value)}>
                <option value="all">Все</option>
                <option value="timing">По расписанию</option>
                <option value="alarm">Тревожные</option>
                <option value="manual">Вручную</option>
              </select>
            </div>

            <div>
              <label style={{ color:'#4a5a66',fontSize:9,fontFamily:'monospace',display:'block',marginBottom:3 }}>Тип потока:</label>
              <select className="ns" value={streamType} onChange={e=>setStreamType(e.target.value)}>
                <option value="main">Основной поток</option>
                <option value="sub">Дополнительный</option>
              </select>
            </div>

            <div>
              <label style={{ color:'#4a5a66',fontSize:9,fontFamily:'monospace',display:'block',marginBottom:3 }}>Время начала</label>
              <input className="ni" value={timeFrom} onChange={e=>setTimeFrom(e.target.value)}/>
            </div>

            <div>
              <label style={{ color:'#4a5a66',fontSize:9,fontFamily:'monospace',display:'block',marginBottom:3 }}>Время окончания</label>
              <input className="ni" value={timeTo} onChange={e=>setTimeTo(e.target.value)}/>
            </div>

            <button onClick={handleSearch} disabled={scanning||phase==='connecting'} style={{
              width:'100%',padding:'10px 0',marginTop:'auto',
              background:scanning?'#1a0a0a':'linear-gradient(180deg,#3a0a0a,#1a0505)',
              color:'#ff003c',border:'1px solid #ff003c',fontSize:12,fontFamily:'Consolas,monospace',
              fontWeight:'bold',cursor:scanning?'not-allowed':'pointer',letterSpacing:1,
              transition:'all .2s',opacity:scanning?.5:1
            }}
              onMouseEnter={e=>{if(!scanning){e.target.style.background='#ff003c';e.target.style.color='#000'}}}
              onMouseLeave={e=>{e.target.style.background='linear-gradient(180deg,#3a0a0a,#1a0505)';e.target.style.color='#ff003c'}}
            >{scanning?'\u25c9 \u041f\u041e\u0418\u0421\u041a...':'\ud83d\udd0e \u041f\u043e\u0438\u0441\u043a'}</button>
          </div>

          {/* RIGHT TABLE */}
          <div style={{ flex:1,display:'flex',flexDirection:'column',overflow:'hidden' }}>
            {/* toolbar */}
            <div style={{ display:'flex',alignItems:'center',justifyContent:'space-between',padding:'6px 14px',borderBottom:'1px solid #151518',background:'#0a0a0e' }}>
              <span style={{ color:'#555',fontSize:10,fontFamily:'monospace',letterSpacing:1 }}>СПИСОК ФАЙЛОВ</span>
              <button onClick={()=>records.filter(r=>r.playbackUri).forEach((r,i)=>handleDownload(r,i))} disabled={!records.filter(r=>r.playbackUri).length}
                style={{ background:'#0a1a0a',color:'#00ff9c',border:'1px solid #00ff9c55',padding:'4px 12px',fontSize:10,fontFamily:'monospace',cursor:'pointer',transition:'all .15s',opacity:records.filter(r=>r.playbackUri).length?1:.3 }}
                onMouseEnter={e=>{e.target.style.background='#00ff9c';e.target.style.color='#000'}}
                onMouseLeave={e=>{e.target.style.background='#0a1a0a';e.target.style.color='#00ff9c'}}
              >{'\u2b07'} Загрузка из сети</button>
            </div>

            {/* col headers */}
            <div style={{ display:'flex',alignItems:'center',padding:'5px 14px',borderBottom:'1px solid #1a1a1e',background:'#0c0c10',fontSize:9,fontFamily:'monospace',color:'#555',letterSpacing:1,flexShrink:0 }}>
              <div style={{ width:26 }}>{'\u2610'}</div>
              <div style={{ width:32 }}>№</div>
              <div style={{ flex:2 }}>Имя файла</div>
              <div style={{ flex:1 }}>Время начала</div>
              <div style={{ flex:1 }}>Время окончания</div>
              <div style={{ width:65,textAlign:'right' }}>Размер</div>
              <div style={{ width:100,textAlign:'center' }}>Прогресс</div>
            </div>

            {/* rows */}
            <div style={{ flex:1,overflowY:'auto' }}>
              {records.length===0&&!scanning&&(
                <div style={{ padding:40,textAlign:'center' }}>
                  <div style={{ color:'#1a1a22',fontSize:32,marginBottom:8 }}>{'\u2622'}</div>
                  <div style={{ color:'#333',fontSize:11,fontFamily:'monospace' }}>{phase==='connecting'?'УСТАНОВКА СВЯЗИ...':'Нажмите «Поиск» чтобы найти записи'}</div>
                </div>
              )}
              {scanning&&(
                <div style={{ padding:40,textAlign:'center' }}>
                  <div style={{ color:'#00f0ff',fontSize:16,animation:'nemP .8s infinite' }}>{'\u25c9'}</div>
                  <div style={{ color:'#00f0ff',fontSize:11,fontFamily:'monospace',marginTop:6 }}>ИДЁТ ПОИСК...</div>
                </div>
              )}
              {records.map((item,idx)=>{
                const k=`dl_${idx}`;const ds=activeDownloads[k];
                const fid=item.playbackUri?.match(/name=([^&]+)/)?.[1]||item.playbackUri?.match(/(\d{10,})/)?.[1]||`rec_${idx}`;
                return (
                  <div key={idx} className="nr">
                    <div style={{ width:26 }}><input type="checkbox" style={{ accentColor:'#00f0ff' }}/></div>
                    <div style={{ width:32,color:'#444',fontSize:10,fontFamily:'monospace' }}>{idx+1}</div>
                    <div style={{ flex:2,color:'#9fd7ff',fontSize:10,fontFamily:'monospace',overflow:'hidden',textOverflow:'ellipsis',whiteSpace:'nowrap' }}>{fid}</div>
                    <div style={{ flex:1,color:'#7fa9cb',fontSize:10,fontFamily:'monospace' }}>{item.startTime?.replace('T',' ').replace('Z','')||'\u2014'}</div>
                    <div style={{ flex:1,color:'#7fa9cb',fontSize:10,fontFamily:'monospace' }}>{item.endTime?.replace('T',' ').replace('Z','')||'\u2014'}</div>
                    <div style={{ width:65,textAlign:'right',color:'#ff9900',fontSize:10,fontFamily:'monospace' }}>\u2014</div>
                    <div style={{ width:100,display:'flex',gap:4,justifyContent:'center' }}>
                      {ds==='done'?<span style={{color:'#00ff9c',fontSize:9,fontFamily:'monospace'}}>{'\u2713'} OK</span>
                       :ds==='error'?<span style={{color:'#ff003c',fontSize:9,fontFamily:'monospace'}}>{'\u2716'} ERR</span>
                       :ds==='working'?<span style={{color:'#00f0ff',fontSize:9,fontFamily:'monospace',animation:'nemP .6s infinite'}}>{'\u25cc'} ...</span>
                       :item.playbackUri?<><button className="nb" onClick={()=>handleDownload(item,idx)}>{'\u2b07'}</button><button className="nc" onClick={()=>handleCapture(item)}>{'\u25c9'}</button></>
                       :<span style={{color:'#222',fontSize:9,fontFamily:'monospace'}}>\u2014</span>}
                    </div>
                  </div>
                );
              })}
            </div>

            {/* pagination */}
            <div style={{ borderTop:'1px solid #151518',padding:'5px 14px',background:'#0a0a0e',display:'flex',justifyContent:'flex-end',alignItems:'center',gap:6,fontSize:10,fontFamily:'monospace',color:'#555',flexShrink:0 }}>
              <span>Всего {records.length} Страница</span>
              <span style={{ color:'#00f0ff' }}>1/1</span>
            </div>
          </div>
        </div>

        {/* TERMINAL LOG */}
        <div style={{ borderTop:'1px solid #151518',background:'#040406',height:75,zIndex:2,display:'flex',flexDirection:'column' }}>
          <div style={{ padding:'3px 14px',borderBottom:'1px solid #0e0e12' }}>
            <span style={{ color:'#222',fontSize:8,fontFamily:'monospace',letterSpacing:1 }}>{'\u25b8'} ТЕРМИНАЛ</span>
          </div>
          <div ref={logRef} style={{ flex:1,overflowY:'auto',padding:'2px 14px' }}>
            {logs.map((l,i)=>(
              <div key={i} style={{ color:l.type==='err'?'#ff003c':l.type==='ok'?'#00ff9c':l.type==='warn'?'#ff9900':l.type==='sys'?'#b366ff':'#3a5a6a',fontSize:9,fontFamily:'Consolas,Courier New,monospace',lineHeight:1.6 }}>
                <span style={{ color:'#222' }}>[{l.ts}]</span> {l.msg}
              </div>
            ))}
          </div>
        </div>

        {/* FOOTER */}
        <div style={{ borderTop:'1px solid #0e0e12',padding:'3px 14px',background:'#06060a',display:'flex',justifyContent:'space-between',zIndex:2 }}>
          <span style={{ color:'#1a1a22',fontSize:8,fontFamily:'monospace' }}>HYPERION NEMESIS ENGINE v2.0</span>
          <span style={{ color:'#1a1a22',fontSize:8,fontFamily:'monospace' }}>{target.host}:2019</span>
        </div>
      </div>
    </div>
  );
}
