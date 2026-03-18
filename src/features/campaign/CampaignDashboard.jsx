import React, { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function CampaignDashboard({ campaignId }) {
  const [tab, setTab] = useState('findings');
  const [campaign, setCampaign] = useState(null);
  const [severity, setSeverity] = useState('all');

  const load = async () => { if (!campaignId) return; setCampaign(await invoke('get_campaign', { campaignId })); };
  useEffect(() => { load(); }, [campaignId]);

  const findings = useMemo(() => (campaign?.findings || []).filter((f) => severity === 'all' || f.severity === severity), [campaign, severity]);

  if (!campaignId) return null;
  if (!campaign) return <div>Loading campaign…</div>;

  return (
    <div style={{ border: '1px solid #222', marginTop: 8, padding: 10 }}>
      <div style={{ marginBottom: 8 }}><b>{campaign.name}</b> · {campaign.clientName}</div>
      <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
        {['findings','timeline','notes','report'].map((t) => <button key={t} onClick={() => setTab(t)}>{t}</button>)}
      </div>

      {tab === 'findings' && (
        <div>
          <select value={severity} onChange={(e) => setSeverity(e.target.value)}><option value='all'>all</option><option>critical</option><option>high</option><option>medium</option><option>low</option><option>info</option></select>
          <table style={{ width: '100%', fontSize: 12 }}><thead><tr><th>Severity</th><th>Target</th><th>Title</th><th>Status</th><th>Module</th></tr></thead><tbody>
            {findings.map((f) => <tr key={f.id}><td>{f.severity}</td><td>{f.targetIp}</td><td>{f.title}</td><td>{f.status}</td><td>{f.module}</td></tr>)}
          </tbody></table>
        </div>
      )}
      {tab === 'timeline' && <div>{(campaign.timeline||[]).map((t, i) => <div key={i}>{t.timestamp} · {t.eventType} · {t.description}</div>)}</div>}
      {tab === 'notes' && <div>{(campaign.notes||[]).map((n) => <div key={n.id}>{n.author}: {n.text}</div>)}</div>}
      {tab === 'report' && <div><button onClick={() => invoke('export_campaign_report', { campaignId, format: 'json' })}>JSON</button><button onClick={() => invoke('export_campaign_report', { campaignId, format: 'markdown' })}>Markdown</button><button onClick={() => invoke('export_campaign_report', { campaignId, format: 'csv' })}>CSV</button></div>}
    </div>
  );
}
