import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function CameraScanPanel({ onPlayCamera }) {
  const [target, setTarget] = useState('192.168.1.0/24');
  const [mode, setMode] = useState('normal');
  const [isScanning, setIsScanning] = useState(false);
  const [report, setReport] = useState(null);

  const handleScan = async () => {
    if (!target) return;
    setIsScanning(true);
    setReport(null);

    try {
      const res = await invoke('unified_camera_scan', {
        targetInput: target,
        scanMode: mode,
        maxConcurrent: 50,
        knownLogin: 'admin',
        knownPassword: '',
      });
      setReport(res);
    } catch (err) {
      console.error('Ошибка сканирования:', err);
    } finally {
      setIsScanning(false);
    }
  };

  return (
    <div style={{ border: '1px solid #00f0ff', padding: '10px', backgroundColor: '#001a1a', marginBottom: '20px', boxShadow: '0 0 10px rgba(0,240,255,0.15)' }}>
      <h3 style={{ color: '#00f0ff', marginTop: '0', fontSize: '0.9rem' }}>📡 РАДАР КАМЕР (WAVE 2)</h3>
      <div style={{ fontSize: '10px', color: '#6bb3b3', marginBottom: '8px' }}>
        Массовый скан подсетей (CIDR) на наличие камер. Определение вендора и доступных RTSP путей.
      </div>

      <div style={{ display: 'flex', gap: '6px', marginBottom: '10px' }}>
        <input
          type="text"
          value={target}
          onChange={(e) => setTarget(e.target.value)}
          placeholder="Подсеть (напр. 192.168.1.0/24)"
          style={{ flex: 2, padding: '6px', background: '#000', color: '#00f0ff', border: '1px solid #00f0ff', fontSize: '11px' }}
        />
        <select
          value={mode}
          onChange={(e) => setMode(e.target.value)}
          style={{ flex: 1, padding: '6px', background: '#000', color: '#fff', border: '1px solid #00f0ff', fontSize: '11px' }}
        >
          <option value="fast">Fast (Порты)</option>
          <option value="normal">Normal (+RTSP)</option>
          <option value="deep">Deep (+Brute)</option>
        </select>
      </div>

      <button
        onClick={handleScan}
        disabled={isScanning}
        style={{ width: '100%', padding: '8px', background: isScanning ? '#333' : '#00f0ff', color: '#000', fontWeight: 'bold', border: 'none', cursor: isScanning ? 'wait' : 'pointer', fontSize: '11px', letterSpacing: '1px' }}
      >
        {isScanning ? '⏳ СКАНИРОВАНИЕ ЭФИРА...' : 'ЗАПУСТИТЬ РАДАР'}
      </button>

      {report && (
        <div style={{ marginTop: '10px', maxHeight: '300px', overflowY: 'auto', borderTop: '1px solid #00f0ff', paddingTop: '10px' }}>
          <div style={{ color: '#00f0ff', fontSize: '10px', fontWeight: 'bold', marginBottom: '6px' }}>
            НАЙДЕНО УСТРОЙСТВ: {report.camerasFound} ({report.durationMs}ms)
          </div>
          {report.cameras.map((cam, idx) => (
            <div key={idx} style={{ background: '#051111', padding: '8px', marginBottom: '6px', borderLeft: '3px solid #00f0ff' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', marginBottom: '4px' }}>
                <strong style={{ color: '#fff' }}>{cam.ip}</strong>
                <span style={{ color: '#00f0ff' }}>{cam.vendor}</span>
              </div>

              {cam.credentials && (
                <div style={{ color: '#ff003c', fontSize: '10px', marginBottom: '4px' }}>
                  🔑 {cam.credentials.login}:{cam.credentials.password}
                </div>
              )}

              {cam.rtspPaths && cam.rtspPaths.length > 0 && (
                <div style={{ marginTop: '6px' }}>
                  {cam.rtspPaths.map((rtsp, i) => (
                    <div key={i} style={{ display: 'flex', gap: '6px', alignItems: 'center', marginBottom: '4px' }}>
                      <code style={{ background: '#000', padding: '2px 4px', fontSize: '9px', flex: 1, color: '#aaa', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                        {rtsp.url}
                      </code>
                      <button
                        onClick={() =>
                          onPlayCamera &&
                          onPlayCamera({
                            ip: cam.ip,
                            terminalId: `radar_${cam.ip}`,
                            terminal: {
                              host: cam.ip,
                              login: cam.credentials?.login || 'admin',
                              password: cam.credentials?.password || '',
                              name: `Радар: ${cam.vendor}`,
                            },
                            channel: { id: 'ch_1', index: 1, name: 'Main Stream' },
                          })
                        }
                        style={{ background: '#1a4a4a', color: '#00f0ff', border: '1px solid #00f0ff', padding: '2px 6px', cursor: 'pointer', fontSize: '9px' }}
                      >
                        PLAY
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
          {report.cameras.length === 0 && <div style={{ color: '#888', fontSize: '10px' }}>В данной подсети камер не обнаружено.</div>}
        </div>
      )}
    </div>
  );
}
