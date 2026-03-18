import React from 'react';

export default function TargetList({
  targets,
  targetSearch,
  setTargetSearch,
  targetTypeFilter,
  setTargetTypeFilter,
  archiveOnly,
  setArchiveOnly,
  onNemesis,
  onMemoryRequest,
  onIsapiInfo,
  onIsapiSearch,
  onOnvifInfo,
  onOnvifRecordings,
  onArchiveEndpoints,
  onDelete,
}) {
  return (
    <>
      <h3 style={{ color: '#00f0ff', marginTop: '40px', fontSize: '0.9rem' }}>БАЗА ЦЕЛЕЙ</h3>
      <div style={{ border: '1px solid #222', background: '#050505', padding: '8px', marginBottom: '10px' }}>
        <input
          style={{ width: '100%', backgroundColor: '#000', border: '1px solid #333', color: '#00f0ff', padding: '8px', marginBottom: '6px', boxSizing: 'border-box' }}
          placeholder="Поиск по имени/IP"
          value={targetSearch}
          onChange={(e) => setTargetSearch(e.target.value)}
        />
        <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
          <button onClick={() => setTargetTypeFilter('all')} style={{ flex: 1, background: targetTypeFilter === 'all' ? '#1a4a4a' : '#111', color: '#00f0ff', border: '1px solid #00f0ff', padding: '6px', cursor: 'pointer', fontSize: '11px' }}>ВСЕ</button>
          <button onClick={() => setTargetTypeFilter('hub')} style={{ flex: 1, background: targetTypeFilter === 'hub' ? '#4a1a4a' : '#111', color: '#ff00ff', border: '1px solid #ff00ff', padding: '6px', cursor: 'pointer', fontSize: '11px' }}>HUB</button>
          <button onClick={() => setTargetTypeFilter('local')} style={{ flex: 1, background: targetTypeFilter === 'local' ? '#4a3a1a' : '#111', color: '#ffcc66', border: '1px solid #ffcc66', padding: '6px', cursor: 'pointer', fontSize: '11px' }}>LOCAL</button>
        </div>
        <label style={{ fontSize: '11px', color: '#bbb', display: 'flex', alignItems: 'center', gap: '6px' }}>
          <input type="checkbox" checked={archiveOnly} onChange={(e) => setArchiveOnly(e.target.checked)} />
          Только цели с архивом
        </label>
      </div>
      {targets.map((t) => (
        <div key={t.id} style={{ border: '1px solid #222', padding: '10px', marginBottom: '8px', position: 'relative', backgroundColor: '#0a0a0c' }}>
          <div style={{ color: t.type === 'hub' ? '#ff00ff' : '#00f0ff', fontSize: '0.9rem', paddingRight: '20px' }}>{t.name}</div>
          <div style={{ fontSize: '10px', color: '#555', marginBottom: '8px' }}>{t.host}</div>

          {t.type !== 'hub' && (
            <>
              <button onClick={() => onNemesis?.(t)} style={{ width: '100%', background: 'linear-gradient(90deg, #2a0808, #0a0808)', color: '#ff003c', border: '1px solid #ff003c', padding: '6px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold', letterSpacing: '1px' }}>
                ☢ NEMESIS ARCHIVE
              </button>
              <button onClick={() => onMemoryRequest?.(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#4a1a4a', color: '#ff9900', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                ⏳ ЗАПРОС ПАМЯТИ
              </button>
              <button onClick={() => onIsapiInfo?.(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#1a1a4a', color: '#9fc2ff', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                ℹ ISAPI DEVICE INFO
              </button>
              <button onClick={() => onIsapiSearch?.(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#1f2d4a', color: '#9fd7ff', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                🔎 ISAPI SEARCH RECORDS
              </button>
              <button onClick={() => onOnvifInfo?.(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#1a3a1a', color: '#a8ffb0', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                ℹ ONVIF DEVICE INFO
              </button>
              <button onClick={() => onOnvifRecordings?.(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#1a3a1a', color: '#b9ffcf', border: '1px solid #2a5a36', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                🔎 ONVIF RECORDINGS
              </button>
              <button onClick={() => onArchiveEndpoints?.(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#3a2a1a', color: '#ffd27a', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                📦 PROBE EXPORT ENDPOINTS
              </button>
            </>
          )}

          <button onClick={() => onDelete?.(t.id)} style={{ position: 'absolute', right: 8, top: 8, background: 'none', border: 'none', color: '#ff003c', cursor: 'pointer' }}>✖</button>
        </div>
      ))}
    </>
  );
}
