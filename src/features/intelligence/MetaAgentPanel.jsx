// src/features/intelligence/MetaAgentPanel.jsx
// ПОЛНАЯ ЗАМЕНА — скопировать целиком
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';


const S = {
  wrap: { border:'1px solid #2a1a4a', padding:'10px', backgroundColor:'#0a0415', marginBottom:'8px' },
  h: { color:'#b06fff', marginTop:0, fontSize:'0.85rem', letterSpacing:'0.08em' },
  input: { width:'100%', padding:'5px 8px', background:'#0a0a14', color:'#ccc',
           border:'1px solid #333', marginBottom:'6px', fontSize:'12px', boxSizing:'border-box' },
  btn: (c='#b06fff') => ({ width:'100%', padding:'6px', cursor:'pointer', fontWeight:'bold',
    fontSize:'12px', marginBottom:'4px', background:c+'22', color:c, border:'1px solid '+c+'55' }),
  log: { background:'#080810', border:'1px solid #2a1a4a', padding:'8px', fontSize:'10px',
         color:'#9b7fcf', maxHeight:'100px', overflowY:'auto', fontFamily:'monospace' },
  chip: (c) => ({ display:'inline-block', padding:'2px 8px', borderRadius:'10px',
    fontSize:'10px', fontWeight:600, background:c+'20', color:c, border:'1px solid '+c+'40', marginRight:'4px' }),
  bar: { height:'6px', background:'#1a1a2a', borderRadius:'3px', overflow:'hidden', margin:'4px 0' },
};


