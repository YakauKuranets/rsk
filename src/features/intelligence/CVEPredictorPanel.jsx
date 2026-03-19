import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

const S = {
  wrap: { border: '1px solid #4c2a55', borderRadius: '8px', padding: '12px', background: '#120816', color: '#f0d9ff' },
  title: { margin: '0 0 10px', fontSize: '13px', color: '#d39cff', textTransform: 'uppercase', letterSpacing: '0.08em' },
  input: { width: '100%', boxSizing: 'border-box', padding: '7px 8px', background: '#160b1d', color: '#f0d9ff', border: '1px solid #5a3169', borderRadius: '4px', marginBottom: '8px' },
  btn: { width: '100%', padding: '8px', background: '#2c1238', color: '#d39cff', border: '1px solid #854da0', borderRadius: '4px', cursor: 'pointer', fontWeight: 700 },
};

export default function CVEPredictorPanel() {
  const [vendor, setVendor] = useState('hikvision');
  const [firmware, setFirmware] = useState('unknown');
  const [result, setResult] = useState([]);
  const [status, setStatus] = useState('Awaiting input');

  const runPrediction = async () => {
    setStatus('Generating CVE predictions…');
    try {
      const res = await invoke('predict_cves', { vendor: vendor.trim(), firmware: firmware.trim() });
      setResult(Array.isArray(res) ? res : []);
      setStatus(`Received ${Array.isArray(res) ? res.length : 0} predictions.`);
    } catch (error) {
      setStatus(`Predictor unavailable: ${error}`);
      setResult([]);
    }
  };

  return (
    <section style={S.wrap}>
      <h3 style={S.title}>CVE Predictor</h3>
      <input style={S.input} value={vendor} onChange={(e) => setVendor(e.target.value)} placeholder="Vendor" />
      <input style={S.input} value={firmware} onChange={(e) => setFirmware(e.target.value)} placeholder="Firmware" />
      <button type="button" style={S.btn} onClick={runPrediction}>🧬 Predict CVEs</button>
      <div style={{ marginTop: '10px', fontSize: '11px', color: '#cda4e9' }}>{status}</div>
      <div style={{ marginTop: '10px', display: 'grid', gap: '6px' }}>
        {result.length > 0 ? result.map((entry, idx) => (
          <div key={idx} style={{ border: '1px solid #5a3169', borderRadius: '6px', padding: '8px', background: '#180d20' }}>
            <div style={{ fontWeight: 700, color: '#f0d9ff', fontSize: '11px' }}>{entry.cve || `Prediction ${idx + 1}`}</div>
            <div style={{ fontSize: '10px', color: '#bf95d8' }}>{entry.reasoning || entry.summary || 'No summary available.'}</div>
          </div>
        )) : <div style={{ fontSize: '11px', color: '#8c6d99' }}>No predictions yet.</div>}
      </div>
    </section>
  );
}
