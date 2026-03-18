// src/features/relay/RelayPanel.jsx
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../../utils/toast';

export default function RelayPanel() {
  const [relayUrl, setRelayUrl] = useState(() => localStorage.getItem('relay_url') || 'http://192.168.1.100:8090');
  const [relayToken, setRelayToken] = useState(() => localStorage.getItem('relay_token') || '');
  const [relayStatus, setRelayStatus] = useState(null);

  const checkRelay = async () => {
    try {
      await invoke('relay_ping', { relayUrl: relayUrl.trim() });
      setRelayStatus('ok');
      localStorage.setItem('relay_url', relayUrl);
      localStorage.setItem('relay_token', relayToken);
    } catch (e) {
      setRelayStatus('error');
      toast(`Relay: ${e}`);
    }
  };

  useEffect(() => {
    return () => {};
  }, []);

  const S = {
    box: { border: '1px solid #2a2a6a', padding: '10px', backgroundColor: '#05050f', marginBottom: '16px' },
    h: { color: '#6a6aff', marginTop: 0, fontSize: '0.85rem' },
    input: {
      width: '100%',
      padding: '5px 8px',
      background: '#08080f',
      color: '#ccc',
      border: '1px solid #2a2a4a',
      marginBottom: '6px',
      fontSize: '12px',
      boxSizing: 'border-box',
    },
  };

  return (
    <div style={S.box}>
      <h3 style={S.h}>🔗 FTP RELAY (ПК 2)</h3>
      <div style={{ fontSize: '10px', color: '#3a3a7a', marginBottom: '8px' }}>
        Если FTP недоступен — запустить hyperion-relay.exe на ПК с доступом
      </div>
      <input style={S.input} value={relayUrl} onChange={(e) => setRelayUrl(e.target.value)} placeholder="http://192.168.1.100:8090" />
      <input style={S.input} value={relayToken} onChange={(e) => setRelayToken(e.target.value)} placeholder="Token (опц.)" />
      <button
        onClick={checkRelay}
        style={{ width: '100%', padding: '6px', background: '#1a1a3a', color: '#6a6aff', border: '1px solid #3a3aaa', cursor: 'pointer', fontWeight: 'bold', fontSize: '11px' }}
      >
        🔗 ПРОВЕРИТЬ СВЯЗЬ
      </button>
      {relayStatus && (
        <div style={{ marginTop: '6px', fontSize: '11px', color: relayStatus === 'ok' ? '#00cc66' : '#ff4444' }}>
          {relayStatus === 'ok' ? '✓ Relay доступен' : '✗ Relay недоступен'}
        </div>
      )}
    </div>
  );
}
