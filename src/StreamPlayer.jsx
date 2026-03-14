import React, { useEffect, useRef } from 'react';

export default function StreamPlayer({ streamUrl, cameraName, onRefresh, onClose }) {
  const videoContainerRef = useRef(null);
  const playerRef = useRef(null);

  useEffect(() => {
    if (streamUrl && videoContainerRef.current) {
      videoContainerRef.current.innerHTML = '';

      const Jessibuca = window.Jessibuca;
      if (!Jessibuca) {
        console.warn('[PLAYER] Jessibuca is not loaded');
        return;
      }

      const player = new Jessibuca({
        container: videoContainerRef.current,
        videoBuffer: 0.2,
        isResize: true,
        hasAudio: false,
      });

      player.play(streamUrl);
      playerRef.current = player;

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
    <div style={{ position: 'absolute', bottom: 20, left: 20, width: '520px', border: '2px solid #00f0ff', zIndex: 1000, backgroundColor: '#000', boxShadow: '0 0 20px rgba(0,240,255,0.3)' }}>
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
      <div
        ref={videoContainerRef}
        style={{ width: '100%', aspectRatio: '16/9', backgroundColor: '#05070a' }}
      ></div>
    </div>
  );
}
