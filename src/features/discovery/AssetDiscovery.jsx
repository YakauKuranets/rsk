import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { discoverExternalAssets } from '../../api/tauri';
import { toast } from '../../utils/toast';

export default function AssetDiscovery() {
  const [targetDomain, setTargetDomain] = useState('example.com');
  const [shodanApiKey, setShodanApiKey] = useState('');
  const [enableCertTransparency, setEnableCertTransparency] = useState(true);
  const [enableDnsEnum, setEnableDnsEnum] = useState(true);
  const [loading, setLoading] = useState(false);
  const [report, setReport] = useState(null);
  const [error, setError] = useState('');
  const [intelTarget, setIntelTarget] = useState('');
  const [breachResults, setBreachResults] = useState(null);
  const [isSearchingIntel, setIsSearchingIntel] = useState(false);


  const handleIntelSearch = async () => {
    if (!intelTarget.trim()) {
      toast('Введите email или домен для поиска', 'error');
      return;
    }

    setIsSearchingIntel(true);
    setBreachResults(null);

    try {
      toast('🕵️ Поиск по базам утечек...');
      const results = await invoke('check_breaches', { target: intelTarget.trim() });
      setBreachResults(results);
      if (results.length > 0) {
        toast(`⚠️ НАЙДЕНЫ СЛЕДЫ КОМПРОМЕТАЦИИ: ${results.length}`, 'error');
      } else {
        toast('Цель чиста. Утечек не найдено.', 'success');
      }
    } catch (err) {
      toast('Ошибка Threat Intel: ' + err, 'error');
    } finally {
      setIsSearchingIntel(false);
    }
  };

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



      <div style={{ marginTop: '20px', padding: '15px', background: '#0a0a0c', border: '1px solid #ff3366', borderRadius: '4px' }}>
        <h3 style={{ color: '#ff3366', marginTop: 0, fontSize: '14px' }}>🕵️ THREAT INTELLIGENCE (Утечки данных)</h3>
        <div style={{ display: 'flex', gap: '10px' }}>
          <input
            type="text"
            value={intelTarget}
            onChange={e => setIntelTarget(e.target.value)}
            placeholder="Введите домен (example.com) или email"
            style={{ flex: 1, padding: '8px', background: '#000', color: '#fff', border: '1px solid #ff3366' }}
          />
          <button
            onClick={handleIntelSearch}
            disabled={isSearchingIntel}
            style={{ padding: '8px 15px', background: '#ff3366', color: '#fff', fontWeight: 'bold', border: 'none', cursor: isSearchingIntel ? 'wait' : 'pointer' }}
          >
            {isSearchingIntel ? 'ПОИСК...' : 'ПРОБИТЬ ПО БАЗАМ'}
          </button>
        </div>

        {breachResults && (
          <div style={{ marginTop: '15px' }}>
            {breachResults.length === 0 ? (
              <div style={{ color: '#00ffcc' }}>✅ Следов компрометации в публичных базах не обнаружено.</div>
            ) : (
              <div style={{ display: 'grid', gap: '10px' }}>
                {breachResults.map((b, i) => (
                  <div key={`breach_${i}`} style={{ background: '#1a0000', padding: '10px', borderLeft: '3px solid #ff3366', fontSize: '12px' }}>
                    <strong style={{ color: '#ff3366' }}>{b.source}: {b.title}</strong>
                    <div style={{ color: '#aaa', marginTop: '4px' }}>{b.description}</div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>

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
