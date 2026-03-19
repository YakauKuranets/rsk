// src/features/archive/HubReconPanel.jsx
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../../utils/toast';


export default function HubReconPanel({
  hubRecon,            // весь хук useHubRecon()
  capture,             // весь хук useCapturePanel()
  hubConfig,           // { cookie }
  fuzzPath,            // string из appStore
  formatBytes,         // fn(bytes) -> string
  handleCaptureArchive // (url, hint, sec, headers) -> void
}) {
  return (
    <div style={{ border: '1px solid #00ff9c', padding: '10px', backgroundColor: '#001a0a', marginBottom: '20px', boxShadow: '0 0 10px rgba(0,255,156,0.15)' }}>
      <h3 style={{ color: '#00ff9c', marginTop: '0', fontSize: '0.9rem' }}>🔍 РАЗВЕДКА АРХИВА (HUB)</h3>
      <div style={{ fontSize: '10px', color: '#6b9', marginBottom: '8px' }}>
        Прощупывает все PHP-эндпоинты stream.example.local на наличие архивного доступа для конкретной камеры.
      </div>


      <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
        <input
          style={{ flex: 1, backgroundColor: '#000', border: '1px solid #00ff9c', color: '#00ff9c', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
          placeholder='User ID (напр. 1234)'
          value={hubRecon.reconUserId}
          onChange={e => hubRecon.setReconUserId(e.target.value)}
        />
        <input
          style={{ flex: 1, backgroundColor: '#000', border: '1px solid #00ff9c', color: '#00ff9c', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
          placeholder='Channel (0,1,2...)'
          value={hubRecon.reconChannelId}
          onChange={e => hubRecon.setReconChannelId(e.target.value)}
        />
        <input
          type='date'
          style={{ flex: 1, backgroundColor: '#000', border: '1px solid #00ff9c', color: '#00ff9c', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
          value={hubRecon.reconDate}
          onChange={e => hubRecon.setReconDate(e.target.value)}
        />
      </div>


      <button
        disabled={hubRecon.reconRunning}
        onClick={async () => {
          if (!hubRecon.reconUserId.trim()) return toast('Введите User ID камеры');
          hubRecon.setReconRunning(true);
          hubRecon.setReconResults([]);
          try {
            const results = await invoke('recon_hub_archive_routes', {
              userId: hubRecon.reconUserId,
              channelId: hubRecon.reconChannelId,
              adminHash: hubConfig.cookie.split('admin=')[1]?.split(';')[0]?.trim() || '',
              targetDate: hubRecon.reconDate || null,
              targetFtpPath: fuzzPath || null,
            });
            hubRecon.setReconResults(results);
          } catch (err) {
            toast(`Ошибка разведки: ${err}`);
          } finally {
            hubRecon.setReconRunning(false);
          }
        }}
        style={{ width: '100%', backgroundColor: hubRecon.reconRunning ? '#333' : '#00ff9c', color: '#000', border: 'none', padding: '8px', cursor: hubRecon.reconRunning ? 'wait' : 'pointer', fontWeight: 'bold', fontSize: '11px', letterSpacing: '1px' }}
      >
        {hubRecon.reconRunning ? '⏳ РАЗВЕДКА...' : '🔍 ЗАПУСТИТЬ РАЗВЕДКУ МАРШРУТОВ'}
      </button>


      {hubRecon.reconResults.length > 0 && (
        <div style={{ marginTop: '10px', border: '1px solid #00ff9c', background: '#000', maxHeight: '300px', overflowY: 'auto', padding: '6px' }}>
          <div style={{ color: '#00ff9c', fontSize: '10px', fontWeight: 'bold', marginBottom: '6px' }}>
            РЕЗУЛЬТАТЫ: {hubRecon.reconResults.length} маршрутов |{' '}
            {hubRecon.reconResults.filter(r => r.isVideo).length} видео |{' '}
            {hubRecon.reconResults.filter(r => r.isRedirect).length} редиректов
          </div>
          {hubRecon.reconResults.map((r, idx) => (
            <div key={idx} style={{ borderBottom: '1px solid #112', padding: '6px 0', opacity: r.verdict.includes('НЕ НАЙДЕНО') || r.verdict.includes('ПУСТО') ? 0.4 : 1 }}>
              <div style={{ fontSize: '10px', color: r.isVideo ? '#00ff9c' : r.isRedirect ? '#ffcc00' : '#888', fontWeight: r.isVideo ? 'bold' : 'normal' }}>{r.verdict}</div>
              <div style={{ fontSize: '9px', color: '#666', wordBreak: 'break-all' }}>{r.method} {r.url}</div>
              <div style={{ fontSize: '9px', color: '#555' }}>HTTP {r.statusCode} | {r.contentType || 'n/a'} | {r.contentLength > 0 ? formatBytes(r.contentLength) : '0'}</div>
              {r.bodyPreview && r.bodyPreview.length > 10 && !r.bodyPreview.startsWith('[') && (
                <div style={{ fontSize: '9px', color: '#444', marginTop: '2px', maxHeight: '30px', overflow: 'hidden' }}>{r.bodyPreview.substring(0, 150)}</div>
              )}
              {r.isVideo && (
                <button
                  onClick={() => {
                    capture.setCaptureUrl(r.url);
                    handleCaptureArchive(r.url, `recon_${hubRecon.reconUserId}_ch${hubRecon.reconChannelId}_${hubRecon.reconDate}.mp4`, capture.captureDuration, `Cookie: ${hubConfig.cookie}\r\nReferer: https://stream.example.local/stream/admin.php\r\n`);
                  }}
                  style={{ marginTop: '4px', background: '#1a4a1a', color: '#00ff9c', border: '1px solid #00ff9c', padding: '3px 8px', cursor: 'pointer', fontSize: '9px', fontWeight: 'bold' }}
                >🎬 ЗАХВАТИТЬ ЭТОТ ПОТОК</button>
              )}
              {r.isRedirect && r.redirectTo && (
                <button
                  onClick={() => { const fullUrl = r.redirectTo.startsWith('http') ? r.redirectTo : `https://stream.example.local${r.redirectTo}`; capture.setCaptureUrl(fullUrl); }}
                  style={{ marginTop: '4px', background: '#4a4a1a', color: '#ffcc00', border: '1px solid #ffcc00', padding: '3px 8px', cursor: 'pointer', fontSize: '9px' }}
                >↗️ СЛЕДОВАТЬ ЗА РЕДИРЕКТОМ</button>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
