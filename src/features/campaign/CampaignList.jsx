import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function CampaignList({ onOpen }) {
  const [items, setItems] = useState([]);
  const load = async () => { try { setItems(await invoke('list_campaigns')); } catch {} };
  useEffect(() => { load(); }, []);

  const sevCount = (f, sev) => f.filter((x) => x.severity === sev).length;

  return (
    <div style={{ border: '1px solid #222', padding: 10 }}>
      <h4 style={{ color: '#00f0ff', marginTop: 0 }}>Кампании</h4>
      {items.map((c) => (
        <div key={c.id} style={{ border: '1px solid #333', background: '#0a0a0c', padding: 8, marginBottom: 8 }}>
          <div><b>{c.name}</b> · {c.clientName}</div>
          <div style={{ fontSize: 11 }}>Status: <span style={{ color: '#7dff9c' }}>{String(c.status)}</span></div>
          <div style={{ fontSize: 11 }}>critical:{sevCount(c.findings||[], 'critical')} high:{sevCount(c.findings||[], 'high')} medium:{sevCount(c.findings||[], 'medium')} low:{sevCount(c.findings||[], 'low')}</div>
          <button onClick={() => onOpen?.(c.id)}>Открыть</button>
          <button onClick={async () => { await invoke('export_campaign_report', { campaignId: c.id, format: 'markdown' }); }}>Экспорт</button>
          <button onClick={async () => { await invoke('update_campaign_status', { campaignId: c.id, newStatus: 'archived' }); load(); }}>Архивировать</button>
        </div>
      ))}
    </div>
  );
}
