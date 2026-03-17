import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function PassiveScanner() {
  const [subnet, setSubnet] = useState('192.168.1.0/24');
  const [depth, setDepth] = useState('normal');
  const [report, setReport] = useState(null);

  const run = async () => setReport(await invoke('passive_scan_network', { targetSubnet: subnet, scanDepth: depth }));
  const runPcap = async () => {
    const path = window.prompt('Укажите путь к PCAP');
    if (!path) return;
    setReport(await invoke('analyze_pcap_file', { pcapPath: path }));
  };

  return (
    <div style={{ border: '1px solid #222', padding: 10 }}>
      <div style={{ fontSize: 11, color: '#d7c87b', marginBottom: 6 }}>Режим read-only: никакие данные на устройствах не модифицируются</div>
      <input value={subnet} onChange={(e) => setSubnet(e.target.value)} placeholder='192.168.1.0/24' />
      <select value={depth} onChange={(e) => setDepth(e.target.value)}><option value='quick'>Quick</option><option value='normal'>Normal</option><option value='deep'>Deep</option></select>
      <button onClick={run}>ПАССИВНЫЙ СКАН</button>
      <button onClick={runPcap}>АНАЛИЗ PCAP</button>

      {report && (
        <>
          <div style={{ marginTop: 8 }}>Всего: {report.totalDevices} | Медицинские: {report.medicalDevices?.length || 0} | Высокий риск: {report.highRiskCount}</div>
          <table style={{ width: '100%', fontSize: 12 }}><thead><tr><th>IP</th><th>MAC</th><th>Manufacturer</th><th>Type</th><th>Risk</th><th>CVEs</th><th>Ports</th></tr></thead><tbody>
            {(report.devices || []).map((d) => <tr key={d.ip} style={{ background: (d.deviceType==='medical' || d.riskLevel==='critical') ? '#2b1212' : undefined }}><td>{d.ip}</td><td>{d.macAddress}</td><td>{d.manufacturer}</td><td>{d.deviceType}</td><td>{d.riskLevel}</td><td>{(d.knownCves||[]).length}</td><td>{(d.openPorts||[]).join(',')}</td></tr>)}
          </tbody></table>
          <div style={{ marginTop: 8, background: '#2f280d', padding: 6 }}>⚠️ {(report.warnings || []).map((w, i) => <div key={i}>{w}</div>)}</div>
        </>
      )}
    </div>
  );
}
