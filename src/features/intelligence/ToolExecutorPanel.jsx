import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../store/appStore';
import { verifySessionCookieFlagsCapability } from '../../api/capabilities';

const S={
  wrap:{border:'1px solid #2a2a2a',padding:'10px',backgroundColor:'#0a0a0a',marginBottom:'8px'},
  h:{color:'#cccccc',marginTop:0,fontSize:'0.85rem',letterSpacing:'0.08em'},
  inp:{width:'100%',padding:'5px 8px',background:'#0a0a0a',color:'#ccc',border:'1px solid #2a2a2a',marginBottom:'6px',fontSize:'12px',boxSizing:'border-box'},
  btn:(c='#cccccc')=>({width:'100%',padding:'6px',cursor:'pointer',fontWeight:'bold',fontSize:'12px',marginBottom:'4px',background:c+'22',color:c,border:'1px solid '+c+'55'}),
};

const TOOLS=['nmap','nikto','nuclei','hydra','sqlmap','amass','gobuster','masscan','ffuf'];
const PRESETS={nmap:'-sV -sC -p 80,443,554',nikto:'-h',nuclei:'-t cves/',hydra:'-l admin -P wordlist.txt http-get',sqlmap:'-u',amass:'enum -passive -d',masscan:'-p 80,443,554 --rate 1000'};
const RUN_PROFILES={
  nmap:[
    {id:'fast',label:'Быстрый',args:'-T4 -F -sV'},
    {id:'base',label:'Базовый',args:'-sV -sC -p 80,443,554'},
    {id:'careful',label:'Осторожный',args:'-T2 -sV -sC -p 80,443,554 --max-retries 1'},
  ],
  nikto:[
    {id:'fast',label:'Быстрый',args:'-h'},
    {id:'full',label:'Полный',args:'-h -C all -Tuning x'},
  ],
  ffuf:[
    {id:'base',label:'Базовый',args:'-w wordlist.txt -u http://TARGET/FUZZ -mc 200,204,301,302'},
    {id:'careful',label:'Аккуратный',args:'-w wordlist.txt -u http://TARGET/FUZZ -mc all -rate 25 -timeout 10'},
  ],
  masscan:[
    {id:'limited',label:'Ограниченный',args:'-p 80,443,554 --rate 500'},
    {id:'careful',label:'Осторожный',args:'-p 80,443 --rate 200 --wait 5'},
  ],
  default:[
    {id:'base',label:'Базовый',args:''},
    {id:'safe',label:'Аккуратный',args:'--help'},
  ],
};
const QUICK_SCENARIOS = [
  { id: 'net_fast', label: 'Быстрая сетевая проверка', tool: 'nmap', profileId: 'fast', kind: 'network' },
  { id: 'net_careful', label: 'Осторожная сетевая проверка', tool: 'nmap', profileId: 'careful', kind: 'network' },
  { id: 'web_fast', label: 'Быстрая web-проверка', tool: 'nikto', profileId: 'fast', kind: 'web' },
  { id: 'web_paths', label: 'Проверка web-путей', tool: 'ffuf', profileId: 'base', kind: 'web' },
];
const TOOL_EXEC_STATE_KEY = 'hyperion_tool_executor_state_v1';
const TOOL_EXEC_FAVORITE_SCENARIOS_KEY = 'hyperion_tool_executor_favorite_scenarios_v1';
const TOOL_EXEC_RECENT_RUNS_KEY = 'hyperion_tool_executor_recent_runs_v1';
const TOOL_EXEC_RECENT_RUNS_LIMIT = 6;
const TOOL_EXEC_USER_SCENARIOS_KEY = 'hyperion_tool_executor_user_scenarios_v1';
const TOOL_EXEC_USER_SCENARIOS_LIMIT = 10;

function getRecommendedToolPlan(selectedTarget) {
  if (!selectedTarget) return null;
  const text = `${selectedTarget?.type || ''} ${selectedTarget?.name || ''} ${selectedTarget?.host || ''} ${selectedTarget?.ip || ''}`.toLowerCase();
  const looksCamera = /(cam|camera|nvr|dvr|rtsp|554|onvif|hub)/.test(text);
  const looksWeb = /(http|https|www|web|:80|:443|\.[a-z]{2,})/.test(text);
  if (looksCamera) {
    return {
      tool: 'nmap',
      profileId: 'careful',
      reason: 'Похоже на камеру/NVR/HUB: начни с осторожного сетевого профиля.',
    };
  }
  if (looksWeb) {
    return {
      tool: 'nikto',
      profileId: 'fast',
      reason: 'Похоже на web-цель: сначала быстрый web-аудит.',
    };
  }
  return {
    tool: 'nmap',
    profileId: 'base',
    reason: 'Тип цели неочевиден: начни с базового сетевого профиля.',
  };
}

