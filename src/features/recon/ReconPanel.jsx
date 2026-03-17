import React from 'react';

export default function ReconPanel({
  hubSearch,
  setHubSearch,
  onHubSearch,
  portScanHost,
  setPortScanHost,
  onPortScan,
  onSecurityAudit,
  reconUserId,
  setReconUserId,
  reconChannelId,
  setReconChannelId,
  onReconRoutes,
}) {
  return (
    <div style={{ border: '1px solid #2a2a2e', padding: 12, background: '#0d0d11' }}>
      <h3 style={{ marginTop: 0, color: '#ff003c' }}>Recon Panel</h3>
      <div style={{ display: 'grid', gap: 8 }}>
        <input value={hubSearch} onChange={(e) => setHubSearch(e.target.value)} placeholder="Поиск по хабу" />
        <button onClick={onHubSearch}>Поиск HUB</button>

        <input value={portScanHost} onChange={(e) => setPortScanHost(e.target.value)} placeholder="Host/IP" />
        <div style={{ display: 'flex', gap: 8 }}>
          <button onClick={onPortScan}>Port Scan</button>
          <button onClick={onSecurityAudit}>Security Audit</button>
        </div>

        <input value={reconUserId} onChange={(e) => setReconUserId(e.target.value)} placeholder="Recon User ID" />
        <input value={reconChannelId} onChange={(e) => setReconChannelId(e.target.value)} placeholder="Recon Channel ID" />
        <button onClick={onReconRoutes}>Recon archive routes</button>
      </div>
    </div>
  );
}
