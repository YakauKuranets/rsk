import React, { useEffect, useRef } from 'react';
import mpegts from 'mpegts.js';

export default function StreamPlayer({ streamUrl, cameraName, onRefresh, onClose }) {
  const videoRef = useRef(null);
  const playerRef = useRef(null);

  useEffect(() => {
    if (streamUrl && videoRef.current) {
      if (mpegts.getFeatureList().mseLivePlayback) {
        const player = mpegts.createPlayer({
          type: 'flv',
          isLive: true,
          url: streamUrl,
          hasAudio: false
        });

        player.attachMediaElement(videoRef.current);
        player.load();

        // Запускаем видео и ловим возможные ошибки автоплея
        player.play().catch(e => console.error('[PLAYER] Play error:', e));

        playerRef.current = player;
      } else {
        console.error('[PLAYER] MSE is not supported in this browser');
      }

      // Автоматическая очистка при смене URL или закрытии компонента
      return () => {
        if (playerRef.current) {
          playerRef.current.destroy();
          playerRef.current = null;
        }
      };
    }
  }, [streamUrl]);

  if (!streamUrl) return null;

  return (
    <div style={{ position: 'absolute', bottom: 20, left: 20, width: '520px', border: '2px solid #00f0ff', zIndex: 1000, backgroundColor: '#050505', boxShadow: '0 0 20px rgba(0,240,255,0.3)' }}>
      <div style={{ background: '#00f0ff', color: '#000', padding: '5px 8px', fontSize: '12px', display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontWeight: 'bold' }}>
        <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>LIVE: {cameraName}</span>
        <div style={{ display: 'flex', gap: '8px', marginLeft: '10px', flexShrink: 0 }}>
          <span
            onClick={onRefresh}
            style={{ cursor: 'pointer', padding: '2px 8px', backgroundColor: 'rgba(0,0,0,0.2)', borderRadius: '3px', fontSize: '11px', userSelect: 'none' }}
            title="Перезапустить поток"
          >
            ↻ ОБНОВИТЬ
          </span>
          <span
            onClick={onClose}
            style={{ cursor: 'pointer', padding: '2px 8px', backgroundColor: 'rgba(255,0,0,0.3)', borderRadius: '3px', fontSize: '11px', userSelect: 'none' }}
            title="Закрыть поток"
          >
            ✖ ЗАКРЫТЬ
          </span>
        </div>
      </div>

      {/* Контейнер видео с правильными пропорциями */}
      <video
        ref={videoRef}
        style={{ width: '100%', aspectRatio: '16/9', display: 'block', backgroundColor: '#000', objectFit: 'contain' }}
        muted
        autoPlay
      />
    </div>
  );
}
