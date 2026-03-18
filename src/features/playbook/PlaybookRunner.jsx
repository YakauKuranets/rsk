import React, { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import PlaybookLibrary from './PlaybookLibrary';

const icon = (status) => ({ pending: '⚪', waitingApproval: '🟡', running: '🔵', completed: '✅', failed: '❌', skipped: '➖', cancelled: '✖️' }[status] || '⚪');

export default function PlaybookRunner() {
  const [yaml, setYaml] = useState('');
  const [status, setStatus] = useState({ steps: [], status: 'idle' });

  const waiting = useMemo(() => (status.steps || []).find((s) => s.status === 'waitingApproval'), [status]);

  const refresh = async () => {
    try { setStatus(await invoke('get_playbook_status')); } catch {}
  };

  useEffect(() => { const t = setInterval(refresh, 2000); return () => clearInterval(t); }, []);

  return (
    <div style={{ border: '1px solid #333', padding: 10, marginBottom: 10 }}>
      <PlaybookLibrary onSelect={setYaml} />
      <textarea value={yaml} onChange={(e) => setYaml(e.target.value)} rows={12} style={{ width: '100%', background: '#090909', color: '#d0faff', fontFamily: 'monospace' }} />
      <button onClick={async () => setStatus(await invoke('start_playbook', { yamlContent: yaml }))} style={{ width: '100%', marginTop: 8, background: '#102a2f', color: '#00f0ff', border: '1px solid #00f0ff', padding: 8 }}>ЗАПУСТИТЬ СЦЕНАРИЙ</button>

      <div style={{ marginTop: 10 }}>
        {(status.steps || []).map((s) => (
          <div key={`${s.stepId}_${s.status}`} style={{ borderBottom: '1px solid #222', padding: '6px 0', color: '#ddd' }}>
            {icon(s.status)} {s.stepName} <span style={{ color: '#888' }}>({s.module})</span>
            {s.status === 'completed' && <pre style={{ whiteSpace: 'pre-wrap', fontSize: 10 }}>{JSON.stringify(s.output, null, 2)}</pre>}
          </div>
        ))}
      </div>

      {waiting && (
        <div style={{ marginTop: 10, border: '1px solid #665500', background: '#221d08', padding: 8 }}>
          <div>Этот шаг выполнит <b>{waiting.module}</b>. Подтвердить?</div>
          <button onClick={async () => setStatus(await invoke('approve_playbook_step', { stepId: waiting.stepId, approved: true }))}>ПОДТВЕРДИТЬ</button>
          <button onClick={async () => setStatus(await invoke('approve_playbook_step', { stepId: waiting.stepId, approved: false }))}>ПРОПУСТИТЬ</button>
        </div>
      )}
    </div>
  );
}
