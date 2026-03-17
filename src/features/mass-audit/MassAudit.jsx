import React from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../store/appStore';

export default function MassAudit() {
  const massAuditIps = useAppStore((s) => s.massAuditIps);
  const massAuditLogin = useAppStore((s) => s.massAuditLogin);
  const massAuditPass = useAppStore((s) => s.massAuditPass);
  const massAuditResults = useAppStore((s) => s.massAuditResults);
  const massAuditing = useAppStore((s) => s.massAuditing);
  const setMassAuditIps = useAppStore((s) => s.setMassAuditIps);
  const setMassAuditLogin = useAppStore((s) => s.setMassAuditLogin);
  const setMassAuditPass = useAppStore((s) => s.setMassAuditPass);
  const setMassAuditResults = useAppStore((s) => s.setMassAuditResults);
  const setMassAuditing = useAppStore((s) => s.setMassAuditing);

  const handleMassAudit = async () => {
    if (!massAuditIps.trim()) return;

    setMassAuditing(true);
    setMassAuditResults([]);

    const ipRegex = /\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b/g;
    const extractedIps = massAuditIps.match(ipRegex) || [];
    const uniqueIps = [...new Set(extractedIps)];

    if (uniqueIps.length === 0) {
      alert('В тексте не найдено валидных IP-адресов!');
      setMassAuditing(false);
      return;
    }

    try {
      const results = await invoke('run_mass_audit', {
        targetIps: uniqueIps,
        knownLogin: massAuditLogin,
        knownPass: massAuditPass,
      });
      setMassAuditResults(results);
    } catch (error) {
      alert(`Ошибка массового аудита: ${error}`);
    } finally {
      setMassAuditing(false);
    }
  };

  const handleGetMetadata = async (ip) => {
    try {
      const meta = await invoke('collect_metadata', { ip });
      alert(`Метаданные для ${ip}\nНазвание: ${meta.sessionName || meta.session_name}\nСервер: ${meta.serverHeader || meta.server_header}`);
    } catch (error) {
      alert(`Не удалось получить метаданные: ${error}`);
    }
  };

  return (
    <div style={{ border: '1px solid #6a88ff', padding: '10px', backgroundColor: '#090f1f', marginBottom: '20px', boxShadow: '0 0 10px rgba(106,136,255,0.15)' }}>
      <h3 style={{ color: '#9fc2ff', marginTop: 0, fontSize: '0.9rem' }}>🚀 ПАКЕТНАЯ ИНВЕНТАРИЗАЦИЯ (PTES Batch Auditor)</h3>
      <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
        <input style={{ flex: 1, backgroundColor: '#000', border: '1px solid #6a88ff', color: '#9fc2ff', padding: '6px', fontSize: '11px' }} placeholder="Логин" value={massAuditLogin} onChange={(e) => setMassAuditLogin(e.target.value)} />
        <input type="password" style={{ flex: 1, backgroundColor: '#000', border: '1px solid #6a88ff', color: '#9fc2ff', padding: '6px', fontSize: '11px' }} placeholder="Пароль" value={massAuditPass} onChange={(e) => setMassAuditPass(e.target.value)} />
      </div>

      <textarea style={{ width: '100%', minHeight: '70px', backgroundColor: '#000', border: '1px solid #6a88ff', color: '#9fc2ff', padding: '6px', fontSize: '11px', marginBottom: '8px' }} placeholder="IP через запятую/пробел" value={massAuditIps} onChange={(e) => setMassAuditIps(e.target.value)} />

      <button disabled={massAuditing} onClick={handleMassAudit} style={{ width: '100%', backgroundColor: massAuditing ? '#334' : '#6a88ff', color: '#fff', border: 'none', padding: '8px', cursor: massAuditing ? 'default' : 'pointer', fontWeight: 'bold' }}>
        {massAuditing ? '⏳ Идет сканирование...' : 'Запустить массовый аудит'}
      </button>

      {massAuditResults.length > 0 && (
        <div style={{ marginTop: '10px', border: '1px solid #30406a', background: '#050913', maxHeight: '220px', overflowY: 'auto', padding: '6px' }}>
          {massAuditResults.map((res, idx) => (
            <div key={`${res.ip}_${idx}`} style={{ borderBottom: '1px solid #1b2440', padding: '6px 0', fontSize: '11px', color: '#d9e4ff' }}>
              <div><b>{res.ip}</b></div>
              <div>{res.is_alive ? '🟢 Доступен (554)' : '🔴 Недоступен'}</div>
              <div>{res.creds_reused ? '🚨 Пароль подошел!' : '✅ Reuse не подтвержден'}</div>
              {res.is_alive && <button onClick={() => handleGetMetadata(res.ip)} style={{ marginTop: 4, background: '#16264f', border: '1px solid #6a88ff', color: '#9fc2ff', padding: '4px 8px', cursor: 'pointer', fontSize: '10px' }}>SDP метаданные</button>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
