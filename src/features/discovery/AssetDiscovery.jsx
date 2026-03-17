import React, { useState } from 'react';
import { discoverExternalAssets } from '../../api/tauri';

export default function AssetDiscovery() {
  const [targetDomain, setTargetDomain] = useState('example.com');
  const [shodanApiKey, setShodanApiKey] = useState('');
  const [enableCertTransparency, setEnableCertTransparency] = useState(true);
  const [enableDnsEnum, setEnableDnsEnum] = useState(true);
  const [loading, setLoading] = useState(false);
  const [report, setReport] = useState(null);
  const [error, setError] = useState('');

  const runDiscovery = async () => {
    setLoading(true);
    setError('');
    try {
      const result = await discoverExternalAssets({
        targetDomain,
        shodanApiKey: shodanApiKey.trim() || null,
        enableCertTransparency,
        enableDnsEnum,
      });
      setReport(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ border: '1px solid #00f0ff', padding: 10, backgroundColor: '#001a1a', marginBottom: 20 }}>
      <h3 style={{ color: '#00f0ff', marginTop: 0, fontSize: '0.9rem' }}>🌐 External Asset Discovery</h3>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8, marginBottom: 8 }}>
        <input
          value={targetDomain}
          onChange={(e) => setTargetDomain(e.target.value)}
          placeholder="target domain"
          style={{ background: '#000', border: '1px solid #00f0ff', color: '#00f0ff', padding: 8 }}
        />
        <input
          value={shodanApiKey}
          onChange={(e) => setShodanApiKey(e.target.value)}
          placeholder="Shodan API key (optional)"
          style={{ background: '#000', border: '1px solid #00f0ff', color: '#00f0ff', padding: 8 }}
        />
      </div>
      <div style={{ display: 'flex', gap: 12, marginBottom: 8, color: '#9dd' }}>
        <label><input type="checkbox" checked={enableCertTransparency} onChange={(e) => setEnableCertTransparency(e.target.checked)} /> Cert Transparency</label>
        <label><input type="checkbox" checked={enableDnsEnum} onChange={(e) => setEnableDnsEnum(e.target.checked)} /> DNS enum</label>
      </div>
      <button
        onClick={runDiscovery}
        disabled={loading}
        style={{ backgroundColor: '#00f0ff', color: '#000', border: 'none', padding: '10px', cursor: 'pointer', fontWeight: 'bold' }}
      >
        {loading ? 'Сканирование...' : '🕷️ Запустить разведку'}
      </button>

      {error && <div style={{ color: '#ff6677', marginTop: 8, fontSize: 12 }}>{error}</div>}

      {report && (
        <div style={{ marginTop: 10, fontSize: 11 }}>
          <div style={{ color: '#9ff' }}>
            Query: <b>{report.query}</b> · Assets: <b>{report.totalAssets}</b> · Certs: <b>{report.certificates.length}</b> · DNS: <b>{report.dnsRecords.length}</b> · {report.durationMs}ms
          </div>
          <div style={{ maxHeight: 180, overflowY: 'auto', marginTop: 8 }}>
            {report.assets.map((asset, idx) => (
              <div key={`${asset.ip}_${asset.port}_${idx}`} style={{ borderBottom: '1px solid #113', padding: '5px 0', color: '#bfe' }}>
                {asset.ip}:{asset.port} [{asset.source}] {asset.banner?.slice(0, 80)}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
