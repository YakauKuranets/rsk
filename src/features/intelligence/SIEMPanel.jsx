import { useMemo, useState } from 'react';

const seedEvents = [
  { ts: '00:11:52', source: 'suricata', level: 'medium', msg: 'RTSP brute-force pattern detected' },
  { ts: '00:14:07', source: 'syslog', level: 'high', msg: 'Unexpected admin login from maintenance VLAN' },
  { ts: '00:15:33', source: 'zeek', level: 'low', msg: 'New ONVIF endpoint fingerprinted' },
];

const colorFor = (level) => ({ low: '#6ba8ff', medium: '#ffbf47', high: '#ff667d' }[level] || '#9ba4c7');

export default function SIEMPanel() {
  const [filter, setFilter] = useState('all');
  const events = useMemo(() => seedEvents.filter((entry) => filter === 'all' || entry.level === filter), [filter]);

  return (
    <section style={{ border: '1px solid #2a3247', borderRadius: '8px', padding: '12px', background: '#0a0f17', color: '#e0e7ff' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '10px' }}>
        <h3 style={{ margin: 0, fontSize: '13px', color: '#95b3ff', textTransform: 'uppercase', letterSpacing: '0.08em' }}>SIEM Panel</h3>
        <select value={filter} onChange={(e) => setFilter(e.target.value)} style={{ padding: '6px 8px', borderRadius: '4px', background: '#121a27', color: '#e0e7ff', border: '1px solid #33415d' }}>
          <option value="all">All</option>
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
        </select>
      </div>
      <div style={{ display: 'grid', gap: '8px' }}>
        {events.map((event, idx) => {
          const color = colorFor(event.level);
          return (
            <div key={`${event.ts}-${idx}`} style={{ border: `1px solid ${color}40`, borderRadius: '6px', padding: '8px', background: `${color}10` }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px', fontSize: '10px' }}>
                <span style={{ color }}>{event.level.toUpperCase()}</span>
                <span style={{ color: '#7c88b0' }}>{event.ts} · {event.source}</span>
              </div>
              <div style={{ fontSize: '11px', color: '#dce4ff' }}>{event.msg}</div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
