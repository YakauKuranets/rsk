// src/features/archive/NvrProbePanel.jsx


export default function NvrProbePanel({
  nvr,                         // весь хук useNvrPanel()
  capture,                     // весь хук useCapturePanel()
  auditResults,                // state из App.jsx
  handlePortScan,
  handleSecurityAudit,
  handleDownloadIsapiPlayback,
  handleCaptureIsapiPlayback,
  handleDownloadOnvifToken,
  isPlayableRecord,            // fn(item) -> bool
  isDownloadableRecord,        // fn(item) -> bool
}) {
  return (
    <div style={{ marginTop: '20px' }}>


      {/* Анализатор узла */}
      <h3 style={{ color: '#00f0ff', fontSize: '0.9rem', marginBottom: '10px' }}>АНАЛИЗАТОР УЗЛА (ПОРТЫ И ЗАЩИТА)</h3>
      <div style={{ display: 'flex', gap: '6px', marginBottom: '10px' }}>
        <input
          style={{ flex: 1, backgroundColor: '#000', border: '1px solid #333', color: '#00f0ff', padding: '10px', boxSizing: 'border-box' }}
          placeholder='IP/Host (пример: 192.168.1.100)'
          value={capture.portScanHost}
          onChange={e => capture.setPortScanHost(e.target.value)}
        />
        <button onClick={handlePortScan} style={{ backgroundColor: '#1a4a4a', color: '#00f0ff', border: '1px solid #00f0ff', cursor: 'pointer', padding: '0 12px', fontWeight: 'bold' }}>ПОРТЫ</button>
        <button onClick={handleSecurityAudit} style={{ backgroundColor: '#4a1a4a', color: '#ff00ff', border: '1px solid #ff00ff', cursor: 'pointer', padding: '0 12px', fontWeight: 'bold' }}>АУДИТ</button>
      </div>


      {capture.portScanResult.length > 0 && (
        <div style={{ border: '1px solid #222', background: '#050505', marginBottom: '10px' }}>
          {capture.portScanResult.map((item) => (
            <div key={item.port} style={{ display: 'flex', justifyContent: 'space-between', padding: '8px 10px', borderBottom: '1px solid #111', fontSize: '11px' }}>
              <span style={{ color: '#aaa' }}>{item.port} / {item.service}</span>
              <span style={{ color: item.open ? '#00ff9c' : '#ff5555', fontWeight: 'bold' }}>{item.open ? 'OPEN' : 'CLOSED'}</span>
            </div>
          ))}
        </div>
      )}


      {auditResults.length > 0 && (
        <div style={{ border: '1px solid #ff00ff', background: '#1a001a', padding: '8px', marginBottom: '10px' }}>
          <div style={{ color: '#ff00ff', fontSize: '10px', marginBottom: '6px', fontWeight: 'bold' }}>РЕЗУЛЬТАТЫ ГЛУБОКОГО АУДИТА:</div>
          {auditResults.map((line, idx) => (
            <div key={idx} style={{ fontSize: '11px', color: line.includes('🔴') ? '#ff5555' : line.includes('🟢') ? '#00ff9c' : '#aaa', marginBottom: '4px' }}>{line}</div>
          ))}
        </div>
      )}


      <hr style={{ borderColor: '#222' }} />


      {/* NVR Probe Results */}
      <div style={{ marginTop: '20px' }}>
        <h3 style={{ color: '#00f0ff', fontSize: '0.9rem', marginBottom: '10px' }}>NVR PROBE (ISAPI/ONVIF)</h3>
        <div style={{ border: '1px solid #222', background: '#050505', maxHeight: '180px', overflowY: 'auto', padding: '8px' }}>
          {nvr.nvrProbeResults.length === 0 && <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «⏳ ЗАПРОС ПАМЯТИ» у локальной цели.</div>}
          {nvr.nvrProbeResults.map((r, idx) => (
            <div key={`${r.protocol}_${r.endpoint}_${idx}`} style={{ borderBottom: '1px solid #111', padding: '6px 0' }}>
              <div style={{ fontSize: '10px', color: '#bbb' }}>{r.protocol}</div>
              <div style={{ fontSize: '10px', color: '#777', wordBreak: 'break-all' }}>{r.endpoint}</div>
              <div style={{ fontSize: '10px', color: r.status === 'detected' ? '#00ff9c' : r.status === 'not_detected' ? '#ffcc66' : '#ff5555' }}>{r.status}</div>
            </div>
          ))}
        </div>
      </div>


      {/* ISAPI Device Info */}
      <div style={{ marginTop: '14px' }}>
        <h3 style={{ color: '#00f0ff', fontSize: '0.85rem', marginBottom: '8px' }}>ISAPI DEVICE INFO</h3>
        <div style={{ border: '1px solid #222', background: '#050505', maxHeight: '160px', overflowY: 'auto', padding: '8px' }}>
          {!nvr.nvrDeviceInfo && <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «ℹ ISAPI DEVICE INFO» у локальной цели.</div>}
          {nvr.nvrDeviceInfo && (<>
            <div style={{ color: '#aaa', fontSize: '10px', marginBottom: '6px', wordBreak: 'break-all' }}>{nvr.nvrDeviceInfo.endpoint} [{nvr.nvrDeviceInfo.status}]</div>
            <pre style={{ margin: 0, color: '#9fc2ff', fontSize: '10px', whiteSpace: 'pre-wrap' }}>{nvr.nvrDeviceInfo.bodyPreview || 'empty'}</pre>
          </>)}
        </div>
      </div>


      {/* ISAPI Search Results */}
      <div style={{ marginTop: '14px' }}>
        <h3 style={{ color: '#9fd7ff', fontSize: '0.85rem', marginBottom: '8px' }}>ISAPI SEARCH RESULTS</h3>
        <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
          <input value={nvr.isapiFrom} onChange={e => nvr.setIsapiFrom(e.target.value)} style={{ flex: 1, background: '#000', color: '#9fd7ff', border: '1px solid #1f2d4a', padding: '4px', fontSize: '10px' }} placeholder='from' />
          <input value={nvr.isapiTo} onChange={e => nvr.setIsapiTo(e.target.value)} style={{ flex: 1, background: '#000', color: '#9fd7ff', border: '1px solid #1f2d4a', padding: '4px', fontSize: '10px' }} placeholder='to' />
        </div>
        <div style={{ border: '1px solid #1f2d4a', background: '#05070b', maxHeight: '160px', overflowY: 'auto', padding: '8px' }}>
          {nvr.isapiSearchResults.length === 0 && <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «🔎 ISAPI SEARCH RECORDS» у локальной цели.</div>}
          {nvr.isapiSearchResults.map((item, idx) => (
            <div key={`${item.endpoint}_${idx}`} style={{ borderBottom: '1px solid #111', padding: '6px 0', fontSize: '10px' }}>
              <div style={{ color: '#90b8d8', wordBreak: 'break-all' }}>{item.endpoint}</div>
              <div style={{ color: '#7fa9cb' }}>track: {item.trackId || '-'} | start: {item.startTime || '-'} | end: {item.endTime || '-'}</div>
              <div style={{ color: '#9fd7ff', wordBreak: 'break-all' }}>uri: {item.playbackUri || '-'}</div>
              {item.playbackUri && (
                <div style={{ display: 'flex', gap: '6px', marginTop: '6px' }}>
                  <button onClick={() => handleDownloadIsapiPlayback(item)} disabled={!isDownloadableRecord(item)}
                    style={{ background: isDownloadableRecord(item) ? '#1f3a2a' : '#1a1a1a', color: isDownloadableRecord(item) ? '#9fffc5' : '#666', border: isDownloadableRecord(item) ? '1px solid #38a169' : '1px solid #333', padding: '3px 6px', cursor: isDownloadableRecord(item) ? 'pointer' : 'not-allowed', fontSize: '10px' }}>
                    {isDownloadableRecord(item) ? '⬇ DOWNLOAD BY URI' : 'NO-DL (probe)'}
                  </button>
                  <button onClick={() => handleCaptureIsapiPlayback(item)}
                    style={{ background: '#12263d', color: '#9fd7ff', border: '1px solid #2f6aa3', padding: '3px 6px', cursor: 'pointer', fontSize: '10px' }}>
                    ◉ CAPTURE FALLBACK
                  </button>
                </div>
              )}
            </div>
          ))}
        </div>
      </div>


      {/* ONVIF Device Info */}
      <div style={{ marginTop: '14px' }}>
        <h3 style={{ color: '#00f0ff', fontSize: '0.85rem', marginBottom: '8px' }}>ONVIF DEVICE INFO</h3>
        <div style={{ border: '1px solid #222', background: '#050505', maxHeight: '160px', overflowY: 'auto', padding: '8px' }}>
          {!nvr.onvifDeviceInfo && <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «ℹ ONVIF DEVICE INFO» у локальной цели.</div>}
          {nvr.onvifDeviceInfo && (<>
            <div style={{ color: '#aaa', fontSize: '10px', marginBottom: '6px', wordBreak: 'break-all' }}>{nvr.onvifDeviceInfo.endpoint} [{nvr.onvifDeviceInfo.status}]</div>
            <pre style={{ margin: 0, color: '#a8ffb0', fontSize: '10px', whiteSpace: 'pre-wrap' }}>{nvr.onvifDeviceInfo.bodyPreview || 'empty'}</pre>
          </>)}
        </div>
      </div>


      {/* ONVIF Tokens */}
      <div style={{ marginTop: '14px' }}>
        <h3 style={{ color: '#b9ffcf', fontSize: '0.85rem', marginBottom: '8px' }}>ONVIF RECORDING TOKENS</h3>
        <div style={{ border: '1px solid #2a5a36', background: '#050b06', maxHeight: '130px', overflowY: 'auto', padding: '8px' }}>
          {nvr.onvifRecordingTokens.length === 0 && <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «🔎 ONVIF RECORDINGS» у локальной цели.</div>}
          {nvr.onvifRecordingTokens.map((item, idx) => (
            <div key={`${item.endpoint}_${item.token}_${idx}`} style={{ borderBottom: '1px solid #132418', padding: '6px 0' }}>
              <div style={{ color: '#88c89b', fontSize: '10px', wordBreak: 'break-all' }}>{item.endpoint}</div>
              <div style={{ color: '#b9ffcf', fontSize: '10px' }}>token: {item.token}</div>
              <button onClick={() => handleDownloadOnvifToken(item)} style={{ marginTop: '6px', background: '#1f3a2a', color: '#b9ffcf', border: '1px solid #38a169', padding: '3px 6px', cursor: 'pointer', fontSize: '10px' }}>⬇ DOWNLOAD TOKEN</button>
            </div>
          ))}
        </div>
      </div>


      {/* Archive Export Endpoints */}
      <div style={{ marginTop: '14px' }}>
        <h3 style={{ color: '#ffd27a', fontSize: '0.85rem', marginBottom: '8px' }}>ARCHIVE EXPORT ENDPOINTS</h3>
        <div style={{ border: '1px solid #3a2a1a', background: '#0b0805', maxHeight: '160px', overflowY: 'auto', padding: '8px' }}>
          {nvr.archiveProbeResults.length === 0 && <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «📦 PROBE EXPORT ENDPOINTS» у локальной цели.</div>}
          {nvr.archiveProbeResults.map((item, idx) => (
            <div key={`${item.endpoint}_${idx}`} style={{ borderBottom: '1px solid #22180f', padding: '6px 0' }}>
              <div style={{ fontSize: '10px', color: '#e9cda1' }}>{item.protocol} · {item.method}</div>
              <div style={{ fontSize: '10px', color: '#777', wordBreak: 'break-all' }}>{item.endpoint}</div>
              <div style={{ fontSize: '10px', color: item.status === 'detected' ? '#7dff9c' : item.status === 'not_detected' ? '#ffcc66' : '#ff5555' }}>
                {item.status}{item.statusCode ? ` (HTTP ${item.statusCode})` : ''}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
