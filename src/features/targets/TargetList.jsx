import React, { useMemo, useState } from 'react';

export default function TargetList({ targets, onDelete }) {
  const [q, setQ] = useState('');
  const filtered = useMemo(() => {
    const t = q.trim().toLowerCase();
    if (!t) return targets;
    return targets.filter((x) => `${x.name || ''} ${x.host || ''}`.toLowerCase().includes(t));
  }, [targets, q]);

  return (
    <div style={{ border: '1px solid #2a2a2e', padding: 12, background: '#0d0d11' }}>
      <h3 style={{ marginTop: 0, color: '#a6ffad' }}>Target List</h3>
      <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Поиск" style={{ width: '100%', marginBottom: 8 }} />
      <div style={{ maxHeight: 280, overflow: 'auto', display: 'grid', gap: 6 }}>
        {filtered.map((t) => (
          <div key={t.id || `${t.host}-${t.name}`} style={{ border: '1px solid #222', padding: 6 }}>
            <div>{t.name || 'Target'}</div>
            <div style={{ color: '#999' }}>{t.host}</div>
            <button onClick={() => onDelete?.(t.id)}>Удалить</button>
          </div>
        ))}
      </div>
    </div>
  );
}
