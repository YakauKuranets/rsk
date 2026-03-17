import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../store/appStore';
import { toast } from '../../utils/toast';

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
  const [proxyList, setProxyList] = useState('');
  const [osintWords, setOsintWords] = useState('');

  const handleMassAudit = async () => {
    if (!massAuditIps.trim()) return;

    setMassAuditing(true);
    setMassAuditResults([]);

    const ipRegex = /\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b/g;
    const extractedIps = massAuditIps.match(ipRegex) || [];
    const uniqueIps = [...new Set(extractedIps)];

    if (uniqueIps.length === 0) {
      toast('В тексте не найдено валидных IP-адресов!');
      setMassAuditing(false);
      return;
    }

    try {
      const proxies = proxyList.split('\n').map((p) => p.trim()).filter((p) => p.length > 0);
      const osintContext = osintWords.split(',').map((w) => w.trim()).filter((w) => w.length > 0);
      const results = await invoke('run_mass_audit', {
        targetIps: uniqueIps,
        knownLogin: massAuditLogin,
        knownPass: massAuditPass,
        proxies,
        osintContext,
      });
      setMassAuditResults(results);
    } catch (error) {
      toast(`Ошибка массового аудита: ${error}`);
    } finally {
      setMassAuditing(false);
    }
  };


  const handleCheckWebshells = async () => {
    const ipRegex = /\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b/g;
    const extractedIps = massAuditIps.match(ipRegex) || [];
    const targetIp = extractedIps[0] || '';

    if (!targetIp) {
      toast('Выберите цель! Укажите хотя бы один IP.');
      return;
    }

    try {
      toast('Запуск поиска веб-шеллов...');
      const result = await invoke('check_persistence', { target: targetIp });
      if (result.length > 0) {
        toast(`⚠️ НАЙДЕНО ШЕЛЛОВ: ${result.length}`);
        console.log('Шеллы:', result);
      } else {
        toast('Веб-шеллы не обнаружены (Чисто)');
      }
    } catch (err) {
      toast('Ошибка проверки: ' + err);
    }
  };

  const handleGetMetadata = async (ip) => {
    try {
      const meta = await invoke('collect_metadata', { ip });
      toast(`Метаданные для ${ip}\nНазвание: ${meta.sessionName || meta.session_name}\nСервер: ${meta.serverHeader || meta.server_header}`);
    } catch (error) {
      toast(`Не удалось получить метаданные: ${error}`);
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

      <div style={{ marginTop: '10px', marginBottom: '10px' }}>
        <div style={{ fontSize: '10px', color: '#ffcc00', marginBottom: '4px' }}>
          🌐 PROXY MESH (SOCKS5/HTTP) — По одному на строку:
        </div>
        <textarea
          value={proxyList}
          onChange={e => setProxyList(e.target.value)}
          placeholder={'socks5://127.0.0.1:9050\nhttp://user:pass@192.168.1.55:8080'}
          style={{ width: '100%', height: '60px', background: '#000', color: '#ffcc00', border: '1px solid #ffcc00', padding: '4px', fontSize: '10px' }}
        />
      </div>


      <div style={{ marginTop: '10px', marginBottom: '10px' }}>
        <div style={{ fontSize: '10px', color: '#ff3366', marginBottom: '4px' }}>
          🧠 OSINT СЛОВАРЬ (Слова через запятую: компания, город, год):
        </div>
        <input
          type="text"
          value={osintWords}
          onChange={e => setOsintWords(e.target.value)}
          placeholder="Например: gazprom, moscow, admin"
          style={{ width: '100%', padding: '6px', background: '#000', color: '#ff3366', border: '1px solid #ff3366', fontSize: '11px' }}
        />
      </div>

      <button
        onClick={handleCheckWebshells}
        style={{ padding: '8px', background: '#330000', color: '#ff3366', border: '1px solid #ff3366', cursor: 'pointer', fontSize: '11px', width: '100%', marginBottom: '8px' }}
      >
        🦠 ИСКАТЬ WEBSHELL (PTES)
      </button>

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
