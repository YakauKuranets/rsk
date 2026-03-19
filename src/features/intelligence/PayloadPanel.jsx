import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function PayloadPanel() {
  const [type, setType] = useState('hta');
  const [lure, setLure] = useState('Monthly camera maintenance notice');
  const [output, setOutput] = useState('');

  const generate = async () => {
    try {
      const res = type === 'hta'
        ? await invoke('generate_hta_payload', { prompt: lure })
        : await invoke('generate_macro_lure', { prompt: lure });
      setOutput(typeof res === 'string' ? res : JSON.stringify(res, null, 2));
    } catch (error) {
      setOutput(`Payload generator unavailable: ${error}`);
    }
  };

  return (
    <section style={{ border: '1px solid #532225', borderRadius: '8px', padding: '12px', background: '#18090a', color: '#ffd6d9' }}>
      <h3 style={{ margin: '0 0 10px', fontSize: '13px', color: '#ff8f9a', textTransform: 'uppercase', letterSpacing: '0.08em' }}>Payload Panel</h3>
      <select value={type} onChange={(e) => setType(e.target.value)} style={{ width: '100%', padding: '7px 8px', borderRadius: '4px', background: '#210b0d', color: '#ffd6d9', border: '1px solid #6e2d33' }}>
        <option value="hta">HTA payload</option>
        <option value="macro">Macro lure</option>
      </select>
      <textarea rows={4} value={lure} onChange={(e) => setLure(e.target.value)} style={{ width: '100%', boxSizing: 'border-box', marginTop: '8px', padding: '7px 8px', borderRadius: '4px', background: '#210b0d', color: '#ffd6d9', border: '1px solid #6e2d33', resize: 'vertical' }} />
      <button type="button" onClick={generate} style={{ width: '100%', marginTop: '8px', padding: '8px', borderRadius: '4px', border: '1px solid #a84d56', background: '#361418', color: '#ff8f9a', fontWeight: 700, cursor: 'pointer' }}>🧰 Generate payload</button>
      <pre style={{ marginTop: '10px', padding: '8px', borderRadius: '6px', border: '1px solid #6e2d33', background: '#120607', color: '#ffc0c5', whiteSpace: 'pre-wrap', fontSize: '11px', fontFamily: 'monospace' }}>{output || 'No payload generated yet.'}</pre>
    </section>
  );
}
