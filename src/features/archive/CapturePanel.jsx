// src/features/archive/CapturePanel.jsx
import { toast } from '../../utils/toast';


export default function CapturePanel({
  capture,             // весь хук useCapturePanel()
  handleCaptureArchive,
  handleDownloadHttp,
  activeTargetId,
  streamRtspUrl,
  activeCameraName,
}) {
  return (
    <div style={{ marginTop: '20px', border: '1px solid #ff9900', padding: '10px', backgroundColor: '#1a1100', marginBottom: '20px' }}>
      <h3 style={{ color: '#ff9900', marginTop: '0', fontSize: '0.9rem' }}>📦 ЗАХВАТ АРХИВА (FFmpeg / HTTP)</h3>
      <div style={{ fontSize: '10px', color: '#aa8833', marginBottom: '8px' }}>
        Введите RTSP, HTTP или MJPEG URL источника. FFmpeg захватит видео в MP4.
      </div>


      <input
        style={{ width: '100%', backgroundColor: '#000', border: '1px solid #ff9900', color: '#ff9900', padding: '8px', marginBottom: '6px', boxSizing: 'border-box', fontSize: '11px' }}
        placeholder='rtsp://admin:pass@192.168.1.100/Streaming/tracks/101'
        value={capture.captureUrl}
        onChange={e => capture.setCaptureUrl(e.target.value)}
      />


      <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
        <input
          style={{ flex: 2, backgroundColor: '#000', border: '1px solid #ff9900', color: '#ff9900', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
          placeholder='Имя файла (авто)'
          value={capture.captureFilename}
          onChange={e => capture.setCaptureFilename(e.target.value)}
        />
        <input
          type='number'
          style={{ flex: 1, backgroundColor: '#000', border: '1px solid #ff9900', color: '#ff9900', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
          placeholder='Сек'
          value={capture.captureDuration}
          onChange={e => capture.setCaptureDuration(parseInt(e.target.value) || 60)}
        />
      </div>


      <div style={{ display: 'flex', gap: '6px' }}>
        <button
          onClick={() => { if (!capture.captureUrl.trim()) return toast('Введите URL источника'); handleCaptureArchive(capture.captureUrl, capture.captureFilename || null, capture.captureDuration); }}
          style={{ flex: 1, backgroundColor: '#ff9900', color: '#000', border: 'none', padding: '8px', cursor: 'pointer', fontWeight: 'bold', fontSize: '11px' }}
        >🎬 ЗАХВАТ (FFmpeg)</button>
        <button
          onClick={() => { if (!capture.captureUrl.trim()) return toast('Введите URL для скачивания'); handleDownloadHttp(capture.captureUrl, { filenameHint: capture.captureFilename || null }); }}
          style={{ flex: 1, backgroundColor: '#1a4a1a', color: '#9f9', border: '1px solid #4a4', padding: '8px', cursor: 'pointer', fontWeight: 'bold', fontSize: '11px' }}
        >⬇ HTTP ПРЯМАЯ</button>
      </div>


      {activeTargetId && streamRtspUrl && streamRtspUrl !== 'hub' && (
        <button
          onClick={() => handleCaptureArchive(streamRtspUrl, `${activeCameraName.replace(/[^a-zA-Zа-яА-Я0-9]/g, '_')}_${Date.now()}.mp4`, capture.captureDuration)}
          style={{ width: '100%', marginTop: '6px', backgroundColor: '#4a3a1a', color: '#ffd27a', border: '1px solid #ff9900', padding: '8px', cursor: 'pointer', fontWeight: 'bold', fontSize: '11px' }}
        >📹 ЗАПИСАТЬ ТЕКУЩИЙ СТРИМ ({capture.captureDuration}с)</button>
      )}
    </div>
  );
}