export default function MetaAgentPanel() {
  const [scope, setScope]     = useState('');
  const [permit, setPermit]   = useState('');
  const [iters, setIters]     = useState(3);
  const [ollamaUrl, setOllamaUrl] = useState('http://localhost:11434');
  const [llmModel, setLlmModel] = useState('llama3');
  const [llmTemp, setLlmTemp] = useState(0.4);
  const [running, setRunning] = useState(false);
  const [result, setResult]   = useState(null);
  const [stats, setStats]     = useState([]);
  const [knnRes, setKnn]      = useState(null);
  const [hyps, setHyps]       = useState([]);
  const [logs, setLogs]       = useState([]);
  const [tab, setTab]         = useState('run');


  const addLog = m => setLogs(p => ['['+new Date().toLocaleTimeString()+'] '+m, ...p].slice(0,40));


  // Загрузить статистику при монтировании
  useEffect(() => { loadStats(); }, []);


  const loadStats = async () => {
    try { setStats(await invoke('get_technique_stats')); }
    catch (e) { addLog('Stats error: '+e); }
  };


  const runCampaign = async () => {
    if (!scope.trim() || permit.trim().length < 8)
      return alert('Нужен scope и токен (мин 8 символов)');
    setRunning(true); setResult(null); setHyps([]);
    addLog('Старт кампании: scope='+scope+' iter='+iters);
    try {
      const r = await invoke('run_meta_campaign', {
        scope: scope.trim(),
        permitToken: permit.trim(),
        maxIterations: +iters,
      });
      setResult(r);
      addLog('Готово: '+r.totalFindings+' findings, '+Math.round(r.successRate*100)+'% успех');
      r.decisionsMade?.forEach(d =>
        addLog('['+d.action+'] eps='+d.epsilon?.toFixed(2)+' knn='+d.similarTargetsFound)
      );
      await loadStats();
    } catch(e) { addLog('Ошибка: '+e); }
    setRunning(false);
  };


  const findSimilar = async () => {
    if (!scope.trim()) return alert('Введите scope');
    try {
      const res = await invoke('context_find_similar', {
        vendor: scope.includes('hik') ? 'Hikvision' : scope.includes('dah') ? 'Dahua' : 'unknown',
        ports: [80, 443, 554, 8080],
        firmware: 'unknown',
        k: 5,
      });
      setKnn(res);
      addLog('k-NN: найдено похожих='+res.similarCount+' sim='+res.avgSimilarity?.toFixed(2));
    } catch(e) { addLog('k-NN ошибка: '+e); }
  };


  const getHypotheses = async () => {
    if (!scope.trim()) return alert('Введите scope');
    setHyps([]);
    addLog('Запрос гипотез у LLM...');
    try {
      const vendor = scope.includes('hik') ? 'Hikvision' : scope.includes('dah') ? 'Dahua' : 'unknown';
      const res = await invoke('llm_generate_hypotheses', {
        req: {
          vendor,
          firmware: 'unknown',
          openPorts: [80, 443, 554],
          alreadyFailed: stats.filter(s => s.failCount > s.successCount).map(s => s.technique),
          config: { ollamaUrl, model: llmModel, temperature: Number(llmTemp) },
        }
      });
      setHyps(res);
      addLog('Получено '+res.length+' гипотез от LLM');
    } catch(e) { addLog('LLM недоступен: '+e+' — используем дефолтные'); }
  };


  const resetMem = async () => {
    if (!confirm('Сбросить всю память кампаний?')) return;
    try { await invoke('reset_campaign_memory'); setStats([]); setKnn(null); addLog('Память сброшена'); }
    catch(e) { addLog('Ошибка: '+e); }
  };


  const TABS = [['run','▶ Кампания'],['stats','📊 Статистика'],['knn','🔬 k-NN'],['hyp','💡 Гипотезы']];


  return (
    <div style={S.wrap}>
      <h3 style={S.h}>🧠 МЕТА-АГЕНТ — Самообучение</h3>


      {/* Таб-навигация */}
      <div style={{ display:'flex', gap:'3px', marginBottom:'8px' }}>
        {TABS.map(([id,label]) => (
          <button key={id} onClick={()=>setTab(id)} style={{
            flex:1, padding:'4px', fontSize:'10px', cursor:'pointer',
            background: tab===id ? '#0a2a3a' : 'transparent',
            color: tab===id ? '#00ccff' : '#555',
            border: '1px solid '+(tab===id?'#1a6a6a':'#1a1a2a'),
          }}>{label}</button>
        ))}
      </div>

      <div style={{ background:'#0a0818', border:'1px solid #2a1a4a', padding:'8px', marginBottom:'8px' }}>
        <div style={{ fontSize:'10px', color:'#7777aa', marginBottom:'4px' }}>LLM-конфигурация для гипотез</div>
        <input style={S.input} value={ollamaUrl} onChange={e=>setOllamaUrl(e.target.value)} placeholder='Ollama URL' />
        <div style={{ display:'flex', gap:'6px' }}>
          <select style={{ ...S.input, flex:1, marginBottom:0 }} value={llmModel} onChange={e=>setLlmModel(e.target.value)}>
            <option value='llama3'>llama3</option>
            <option value='mistral'>mistral</option>
            <option value='phi3'>phi3</option>
            <option value='qwen2.5'>qwen2.5</option>
          </select>
          <input style={{ ...S.input, flex:1, marginBottom:0 }} type='number' min='0' max='1' step='0.1' value={llmTemp} onChange={e=>setLlmTemp(e.target.value)} placeholder='Temp' />
        </div>
      </div>


      {/* ── Вкладка: Кампания ── */}
      {tab==='run' && <>
        <input style={S.input} value={scope} onChange={e=>setScope(e.target.value)} placeholder='Цель: IP или домен' />
        <input style={S.input} value={permit} type='password' onChange={e=>setPermit(e.target.value)} placeholder='Разрешительный токен (мин 8)' />
        <div style={{ display:'flex', gap:'6px', marginBottom:'6px' }}>
          <div style={{ flex:1 }}>
            <div style={{ fontSize:'10px', color:'#555', marginBottom:'2px' }}>Итерации (1-10)</div>
            <input style={{ ...S.input, marginBottom:0 }} type='number' min='1' max='10' value={iters} onChange={e=>setIters(e.target.value)} />
          </div>
          <button style={{ ...S.btn('#b06fff'), flex:1, height:'auto', marginBottom:0 }} onClick={runCampaign} disabled={running}>
            {running ? '⚙ Работает...' : '▶ Запустить'}
          </button>
        </div>


        {result && (
          <div style={{ background:'#0a0818', border:'1px solid #3a2a5a', padding:'8px', marginBottom:'6px', fontSize:'11px' }}>
            <div style={{ color:'#c0a0ff', fontWeight:'bold', marginBottom:'4px' }}>
              Итерация {result.iteration} завершена
            </div>
            <span style={S.chip('#b06fff')}>Итераций: {result.iteration}</span>
            <span style={S.chip('#00ffcc')}>Findings: {result.totalFindings}</span>
            <span style={S.chip('#ffaa00')}>Успех: {Math.round(result.successRate*100)}%</span>
            <div style={{ marginTop:'8px' }}>
              {result.decisionsMade?.map((d,i)=>(
                <div key={i} style={{ fontSize:'10px', color:'#8870af', borderBottom:'1px solid #1a1a2e', padding:'3px 0' }}>
                  [{d.action}] eps={d.epsilon?.toFixed(2)} knn={d.similarTargetsFound}
                  <span style={{ color:'#555', marginLeft:'6px' }}>{d.reasoning?.slice(0,50)}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </>}


      {/* ── Вкладка: Статистика ── */}
      {tab==='stats' && <>
        <button style={S.btn('#9b7fcf')} onClick={loadStats}>🔄 Обновить</button>
        {stats.length===0
          ? <div style={{ fontSize:'11px', color:'#333', textAlign:'center', padding:'12px' }}>Данных нет — запусти кампанию</div>
          : <div style={{ maxHeight:'180px', overflowY:'auto' }}>
              {[...stats].sort((a,b)=>b.avgReward-a.avgReward).map((t,i)=>{
                const total = t.successCount+t.failCount;
                const rate = total>0 ? Math.round(t.successCount/total*100) : 0;
                const rColor = t.avgReward>1.0?'#00ffcc':t.avgReward>0.5?'#ffaa00':'#ff5555';
                return <div key={i} style={{ fontSize:'10px', borderBottom:'1px solid #1a1a2e', padding:'4px 0' }}>
                  <div style={{ display:'flex', justifyContent:'space-between' }}>
                    <span style={{ color:'#9b7fcf' }}>{t.technique}</span>
                    <span>
                      <span style={{ color:rColor }}>R:{t.avgReward?.toFixed(2)}</span>
                      <span style={{ color:'#555', marginLeft:'8px' }}>{rate}% ({total})</span>
                    </span>
                  </div>
                  <div style={S.bar}>
                    <div style={{ height:'100%', width:rate+'%', background:rColor, transition:'width .5s' }} />
                  </div>
                </div>;
              })}
            </div>
        }
        <button style={{ ...S.btn('#ff5555'), marginTop:'4px' }} onClick={resetMem}>🗑 Сбросить память</button>
      </>}


      {/* ── Вкладка: k-NN ── */}
      {tab==='knn' && <>
        <input style={S.input} value={scope} onChange={e=>setScope(e.target.value)} placeholder='Scope для поиска похожих' />
        <button style={S.btn('#00aaff')} onClick={findSimilar}>🔬 Найти похожие устройства</button>
        {knnRes && (
          <div style={{ background:'#05101a', border:'1px solid #1a3a5a', padding:'8px', fontSize:'11px' }}>
            <div style={{ color:'#00aaff', fontWeight:'bold', marginBottom:'6px' }}>
              Найдено похожих: {knnRes.similarCount}
            </div>
            <div style={{ fontSize:'10px', color:'#6666aa', marginBottom:'4px' }}>
              Ближайший вендор: {knnRes.topMatchVendor || 'неизвестен'} · Сходство: {(knnRes.avgSimilarity*100).toFixed(0)}%
            </div>
            {knnRes.recommendedTechniques?.length>0 && <>
              <div style={{ fontSize:'10px', color:'#4a7a9a', marginBottom:'4px' }}>Рекомендованные техники:</div>
              <div style={{ display:'flex', flexWrap:'wrap', gap:'4px' }}>
                {knnRes.recommendedTechniques.map((t,i)=>(
                  <span key={i} style={S.chip('#00aaff')}>{t}</span>
                ))}
              </div>
            </>}
          </div>
        )}
      </>}


      {/* ── Вкладка: Гипотезы ── */}
      {tab==='hyp' && <>
        <input style={S.input} value={scope} onChange={e=>setScope(e.target.value)} placeholder='Scope для генерации гипотез' />
        <button style={S.btn('#00ffcc')} onClick={getHypotheses}>💡 Сгенерировать гипотезы (LLM)</button>
        {hyps.length>0 && (
          <div style={{ maxHeight:'200px', overflowY:'auto' }}>
            {hyps.map((h,i)=>(
              <div key={i} style={{ background:'#050f0f', border:'1px solid #1a4a4a', padding:'7px', marginBottom:'4px' }}>
                <div style={{ display:'flex', justifyContent:'space-between', marginBottom:'3px' }}>
                  <span style={{ color:'#00ffcc', fontWeight:'bold', fontSize:'11px' }}>{h.technique}</span>
                  <span style={{ fontSize:'10px', color:'#00ffcc' }}>
                    P={Math.round(h.expectedProbability*100)}% · Стелс:{h.stealthLevel}/5
                  </span>
                </div>
                <div style={{ fontSize:'10px', color:'#4a9a9a', lineHeight:1.4 }}>{h.description}</div>
                {h.requiredConditions?.length>0 && (
                  <div style={{ fontSize:'9px', color:'#2a5a5a', marginTop:'2px' }}>
                    Условия: {h.requiredConditions.join(', ')}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </>}


      {/* Лог */}
      <div style={S.log}>
        {logs.length===0
          ? <span style={{ color:'#2a2a4a' }}>Лог пустой</span>
          : logs.map((l,i)=><div key={i}>{l}</div>)
        }
      </div>
    </div>
  );
}
