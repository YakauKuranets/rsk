import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

const fieldStyle = {
  width: '100%',
  boxSizing: 'border-box',
  marginTop: '8px',
  padding: '7px 8px',
  borderRadius: '4px',
  background: '#210b0d',
  color: '#ffd6d9',
  border: '1px solid #6e2d33',
};

export default function PayloadPanel() {
  const [type, setType] = useState('hta');
  const [title, setTitle] = useState('Monthly camera maintenance notice');
  const [callbackUrl, setCallbackUrl] = useState('https://callback.local/collect');
  const [permitToken, setPermitToken] = useState('');
  const [output, setOutput] = useState(null);
  const [status, setStatus] = useState('Готово к генерации.');

  const generate = async () => {
    if (!callbackUrl.trim()) {
      setStatus('Укажите callback URL.');
      return;
    }
    if (permitToken.trim().length < 8) {
      setStatus('Нужен разрешительный токен длиной не менее 8 символов.');
      return;
    }

    setStatus('Генерация полезной нагрузки...');
    setOutput(null);

    try {
      const payload = type === 'hta'
        ? await invoke('generate_hta_payload', {
          callbackUrl: callbackUrl.trim(),
          decoyTitle: title.trim() || 'Maintenance Notice',
          permitToken: permitToken.trim(),
        })
        : await invoke('generate_macro_lure', {
          callbackUrl: callbackUrl.trim(),
          docTitle: title.trim() || 'Maintenance Notice',
          permitToken: permitToken.trim(),
        });

      setOutput(payload);
      setStatus(`Payload создан: ${payload.filename}`);
    } catch (error) {
      setOutput(null);
      setStatus(`Генератор payload недоступен: ${error}`);
    }
  };

  return (
    <section style={{ border: '1px solid #532225', borderRadius: '8px', padding: '12px', background: '#18090a', color: '#ffd6d9' }}>
      <h3 style={{ margin: '0 0 10px', fontSize: '13px', color: '#ff8f9a', textTransform: 'uppercase', letterSpacing: '0.08em' }}>Payload Panel</h3>
      <select value={type} onChange={(e) => setType(e.target.value)} style={{ width: '100%', padding: '7px 8px', borderRadius: '4px', background: '#210b0d', color: '#ffd6d9', border: '1px solid #6e2d33' }}>
        <option value="hta">HTA payload</option>
        <option value="macro">Macro lure</option>
      </select>
      <input
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        style={fieldStyle}
        placeholder={type === 'hta' ? 'Decoy title' : 'Document title'}
      />
      <input
        value={callbackUrl}
        onChange={(e) => setCallbackUrl(e.target.value)}
        style={fieldStyle}
        placeholder="Callback URL"
      />
      <input
        value={permitToken}
        onChange={(e) => setPermitToken(e.target.value)}
        style={fieldStyle}
        type="password"
        placeholder="Permit token"
      />
      <button type="button" onClick={generate} style={{ width: '100%', marginTop: '8px', padding: '8px', borderRadius: '4px', border: '1px solid #a84d56', background: '#361418', color: '#ff8f9a', fontWeight: 700, cursor: 'pointer' }}>🧰 Generate payload</button>
      <div style={{ marginTop: '10px', fontSize: '11px', color: '#ffb3ba' }}>{status}</div>
      <pre style={{ marginTop: '10px', padding: '8px', borderRadius: '6px', border: '1px solid #6e2d33', background: '#120607', color: '#ffc0c5', whiteSpace: 'pre-wrap', fontSize: '11px', fontFamily: 'monospace' }}>{output ? JSON.stringify(output, null, 2) : 'No payload generated yet.'}</pre>
    </section>
  );
}
