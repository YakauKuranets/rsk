import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

const wrapStyle = {
  border: '1px solid #4a3b1d',
  borderRadius: '8px',
  padding: '12px',
  background: '#161106',
  color: '#ffe7b3',
};

const inputStyle = {
  width: '100%',
  boxSizing: 'border-box',
  padding: '7px 8px',
  borderRadius: '4px',
  border: '1px solid #6f5622',
  background: '#1b1507',
  color: '#ffe7b3',
};

const buttonStyle = {
  width: '100%',
  marginTop: '8px',
  padding: '8px',
  borderRadius: '4px',
  border: '1px solid #9c7a30',
  background: '#2a210b',
  color: '#ffc857',
  fontWeight: 700,
  cursor: 'pointer',
};

export default function BASPanel() {
  const [target, setTarget] = useState('demo.local');
  const [permitToken, setPermitToken] = useState('');
  const [scenarios, setScenarios] = useState([]);
  const [selectedIds, setSelectedIds] = useState([]);
  const [report, setReport] = useState(null);
  const [status, setStatus] = useState('Загрузка BAS-сценариев...');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;

    invoke('list_bas_scenarios')
      .then((items) => {
        if (cancelled) return;
        const normalized = Array.isArray(items) ? items : [];
        setScenarios(normalized);
        setSelectedIds(normalized.map((item) => item.id));
        setStatus(normalized.length > 0 ? `Загружено ${normalized.length} BAS-сценариев.` : 'BAS-сценарии не найдены.');
      })
      .catch((error) => {
        if (cancelled) return;
        setScenarios([]);
        setSelectedIds([]);
        setStatus(`Не удалось загрузить BAS-сценарии: ${error}`);
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const selectedScenarios = useMemo(
    () => scenarios.filter((scenario) => selectedIds.includes(scenario.id)),
    [scenarios, selectedIds],
  );

  const toggleScenario = (scenarioId) => {
    setSelectedIds((current) => (
      current.includes(scenarioId)
        ? current.filter((id) => id !== scenarioId)
        : [...current, scenarioId]
    ));
  };

  const runSimulation = async () => {
    const normalizedTarget = target.trim();
    if (!normalizedTarget) {
      setStatus('Укажите хост/IP для BAS-проверки.');
      return;
    }
    if (permitToken.trim().length < 8) {
      setStatus('Нужен разрешительный токен длиной не менее 8 символов.');
      return;
    }

    setLoading(true);
    setReport(null);
    setStatus('Запуск BAS-симуляции...');

    try {
      const result = await invoke('run_bas_simulation', {
        targets: [normalizedTarget],
        scenarioIds: selectedIds,
        permitToken: permitToken.trim(),
      });
      setReport(result);
      setStatus(`BAS завершён: coverage ${Number(result.coverageScore || 0).toFixed(1)}%, gaps ${result.topGaps?.length || 0}.`);
    } catch (error) {
      setReport(null);
      setStatus(`BAS недоступен: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <section style={wrapStyle}>
      <h3 style={{ margin: '0 0 10px', fontSize: '13px', color: '#ffc857', textTransform: 'uppercase', letterSpacing: '0.08em' }}>
        BAS Panel
      </h3>
      <input
        value={target}
        onChange={(e) => setTarget(e.target.value)}
        placeholder="Simulation target"
        style={inputStyle}
      />
      <input
        value={permitToken}
        onChange={(e) => setPermitToken(e.target.value)}
        placeholder="Permit token"
        type="password"
        style={{ ...inputStyle, marginTop: '8px' }}
      />

      <div style={{ marginTop: '10px', fontSize: '11px', color: '#d9bb73' }}>Сценарии:</div>
      <div style={{ marginTop: '6px', display: 'grid', gap: '6px', maxHeight: '170px', overflowY: 'auto' }}>
        {scenarios.length > 0 ? scenarios.map((scenario) => (
          <label
            key={scenario.id}
            style={{
              display: 'grid',
              gap: '3px',
              padding: '8px',
              borderRadius: '6px',
              border: '1px solid #5a4520',
              background: selectedIds.includes(scenario.id) ? '#241b09' : '#130d04',
              cursor: 'pointer',
            }}
          >
            <span style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
              <input
                type="checkbox"
                checked={selectedIds.includes(scenario.id)}
                onChange={() => toggleScenario(scenario.id)}
              />
              <span style={{ fontSize: '11px', fontWeight: 700, color: '#ffd37b' }}>{scenario.name}</span>
            </span>
            <span style={{ fontSize: '10px', color: '#caa55d' }}>
              {scenario.id} · {scenario.tactic} · {scenario.techniqueId}
            </span>
            <span style={{ fontSize: '10px', color: '#b8964f' }}>{scenario.description}</span>
          </label>
        )) : <div style={{ fontSize: '11px', color: '#8f7641' }}>Список сценариев пуст.</div>}
      </div>

      <button type="button" onClick={runSimulation} disabled={loading} style={{ ...buttonStyle, opacity: loading ? 0.7 : 1 }}>
        {loading ? '⚙ Выполняю BAS...' : '⚔ Запустить BAS'}
      </button>

      <div style={{ marginTop: '10px', fontSize: '11px', color: '#d9bb73' }}>{status}</div>

      {report && (
        <div style={{ marginTop: '10px', display: 'grid', gap: '8px' }}>
          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', fontSize: '10px' }}>
            <span style={{ border: '1px solid #6f5622', borderRadius: '10px', padding: '2px 8px' }}>Всего: {report.totalScenarios}</span>
            <span style={{ border: '1px solid #3c7c3c', borderRadius: '10px', padding: '2px 8px', color: '#8bf08b' }}>Blocked: {report.blocked}</span>
            <span style={{ border: '1px solid #4c6f9a', borderRadius: '10px', padding: '2px 8px', color: '#8bc7ff' }}>Detected: {report.detected}</span>
            <span style={{ border: '1px solid #9a4c4c', borderRadius: '10px', padding: '2px 8px', color: '#ff9a9a' }}>Bypassed: {report.bypassed}</span>
            <span style={{ border: '1px solid #9c7a30', borderRadius: '10px', padding: '2px 8px' }}>Coverage: {Number(report.coverageScore || 0).toFixed(1)}%</span>
          </div>

          {report.topGaps?.length > 0 && (
            <div style={{ fontSize: '10px', color: '#ffbf75' }}>
              <b>Top gaps:</b> {report.topGaps.join(', ')}
            </div>
          )}

          <div style={{ display: 'grid', gap: '6px', maxHeight: '220px', overflowY: 'auto' }}>
            {report.results?.map((item, idx) => (
              <div key={`${item.scenarioId}-${idx}`} style={{ border: '1px solid #4f3d18', borderRadius: '6px', padding: '8px', background: '#120d05' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px', marginBottom: '4px', fontSize: '11px' }}>
                  <span style={{ color: '#ffd37b', fontWeight: 700 }}>{item.scenarioId}</span>
                  <span style={{ color: item.status === 'bypassed' ? '#ff8d8d' : item.status === 'detected' ? '#8bc7ff' : '#8bf08b' }}>
                    {item.status}
                  </span>
                </div>
                <div style={{ fontSize: '10px', color: '#d1b170', marginBottom: '3px' }}>{item.mitreId} · risk {Number(item.riskScore || 0).toFixed(1)}</div>
                <div style={{ fontSize: '10px', color: '#c39a4d', marginBottom: '3px' }}>{item.evidence}</div>
                <div style={{ fontSize: '10px', color: '#8f7641' }}>{item.remediation}</div>
              </div>
            ))}
          </div>

          <div style={{ fontSize: '10px', color: '#a78b4c' }}>
            Выбрано сценариев: {selectedScenarios.length}
          </div>
        </div>
      )}
    </section>
  );
}
