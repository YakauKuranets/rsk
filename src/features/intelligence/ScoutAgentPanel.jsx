import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

const S = {
  wrap: { border: '1px solid #1f3d5a', borderRadius: '8px', padding: '12px', background: '#07111a', color: '#d9ebff' },
  title: { margin: '0 0 10px', fontSize: '13px', color: '#78d3ff', textTransform: 'uppercase', letterSpacing: '0.08em' },
  input: { width: '100%', boxSizing: 'border-box', padding: '7px 8px', background: '#081521', color: '#d9ebff', border: '1px solid #214460', borderRadius: '4px', marginBottom: '8px' },
  btn: { width: '100%', padding: '8px', background: '#0d2940', color: '#78d3ff', border: '1px solid #2d668b', borderRadius: '4px', cursor: 'pointer', fontWeight: 700 },
  mono: { fontFamily: 'monospace', fontSize: '11px', background: '#04101a', border: '1px solid #173145', borderRadius: '6px', padding: '8px', whiteSpace: 'pre-wrap' },
};

export default function ScoutAgentPanel() {
  const [scope, setScope] = useState('demo.local');
  const [status, setStatus] = useState('Idle');
  const [packet, setPacket] = useState(null);

  const startScout = async () => {
    setStatus('Running scout agent…');
    try {
      const res = await invoke('start_scout_agent', { scope: scope.trim() || 'demo.local' });
      setPacket(res);
      setStatus('Scout agent started successfully.');
    } catch (error) {
      setStatus(`Scout agent unavailable: ${error}`);
    }
  };

  return (
    <section style={S.wrap}>
      <h3 style={S.title}>Scout Agent</h3>
      <input style={S.input} value={scope} onChange={(e) => setScope(e.target.value)} placeholder="Target scope" />
      <button type="button" style={S.btn} onClick={startScout}>▶ Start scout</button>
      <div style={{ marginTop: '10px', fontSize: '11px', color: '#8eb9d6' }}>{status}</div>
      <div style={{ marginTop: '10px', ...S.mono }}>{packet ? JSON.stringify(packet, null, 2) : 'No scout packet yet.'}</div>
    </section>
  );
}
