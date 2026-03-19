// src/features/logs/RuntimeLogs.jsx
import { useEffect, useRef } from 'react';
import { getRuntimeLogs } from '../../api/tauri';

export default function RuntimeLogs({ runtimeLogs, setRuntimeLogs }) {
  const intervalRef = useRef(null);

  useEffect(() => {
    intervalRef.current = setInterval(async () => {
      try {
        const logs = await getRuntimeLogs();
        setRuntimeLogs(logs || []);
      } catch {}
    }, 2000);
    return () => clearInterval(intervalRef.current);
  }, [setRuntimeLogs]);

  return (
    <div style={{ marginBottom: '20px' }}>
      <h3 style={{ color: '#00f0ff', marginTop: '30px', fontSize: '0.9rem' }}>LIVE-ЛОГИ ЯДРА</h3>
      <div style={{ background: '#000', border: '1px solid #111', padding: '8px', height: '120px', overflowY: 'auto', fontFamily: 'monospace', fontSize: '10px' }}>
        {runtimeLogs.length === 0
          ? <span style={{ color: '#333' }}>[boot] runtime log started</span>
          : runtimeLogs.map((log, i) => (
            <div key={i} style={{ color: '#5a8a6a', marginBottom: '2px' }}>
              [{log.time || '??:??:??'}] {log.message || log}
            </div>
          ))}
      </div>
    </div>
  );
}