function buildSemanticSummary(tool, result) {
  if (!result) return 'Нет данных результата.';
  const findings = Array.isArray(result?.findingsExtracted) ? result.findingsExtracted : [];
  const mergedOutput = `${result?.stdout || ''}\n${result?.stderr || ''}`;
  const openPorts = mergedOutput.match(/\b(\d{1,5})\/(tcp|udp)\s+open\b/gi) || [];
  const webSignals = mergedOutput.match(/(vuln|xss|sql|csrf|interesting|directory|admin|login|exposed)/gi) || [];
  const pathSignals = mergedOutput.match(/\/[a-z0-9._\-\/]+/gi) || [];

  if (tool === 'nmap') {
    return openPorts.length > 0
      ? `Есть признаки открытых портов/сервисов (${openPorts.length}).`
      : 'Явных открытых портов по текущему выводу не видно.';
  }
  if (tool === 'nikto') {
    return (findings.length > 0 || webSignals.length > 0)
      ? 'Есть web-находки, нужно проверить вручную.'
      : 'Критичных web-находок по текущему запуску не найдено.';
  }
  if (tool === 'ffuf') {
    return (findings.length > 0 || pathSignals.length > 0)
      ? 'Найдены интересные пути/эндпоинты.'
      : 'Интересные пути по текущему словарю не обнаружены.';
  }
  if (tool === 'masscan') {
    return openPorts.length > 0
      ? `Есть признаки открытых портов (${openPorts.length}).`
      : 'Открытые порты по текущему запуску masscan не зафиксированы.';
  }
  if (findings.length > 0) return `Найдены артефакты (${findings.length}), см. детали ниже.`;
  return 'Нейтральная сводка: данных для уверенного вывода мало.';
}

function inferTargetHintKind(selectedTarget) {
  if (!selectedTarget) return 'unknown';
  const text = `${selectedTarget?.type || ''} ${selectedTarget?.name || ''} ${selectedTarget?.host || ''} ${selectedTarget?.ip || ''}`.toLowerCase();
  const looksCamera = /(cam|camera|nvr|dvr|rtsp|554|onvif|hik|xmeye|ipcam)/.test(text);
  const looksWeb = /(http|https|www|web|:80|:443|site|portal)/.test(text);
  if (looksCamera) return 'camera';
  if (looksWeb) return 'web';
  return 'generic';
}

function inferScenarioHintKind(tool, args = '', fallbackKind = null) {
  if (fallbackKind) return fallbackKind;
  const t = String(tool || '').toLowerCase();
  const a = String(args || '').toLowerCase();
  if (t === 'nikto' || t === 'ffuf' || t === 'sqlmap' || t === 'gobuster') return 'web';
  if (t === 'nmap' || t === 'masscan') {
    if (/(554|rtsp|onvif|camera|nvr|dvr)/.test(a)) return 'camera';
    return 'network';
  }
  return 'generic';
}

function buildScenarioCompatibilityHint(scenarioKind, targetKind) {
  if (targetKind === 'unknown') return { text: 'Сомнительно', color: '#9a8f79' };
  if (scenarioKind === 'web') {
    if (targetKind === 'web') return { text: 'Подходит', color: '#76c893' };
    return { text: 'Лучше для web-цели', color: '#d4a373' };
  }
  if (scenarioKind === 'camera') {
    if (targetKind === 'camera') return { text: 'Подходит', color: '#76c893' };
    return { text: 'Лучше для камеры/NVR', color: '#d4a373' };
  }
  if (scenarioKind === 'network') {
    if (targetKind === 'generic' || targetKind === 'camera') return { text: 'Подходит', color: '#76c893' };
    return { text: 'Сомнительно', color: '#9a8f79' };
  }
  return { text: 'Сомнительно', color: '#9a8f79' };
}

