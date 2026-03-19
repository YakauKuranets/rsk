import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function BASPanel() {
  const [target, setTarget] = useState('demo.local');
  const [plan, setPlan] = useState([]);
  const [status, setStatus] = useState('Ready');

  const runSimulation = async () => {
    setStatus('Requesting BAS execution plan…');
    try {
      const res = await invoke('llm_generate_attack_plan', {
        targetProfile: `BAS simulation target: ${target}`,
        config: { ollamaUrl: 'http://localhost:11434', model: 'llama3', temperature: 0.2 },
      });
      setPlan(Array.isArray(res) ? res : []);
      setStatus(`Loaded ${Array.isArray(res) ? res.length : 0} BAS steps.`);
    } catch (error) {
      setPlan([]);
      setStatus(`BAS planning unavailable: ${error}`);
    }
  };

  return (
    <section style={{ border: '1px solid #4a3b1d', borderRadius: '8px', padding: '12px', background: '#161106', color: '#ffe7b3' }}>
      <h3 style={{ margin: '0 0 10px', fontSize: '13px', color: '#ffc857', textTransform: 'uppercase', letterSpacing: '0.08em' }}>BAS Panel</h3>
      <input value={target} onChange={(e) => setTarget(e.target.value)} placeholder="Simulation target" style={{ width: '100%', boxSizing: 'border-box', padding: '7px 8px', borderRadius: '4px', border: '1px solid #6f5622', background: '#1b1507', color: '#ffe7b3' }} />
      <button type="button" onClick={runSimulation} style={{ width: '100%', marginTop: '8px', padding: '8px', borderRadius: '4px', border: '1px solid #9c7a30', background: '#2a210b', color: '#ffc857', fontWeight: 700, cursor: 'pointer' }}>⚔ Build BAS plan</button>
      <div style={{ marginTop: '10px', fontSize: '11px', color: '#d9bb73' }}>{status}</div>
      <ol style={{ margin: '10px 0 0', paddingLeft: '18px', fontSize: '11px', color: '#f2d99a' }}>
        {plan.length > 0 ? plan.map((step, idx) => <li key={idx} style={{ marginBottom: '4px' }}>{step}</li>) : <li>No BAS plan generated yet.</li>}
      </ol>
    </section>
  );
}