export default function ToolExecutorPanel({ onSessionAuditStatus, selectedTarget }){
  const intelligenceTarget = useAppStore((s)=>s.intelligenceTarget);
  const setIntelligenceTarget = useAppStore((s)=>s.setIntelligenceTarget);
  const permit = useAppStore((s)=>s.permitToken);
  const setPerm = useAppStore((s)=>s.setPermitToken);
  const [tool,setTool]=useState('nmap');
  const [args,setArgs]=useState('-sV -sC -p 80,443,554');
  const [timeout,setTo]=useState(120);
  const [argsByTool, setArgsByTool] = useState({});
  const [selectedPresetByTool, setSelectedPresetByTool] = useState({});
  const [load,setLoad]=useState(false);
  const [result,setResult]=useState(null);
  const [avail,setAvail]=useState([]);
  const [sessionResult, setSessionResult] = useState('');
  const [sessionDebug, setSessionDebug] = useState(null);
  const [favoriteScenarioIds, setFavoriteScenarioIds] = useState([]);
  const [recentRuns, setRecentRuns] = useState([]);
  const [userScenarios, setUserScenarios] = useState([]);
  const selectedTargetLabel = selectedTarget
    ? (selectedTarget.name || selectedTarget.host || selectedTarget.id || 'Без имени')
    : '';
  const selectedTargetEndpoint = selectedTarget?.host || selectedTarget?.ip || '';
  const recommendedPlan = getRecommendedToolPlan(selectedTarget);
  const semanticSummary = buildSemanticSummary(tool, result);
  const selectedTargetHintKind = inferTargetHintKind(selectedTarget);
  const normalizedArgs = String(args || '').trim();
  const profiles = RUN_PROFILES[tool] || RUN_PROFILES.default;
  const activePresetId = selectedPresetByTool?.[tool] || null;
  const hasTemplatePlaceholders = /(^|\b)(TARGET|FUZZ|example\.com)(\b|$)/i.test(normalizedArgs);
  const launchReadiness = !intelligenceTarget.trim()
    ? { level: 'error', text: 'Нужно указать цель', canRun: false }
    : permit.trim().length < 8
      ? { level: 'error', text: 'Нужен токен', canRun: false }
      : !normalizedArgs
        ? { level: 'error', text: 'Проверь аргументы', canRun: false }
        : hasTemplatePlaceholders
          ? { level: 'warn', text: 'Проверь аргументы', canRun: true }
          : { level: 'ok', text: 'Готово к запуску', canRun: true };

  useEffect(() => {
    try {
      const raw = localStorage.getItem(TOOL_EXEC_STATE_KEY);
      if (!raw) return;
      const saved = JSON.parse(raw);
      const savedTool = TOOLS.includes(saved?.tool) ? saved.tool : 'nmap';
      const savedTimeout = Number(saved?.timeout);
      const nextArgsByTool = saved?.argsByTool && typeof saved.argsByTool === 'object' ? saved.argsByTool : {};
      const nextPresetByTool = saved?.selectedPresetByTool && typeof saved.selectedPresetByTool === 'object' ? saved.selectedPresetByTool : {};
      setArgsByTool(nextArgsByTool);
      setSelectedPresetByTool(nextPresetByTool);
      setTool(savedTool);
      if (Number.isFinite(savedTimeout) && savedTimeout > 0) setTo(savedTimeout);
      setArgs(nextArgsByTool[savedTool] || PRESETS[savedTool] || '');
    } catch {}
  }, []);

  useEffect(() => {
    try {
      const raw = localStorage.getItem(TOOL_EXEC_USER_SCENARIOS_KEY);
      if (!raw) return;
      const saved = JSON.parse(raw);
      if (!Array.isArray(saved)) return;
      setUserScenarios(saved.filter((item) => item && typeof item === 'object').slice(0, TOOL_EXEC_USER_SCENARIOS_LIMIT));
    } catch {}
  }, []);

  useEffect(() => {
    try {
      const raw = localStorage.getItem(TOOL_EXEC_RECENT_RUNS_KEY);
      if (!raw) return;
      const saved = JSON.parse(raw);
      if (!Array.isArray(saved)) return;
      setRecentRuns(saved.filter((item) => item && typeof item === 'object').slice(0, TOOL_EXEC_RECENT_RUNS_LIMIT));
    } catch {}
  }, []);

  useEffect(() => {
    try {
      const raw = localStorage.getItem(TOOL_EXEC_FAVORITE_SCENARIOS_KEY);
      if (!raw) return;
      const saved = JSON.parse(raw);
      if (!Array.isArray(saved)) return;
      const allowed = new Set(QUICK_SCENARIOS.map((s) => s.id));
      setFavoriteScenarioIds(saved.filter((id) => typeof id === 'string' && allowed.has(id)));
    } catch {}
  }, []);

  useEffect(() => {
    setArgsByTool((prev) => ({ ...prev, [tool]: args }));
  }, [tool, args]);

  useEffect(() => {
    try {
      localStorage.setItem(TOOL_EXEC_STATE_KEY, JSON.stringify({
        tool,
        timeout,
        argsByTool,
        selectedPresetByTool,
      }));
    } catch {}
  }, [tool, timeout, argsByTool, selectedPresetByTool]);

  useEffect(() => {
    try {
      localStorage.setItem(TOOL_EXEC_FAVORITE_SCENARIOS_KEY, JSON.stringify(favoriteScenarioIds));
    } catch {}
  }, [favoriteScenarioIds]);

  useEffect(() => {
    try {
      localStorage.setItem(TOOL_EXEC_RECENT_RUNS_KEY, JSON.stringify(recentRuns.slice(0, TOOL_EXEC_RECENT_RUNS_LIMIT)));
    } catch {}
  }, [recentRuns]);

  useEffect(() => {
    try {
      localStorage.setItem(TOOL_EXEC_USER_SCENARIOS_KEY, JSON.stringify(userScenarios.slice(0, TOOL_EXEC_USER_SCENARIOS_LIMIT)));
    } catch {}
  }, [userScenarios]);

  const appendRecentRun = (entry) => {
    if (!entry) return;
    setRecentRuns((prev) => {
      const filtered = prev.filter((item) => item?.id !== entry.id);
      return [entry, ...filtered].slice(0, TOOL_EXEC_RECENT_RUNS_LIMIT);
    });
  };

  const executeToolLaunch = async ({ tool: runTool, target: runTarget, args: runArgs, profileId: runProfileId }) => {
    setLoad(true);
    setResult(null);
    try {
      const response = await invoke('execute_tool', {
        req: {
          tool: runTool,
          target: String(runTarget || '').trim(),
          args: String(runArgs || '').trim().split(/\s+/).filter(Boolean),
          timeoutSecs: +timeout,
          permitToken: permit.trim(),
        },
      });
      setResult(response);
      return true;
    } catch (e) {
      alert('Ошибка: ' + e);
      return false;
    } finally {
      appendRecentRun({
        id: `${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
        tool: runTool,
        target: String(runTarget || '').trim(),
        args: String(runArgs || '').trim(),
        profileId: runProfileId || null,
        executedAt: new Date().toISOString(),
      });
      setLoad(false);
    }
  };

  const run = async () => {
    if (!launchReadiness.canRun) return alert(launchReadiness.text);
    if (launchReadiness.level === 'warn') {
      const proceed = window.confirm('⚠ Обнаружены шаблонные заглушки в аргументах (TARGET/FUZZ/example.com). Запустить всё равно?');
      if (!proceed) return;
    }
    await executeToolLaunch({
      tool,
      target: intelligenceTarget,
      args,
      profileId: activePresetId || null,
    });
  };

  const runSessionCapability = async () => {
    const target = intelligenceTarget.trim();
    if (!target) return alert('Введите цель');
    setSessionResult('Проверка сессионных флагов...');
    setSessionDebug(null);
    const session = await verifySessionCookieFlagsCapability(target, 'discovery_mode');
    if (typeof onSessionAuditStatus === 'function') {
      onSessionAuditStatus({
        mode: session?.inconclusive ? 'inconclusive' : session?.fallbackUsed ? 'fallback' : 'primary',
        source: session?.source || null,
        updatedAt: Date.now(),
      });
    }
    setSessionDebug({
      source: session?.source || null,
      fallbackUsed: typeof session?.fallbackUsed === 'boolean' ? session.fallbackUsed : null,
      inconclusive: typeof session?.inconclusive === 'boolean' ? session.inconclusive : null,
      runId: session?.runId || null,
      issuesCount: typeof session?.issuesCount === 'number' ? session.issuesCount : null,
      reporterSummary: session?.reporterSummary || null,
      evidenceRefsCount: Array.isArray(session?.evidenceRefs) ? session.evidenceRefs.length : null,
    });
    if (!session.ok) {
      setSessionResult(`Ошибка проверки: ${session.message || 'unknown error'}`);
      return;
    }
    if (session.secure) {
      setSessionResult(`✅ Session cookie flags выглядят безопасно (${target})`);
    } else {
      setSessionResult(`⚠️ Найдены проблемы: ${(session.issues || []).join(' | ')}`);
    }
  };

  const copyText = async (text, okLabel) => {
    const payload = String(text || '');
    if (!payload.trim()) return alert('Копировать нечего');
    try {
      await navigator.clipboard.writeText(payload);
      alert(okLabel);
    } catch {
      alert('Не удалось скопировать');
    }
  };

  const applyRecommendation = () => {
    if (!recommendedPlan) return;
    const nextTool = recommendedPlan.tool;
    const nextProfiles = RUN_PROFILES[nextTool] || RUN_PROFILES.default;
    const profile = nextProfiles.find((p) => p.id === recommendedPlan.profileId) || nextProfiles[0] || { id: 'base', args: PRESETS[nextTool] || '' };
    setTool(nextTool);
    setArgs(profile.args || PRESETS[nextTool] || '');
    setSelectedPresetByTool((prev) => ({ ...prev, [nextTool]: profile.id }));
    if (selectedTargetEndpoint) setIntelligenceTarget(selectedTargetEndpoint);
  };

  const applyQuickScenario = (scenario) => {
    if (!scenario) return;
    const nextTool = scenario.tool;
    const nextProfiles = RUN_PROFILES[nextTool] || RUN_PROFILES.default;
    const profile = nextProfiles.find((p) => p.id === scenario.profileId) || nextProfiles[0] || { id: 'base', args: PRESETS[nextTool] || '' };
    setTool(nextTool);
    setArgs(profile.args || PRESETS[nextTool] || '');
    setSelectedPresetByTool((prev) => ({ ...prev, [nextTool]: profile.id }));
    if (!selectedTargetEndpoint) return;
    if (scenario.kind === 'web' && !/^https?:\/\//i.test(selectedTargetEndpoint)) {
      setIntelligenceTarget(`http://${selectedTargetEndpoint}`);
      return;
    }
    setIntelligenceTarget(selectedTargetEndpoint);
  };

  const toggleFavoriteScenario = (scenarioId) => {
    if (!scenarioId) return;
    setFavoriteScenarioIds((prev) => (
      prev.includes(scenarioId)
        ? prev.filter((id) => id !== scenarioId)
        : [...prev, scenarioId]
    ));
  };

  const favoriteScenarios = QUICK_SCENARIOS.filter((scenario) => favoriteScenarioIds.includes(scenario.id));
  const getScenarioCompatibility = (scenario) => {
    const scenarioKind = inferScenarioHintKind(scenario?.tool, scenario?.args, scenario?.kind || null);
    return buildScenarioCompatibilityHint(scenarioKind, selectedTargetHintKind);
  };
  const formatRecentRunTime = (iso) => {
    if (!iso) return 'время неизвестно';
    const parsed = new Date(iso);
    if (Number.isNaN(parsed.getTime())) return 'время неизвестно';
    return parsed.toLocaleString('ru-RU', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' });
  };
  const applyRecentRun = (entry) => {
    if (!entry) return;
    if (entry.tool) setTool(entry.tool);
    if (typeof entry.args === 'string') setArgs(entry.args);
    if (entry.target) setIntelligenceTarget(entry.target);
    if (entry.tool && entry.profileId) {
      setSelectedPresetByTool((prev) => ({ ...prev, [entry.tool]: entry.profileId }));
    }
  };
  const repeatRecentRun = async (entry) => {
    if (!entry) return;
    if (!permit.trim()) return alert('Для повторного запуска нужен разрешительный токен');
    applyRecentRun(entry);
    await executeToolLaunch({
      tool: entry.tool || tool,
      target: entry.target || intelligenceTarget,
      args: entry.args || args,
      profileId: entry.profileId || null,
    });
  };
  const applyRecentRunToSelectedTarget = async (entry) => {
    if (!entry) return;
    if (!selectedTargetEndpoint) return alert('Нет выбранной цели');
    if (!permit.trim()) return alert('Для запуска нужен разрешительный токен');
    if (entry.tool) setTool(entry.tool);
    if (typeof entry.args === 'string') setArgs(entry.args);
    setIntelligenceTarget(selectedTargetEndpoint);
    if (entry.tool && entry.profileId) {
      setSelectedPresetByTool((prev) => ({ ...prev, [entry.tool]: entry.profileId }));
    }
    await executeToolLaunch({
      tool: entry.tool || tool,
      target: selectedTargetEndpoint,
      args: entry.args || args,
      profileId: entry.profileId || null,
    });
  };
  const applyUserScenario = (scenario) => {
    if (!scenario) return;
    if (scenario.tool) setTool(scenario.tool);
    if (typeof scenario.args === 'string') setArgs(scenario.args);
    if (scenario.tool && scenario.profileId) {
      setSelectedPresetByTool((prev) => ({ ...prev, [scenario.tool]: scenario.profileId }));
    }
    if (selectedTargetEndpoint) setIntelligenceTarget(selectedTargetEndpoint);
  };
  const saveRecentRunAsUserScenario = (entry) => {
    if (!entry) return;
    const scenario = {
      id: `${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
      title: `${entry.tool || 'tool'}${entry.profileId ? ` · ${entry.profileId}` : ''} · мой сценарий`,
      tool: entry.tool || tool,
      args: typeof entry.args === 'string' ? entry.args : '',
      profileId: entry.profileId || null,
      createdAt: new Date().toISOString(),
    };
    setUserScenarios((prev) => {
      const dedup = prev.filter((item) => !(item.tool === scenario.tool && item.args === scenario.args && item.profileId === scenario.profileId));
      return [scenario, ...dedup].slice(0, TOOL_EXEC_USER_SCENARIOS_LIMIT);
    });
  };
  const deleteUserScenario = (scenarioId) => {
    if (!scenarioId) return;
    const confirmed = window.confirm('Удалить пользовательский сценарий?');
    if (!confirmed) return;
    setUserScenarios((prev) => prev.filter((item) => item.id !== scenarioId));
  };
  const renameUserScenario = (scenarioId) => {
    if (!scenarioId) return;
    const current = userScenarios.find((item) => item.id === scenarioId);
    if (!current) return;
    const nextTitle = window.prompt('Новое название сценария', current.title || '');
    if (nextTitle == null) return;
    const normalized = String(nextTitle).trim();
    if (!normalized) return alert('Название не может быть пустым');
    setUserScenarios((prev) => prev.map((item) => (item.id === scenarioId ? { ...item, title: normalized } : item)));
  };
  const pinUserScenarioToTop = (scenarioId) => {
    if (!scenarioId) return;
    setUserScenarios((prev) => {
      const index = prev.findIndex((item) => item.id === scenarioId);
      if (index <= 0) return prev;
      const next = [...prev];
      const [picked] = next.splice(index, 1);
      next.unshift(picked);
      return next;
    });
  };

  return(
    <div style={S.wrap}>
      <h3 style={S.h}>🔧 ИНСТРУМЕНТЫ (UNIFIED API)</h3>
      <div style={{background:'#09111b',border:'1px solid #233247',padding:'6px',marginBottom:'6px',fontSize:'10px',borderRadius:'3px'}}>
        <div style={{color:'#7f93a4',marginBottom:'4px'}}>
          {selectedTarget
            ? <>Выбрана цель: <b style={{color:'#a9bfd1'}}>{selectedTargetLabel}</b></>
            : 'Выбранная цель: не выбрана'}
        </div>
        <button
          style={{...S.btn('#66b3ff'),marginBottom:0,fontSize:'10px'}}
          onClick={() => {
            if (!selectedTargetEndpoint) return;
            setIntelligenceTarget(selectedTargetEndpoint);
          }}
          disabled={!selectedTargetEndpoint}
        >
          Подставить выбранную цель
        </button>
      </div>
      <div style={{background:'#101218',border:'1px solid #2a3240',padding:'6px',marginBottom:'6px',fontSize:'10px',borderRadius:'3px'}}>
        <div style={{color:'#7f93a4',marginBottom:'4px'}}>Рекомендация по запуску</div>
        {recommendedPlan
          ? (
            <>
              <div style={{color:'#9fc6d5',marginBottom:'4px'}}>
                Инструмент: <b>{recommendedPlan.tool}</b> · Профиль: <b>{recommendedPlan.profileId}</b>
              </div>
              <div style={{color:'#768aa0',marginBottom:'6px'}}>{recommendedPlan.reason}</div>
              <button style={{...S.btn('#7bc3ff'),marginBottom:0,fontSize:'10px'}} onClick={applyRecommendation}>
                Применить рекомендацию
              </button>
            </>
          )
          : <div style={{color:'#768aa0'}}>Рекомендация недоступна: цель не выбрана.</div>}
      </div>
      <div style={{background:'#10141b',border:'1px solid #2a3342',padding:'6px',marginBottom:'6px',fontSize:'10px',borderRadius:'3px'}}>
        <div style={{color:'#7f93a4',marginBottom:'4px'}}>Избранные сценарии</div>
        {favoriteScenarios.length > 0 ? (
          <div style={{display:'flex',gap:'4px',flexWrap:'wrap',marginBottom:'4px'}}>
            {favoriteScenarios.map((scenario)=>(
              <div key={`fav_${scenario.id}`} style={{display:'flex',gap:'2px',alignItems:'center'}}>
                <button
                  style={{...S.btn('#92cfff'),width:'auto',padding:'4px 8px',marginBottom:0,fontSize:'10px'}}
                  onClick={()=>applyQuickScenario(scenario)}
                  title='Быстрый запуск избранного сценария'
                >
                  ★ {scenario.label}
                </button>
                <span style={{fontSize:'9px',color:getScenarioCompatibility(scenario).color}}>
                  {getScenarioCompatibility(scenario).text}
                </span>
              </div>
            ))}
          </div>
        ) : (
          <div style={{color:'#6f8398',marginBottom:'4px'}}>Нет избранных сценариев. Отметь звёздочкой нужные ниже.</div>
        )}
        <div style={{color:'#7f93a4',marginBottom:'4px'}}>Быстрые рабочие сценарии</div>
        <div style={{display:'flex',gap:'4px',flexWrap:'wrap'}}>
          {QUICK_SCENARIOS.map((scenario)=>(
            <div key={scenario.id} style={{display:'flex',gap:'2px'}}>
              <button
                style={{...S.btn('#7bb7ff'),width:'auto',padding:'4px 8px',marginBottom:0,fontSize:'10px'}}
                onClick={()=>applyQuickScenario(scenario)}
                title={scenario.kind === 'web' ? 'Web-сценарий: цель будет подставлена как http://...' : 'Общий сетевой сценарий'}
              >
                {scenario.label}
              </button>
              <button
                style={{...S.btn(favoriteScenarioIds.includes(scenario.id) ? '#ffd27d' : '#6f7f94'),width:'auto',padding:'4px 6px',marginBottom:0,fontSize:'10px'}}
                onClick={()=>toggleFavoriteScenario(scenario.id)}
                title={favoriteScenarioIds.includes(scenario.id) ? 'Убрать из избранного' : 'Добавить в избранное'}
              >
                {favoriteScenarioIds.includes(scenario.id) ? '★' : '☆'}
              </button>
              <span style={{fontSize:'9px',color:getScenarioCompatibility(scenario).color,alignSelf:'center'}}>
                {getScenarioCompatibility(scenario).text}
              </span>
            </div>
          ))}
        </div>
        <div style={{marginTop:'4px',color:'#6f8398'}}>
          Один клик: инструмент + профиль + цель (если выбрана). Затем можно вручную скорректировать args/цель.
        </div>
      </div>
      <div style={{background:'#10131a',border:'1px solid #2a3342',padding:'6px',marginBottom:'6px',fontSize:'10px',borderRadius:'3px'}}>
        <div style={{color:'#7f93a4',marginBottom:'4px'}}>Пользовательские сценарии</div>
        {userScenarios.length === 0 ? (
          <div style={{color:'#6f8398'}}>Пока пусто. Сохрани удачный запуск из блока «Недавние запуски».</div>
        ) : (
          <div style={{display:'flex',gap:'4px',flexWrap:'wrap'}}>
            {userScenarios.map((scenario) => (
              <div key={scenario.id} style={{display:'flex',gap:'2px'}}>
                <button
                  style={{...S.btn('#9cb8ff'),width:'auto',padding:'4px 8px',marginBottom:0,fontSize:'10px'}}
                  onClick={() => applyUserScenario(scenario)}
                  title='Применить сценарий (инструмент + args + профиль + текущая выбранная цель при наличии)'
                >
                  {scenario.title}
                </button>
                <button
                  style={{...S.btn('#8ea5bf'),width:'auto',padding:'4px 6px',marginBottom:0,fontSize:'10px'}}
                  onClick={() => pinUserScenarioToTop(scenario.id)}
                  title='Закрепить/поднять вверх'
                >
                  Вверх
                </button>
                <button
                  style={{...S.btn('#d0b67a'),width:'auto',padding:'4px 6px',marginBottom:0,fontSize:'10px'}}
                  onClick={() => renameUserScenario(scenario.id)}
                  title='Переименовать'
                >
                  Имя
                </button>
                <button
                  style={{...S.btn('#d98f8f'),width:'auto',padding:'4px 6px',marginBottom:0,fontSize:'10px'}}
                  onClick={() => deleteUserScenario(scenario.id)}
                  title='Удалить сценарий'
                >
                  Удалить
                </button>
                <span style={{fontSize:'9px',color:getScenarioCompatibility(scenario).color,alignSelf:'center'}}>
                  {getScenarioCompatibility(scenario).text}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
      <div style={{background:'#101318',border:'1px solid #2b3442',padding:'6px',marginBottom:'6px',fontSize:'10px',borderRadius:'3px'}}>
        <div style={{color:'#7f93a4',marginBottom:'4px'}}>Недавние запуски</div>
        {recentRuns.length === 0 ? (
          <div style={{color:'#6f8398'}}>Пока пусто. Запусти инструмент — здесь появится история.</div>
        ) : (
          <div style={{display:'flex',flexDirection:'column',gap:'4px'}}>
            {recentRuns.map((entry) => (
              <div key={entry.id} style={{border:'1px solid #263142',background:'#0d1219',padding:'5px',borderRadius:'3px'}}>
                <div style={{color:'#9fc6d5',marginBottom:'3px'}}>
                  <b>{entry.tool || 'tool'}</b> · {entry.target || 'без цели'} · {formatRecentRunTime(entry.executedAt)}
                  {entry.profileId ? <> · профиль: <b>{entry.profileId}</b></> : null}
                </div>
                <div style={{color:'#7d91a5',fontFamily:'monospace',marginBottom:'4px',whiteSpace:'pre-wrap',wordBreak:'break-word'}}>
                  {entry.args || '(без аргументов)'}
                </div>
                <div style={{display:'flex',gap:'4px',flexWrap:'wrap'}}>
                  <button style={{...S.btn('#7bb7ff'),width:'auto',padding:'4px 8px',marginBottom:0,fontSize:'10px'}} onClick={()=>repeatRecentRun(entry)}>
                    Повторить
                  </button>
                  <button style={{...S.btn('#8ea5bf'),width:'auto',padding:'4px 8px',marginBottom:0,fontSize:'10px'}} onClick={()=>applyRecentRun(entry)}>
                    Подставить
                  </button>
                  <button
                    style={{...S.btn('#7fcf9b'),width:'auto',padding:'4px 8px',marginBottom:0,fontSize:'10px'}}
                    onClick={()=>applyRecentRunToSelectedTarget(entry)}
                    disabled={!selectedTargetEndpoint}
                    title={selectedTargetEndpoint ? 'Запустить этот набор на текущей выбранной цели' : 'Сначала выберите цель'}
                  >
                    На текущую цель
                  </button>
                  <button
                    style={{...S.btn('#d0b67a'),width:'auto',padding:'4px 8px',marginBottom:0,fontSize:'10px'}}
                    onClick={()=>saveRecentRunAsUserScenario(entry)}
                    title='Сохранить этот запуск как пользовательский сценарий'
                  >
                    Сохранить как сценарий
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      <div style={{display:'flex',gap:'4px',flexWrap:'wrap',marginBottom:'8px'}}>
        {TOOLS.map(t=>(
          <button key={t} onClick={()=>{setTool(t);setArgs(argsByTool[t] || PRESETS[t] || '');}}
            style={{padding:'3px 8px',background:tool===t?'#1a1a1a':'transparent',
              color:tool===t?'#eee':'#555',border:'1px solid '+(tool===t?'#555':'#1a1a1a'),
              cursor:'pointer',fontSize:'10px',borderRadius:'3px'}}>{t}</button>
        ))}
      </div>
      <div style={{background:'#0d1219',border:'1px solid #202f42',padding:'6px',marginBottom:'6px',borderRadius:'3px'}}>
        <div style={{fontSize:'10px',color:'#7f93a4',marginBottom:'4px'}}>Профиль запуска ({tool})</div>
        <div style={{display:'flex',gap:'4px',flexWrap:'wrap'}}>
          {profiles.map((p)=>(
            <button
              key={`${tool}_${p.id}`}
              style={{...S.btn(activePresetId===p.id?'#66b3ff':'#8bb8ff'),width:'auto',padding:'4px 8px',marginBottom:0,fontSize:'10px'}}
              onClick={()=>{
                setArgs(p.args);
                setSelectedPresetByTool((prev)=>({ ...prev, [tool]: p.id }));
              }}
            >
              {p.label}
            </button>
          ))}
        </div>
      </div>
      <div style={{
        border:'1px solid ' + (launchReadiness.level === 'error' ? '#7a2a2a' : launchReadiness.level === 'warn' ? '#6c5a24' : '#245a3c'),
        background: launchReadiness.level === 'error' ? '#1a0b0b' : launchReadiness.level === 'warn' ? '#17130a' : '#0b1710',
        color: launchReadiness.level === 'error' ? '#ff9b9b' : launchReadiness.level === 'warn' ? '#ffd27d' : '#9fe0b7',
        padding:'6px',marginBottom:'6px',fontSize:'10px',borderRadius:'3px',
      }}>
        Статус запуска: <b>{launchReadiness.text}</b>
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
          <button
            style={{...S.btn('#44cc88'),flex:1,marginBottom:0,fontSize:'10px'}}
            onClick={runSessionCapability}
          >
            Аудит сессии через capability
          </button>
        </div>
      </div>
      {sessionResult && (
        <div style={{background:'#09111b',border:'1px solid #24404e',padding:'6px',marginBottom:'6px',fontSize:'10px',color:'#9fc6d5',borderRadius:'3px'}}>
          {sessionResult}
        </div>
      )}
      {sessionDebug && (
        <div style={{background:'#0f1318',border:'1px solid #2f3d4a',padding:'6px',marginBottom:'6px',fontSize:'9px',color:'#8ea3b6',borderRadius:'3px'}}>
          <div style={{marginBottom:'4px',color:'#7f93a4'}}>Отладка сессии</div>
          {sessionDebug.source && <div>Источник: <b style={{color:'#a9bfd1'}}>{sessionDebug.source}</b></div>}
          {typeof sessionDebug.fallbackUsed === 'boolean' && <div>Использован fallback: <b style={{color:'#a9bfd1'}}>{String(sessionDebug.fallbackUsed)}</b></div>}
          {typeof sessionDebug.inconclusive === 'boolean' && <div>Неопределённый результат: <b style={{color:'#a9bfd1'}}>{String(sessionDebug.inconclusive)}</b></div>}
          {sessionDebug.runId && <div>ID запуска: <b style={{color:'#a9bfd1'}}>{sessionDebug.runId}</b></div>}
          {typeof sessionDebug.issuesCount === 'number' && <div>Количество проблем: <b style={{color:'#a9bfd1'}}>{sessionDebug.issuesCount}</b></div>}
          {typeof sessionDebug.evidenceRefsCount === 'number' && <div>Ссылок на доказательства: <b style={{color:'#a9bfd1'}}>{sessionDebug.evidenceRefsCount}</b></div>}
          {sessionDebug.reporterSummary && <div style={{marginTop:'4px'}}>Сводка отчёта: <span style={{color:'#a9bfd1'}}>{sessionDebug.reporterSummary}</span></div>}
        </div>
      )}
      {avail.length>0&&<div style={{display:'flex',flexWrap:'wrap',gap:'4px',margin:'6px 0'}}>
        {avail.map(t=><span key={t.tool} style={{fontSize:'9px',background:(t.available?'#00aa44':'#aa3333')+'20',color:t.available?'#00aa44':'#aa3333',border:'1px solid '+(t.available?'#00aa44':'#aa3333')+'40',padding:'2px 6px',borderRadius:'8px'}}>{t.tool}: {t.available?'✓':'✗'}</span>)}
      </div>}
      {result&&<div style={{marginTop:'6px'}}>
        <div style={{display:'flex',gap:'8px',fontSize:'10px',color:'#666',marginBottom:'4px'}}>
          <span>Код: <b style={{color:result.exitCode===0?'#00aa44':'#ff4444'}}>{result.exitCode}</b></span>
          <span>Время: <b style={{color:'#aaa'}}>{((result.durationMs||0)/1000).toFixed(1)}с</b></span>
          <span>Найдено: <b style={{color:result.findingsExtracted?.length>0?'#ffaa00':'#444'}}>{result.findingsExtracted?.length||0}</b></span>
        </div>
        <div style={{fontSize:'10px',color:result.exitCode===0?'#8fd3a5':'#ff9b9b',marginBottom:'4px'}}>
          {result.exitCode===0
            ? (result.findingsExtracted?.length>0 ? 'Запуск завершён успешно, есть находки.' : 'Запуск завершён успешно, явных находок нет.')
            : 'Запуск завершился с ошибкой. Проверь параметры и вывод ниже.'}
        </div>
        <div style={{border:'1px solid #2b3b4d',background:'#0d1520',padding:'6px',borderRadius:'3px',marginBottom:'4px'}}>
          <div style={{fontSize:'10px',color:'#8eb6d7',marginBottom:'3px'}}>Смысловая сводка</div>
          <div style={{fontSize:'10px',color:'#9fb8cd'}}>{semanticSummary}</div>
        </div>
        <div style={{display:'flex',gap:'4px',marginBottom:'4px',flexWrap:'wrap'}}>
          <button style={{...S.btn('#66b3ff'),width:'auto',padding:'4px 8px',marginBottom:0,fontSize:'10px'}} onClick={()=>copyText((result.stdout||'').slice(0,2000)||(result.stderr||'').slice(0,500),'Вывод скопирован')}>
            Скопировать вывод
          </button>
          <button style={{...S.btn('#88ddaa'),width:'auto',padding:'4px 8px',marginBottom:0,fontSize:'10px'}} onClick={()=>copyText((result.findingsExtracted||[]).join('\n'),'Находки скопированы')}>
            Скопировать находки
          </button>
          <button style={{...S.btn('#ffaa66'),width:'auto',padding:'4px 8px',marginBottom:0,fontSize:'10px'}} onClick={run}>
            Повторить запуск
          </button>
        </div>
        {result.findingsExtracted?.length>0&&<div style={{background:'#0a1205',border:'1px solid #1a3a1a',padding:'6px',marginBottom:'4px',fontSize:'10px',color:'#00aa44',maxHeight:'80px',overflowY:'auto',fontFamily:'monospace',borderRadius:'3px'}}>
          {result.findingsExtracted.map((f,i)=><div key={i}>{f}</div>)}</div>}
        <div style={{background:'#0a0a0a',border:'1px solid #1a1a1a',padding:'6px',fontSize:'10px',color:'#666',maxHeight:'100px',overflowY:'auto',fontFamily:'monospace',whiteSpace:'pre-wrap',wordBreak:'break-all',borderRadius:'3px'}}>
          {(result.stdout||'').slice(0,2000)||(result.stderr||'').slice(0,500)}</div>
      </div>}
    </div>
  );
}
