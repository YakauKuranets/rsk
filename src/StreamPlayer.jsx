import React, { useEffect, useRef, useState } from 'react';
import mpegts from 'mpegts.js';
import { invoke } from '@tauri-apps/api/core';

export default function StreamPlayer({ streamUrl, cameraName, terminal, channel, hubCookie, onRefresh, onClose, onPlayArchive }) {
  const videoRef = useRef(null);
  const playerRef = useRef(null);

  const [tab, setTab] = useState('live');
  const [archiveDate, setArchiveDate] = useState(() => new Date().toISOString().split('T')[0]);
  const [records, setRecords] = useState([]);
  const [loadingArchive, setLoadingArchive] = useState(false);

  // НОВОЕ: Сохраняем текущий проигрываемый кусок архива для перемотки
  const [playingRecord, setPlayingRecord] = useState(null);
  // НОВЫЕ СОСТОЯНИЯ ДЛЯ ПОЛЗУНКА:
  const [seekOffsetMs, setSeekOffsetMs] = useState(0); // Запоминаем смещение при перемотке
  const [progressPercent, setProgressPercent] = useState(0); // Текущая ширина синей полосы

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
        player.play().catch(e => console.error("[PLAYER] Play error:", e));
        playerRef.current = player;
      }

      return () => {
        if (playerRef.current) {
          playerRef.current.destroy();
          playerRef.current = null;
        }
      };
    }
  }, [streamUrl]);

  const handleSearchArchive = async () => {
    if (!terminal) return alert('Ошибка: данные камеры не переданы в плеер! Проверьте App.jsx');

    setLoadingArchive(true);
    setRecords([]);

    try {
      if (terminal.type === 'hub') {
        // УМНЫЙ ПОИСК ПО ХАБУ (videodvor.by)
        const adminHash = hubCookie ? hubCookie.split('admin=')[1]?.split(';')[0]?.trim() : '';
        const results = await invoke('recon_hub_archive_routes', {
          userId: terminal.hub_id.toString(),
          channelId: channel ? channel.index.toString() : '0',
          adminHash: adminHash || '',
          targetDate: archiveDate,
          targetFtpPath: null,
        });

        // Отбираем только рабочие видео-роуты
        const videoRoutes = results.filter(r => r.isVideo).map(r => ({
          startTime: `${archiveDate}T00:00:00Z`,
          endTime: `${archiveDate}T23:59:59Z`,
          playbackUri: r.url,
          label: r.verdict || 'Запись Хаба'
        }));

        if (videoRoutes.length === 0) alert('Записей на Хабе за эту дату не найдено.');
        setRecords(videoRoutes);

      } else {
        // ПОИСК ПО ПРЯМОЙ КАМЕРЕ (ISAPI)
        const fromTime = `${archiveDate}T00:00:00Z`;
        const toTime = `${archiveDate}T23:59:59Z`;
        const result = await invoke('search_isapi_recordings', {
          host: terminal.host,
          login: terminal.login || 'admin',
          pass: terminal.password || '',
          fromTime,
          toTime,
        });

        if (!result || result.length === 0) alert('Нет записей за эту дату (ISAPI).');
        setRecords(result || []);
      }
    } catch(e) {
      alert('Ошибка поиска: ' + e);
    } finally {
      setLoadingArchive(false);
    }
  };

  const formatTime = (isoString) => {
    if (!isoString) return '';
    const date = new Date(isoString);
    return date.toLocaleTimeString('ru-RU', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  };

  if (!streamUrl) return null;

  return (
    <div style={{ position: 'absolute', bottom: 20, left: 20, width: '520px', border: '2px solid #00f0ff', zIndex: 1000, backgroundColor: '#050505', boxShadow: '0 0 20px rgba(0,240,255,0.3)', display: 'flex', flexDirection: 'column' }}>

      {/* Шапка плеера с вкладками */}
      <div style={{ background: '#00f0ff', color: '#000', padding: '0', fontSize: '12px', display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontWeight: 'bold' }}>
        <div style={{ display: 'flex' }}>
          <div onClick={() => setTab('live')} style={{ padding: '6px 12px', cursor: 'pointer', backgroundColor: tab === 'live' ? 'transparent' : 'rgba(0,0,0,0.2)' }}>
            ⏺ LIVE
          </div>
          <div onClick={() => setTab('archive')} style={{ padding: '6px 12px', cursor: 'pointer', backgroundColor: tab === 'archive' ? 'transparent' : 'rgba(0,0,0,0.2)' }}>
            📁 АРХИВ
          </div>
        </div>

        <div style={{ display: 'flex', gap: '8px', marginRight: '8px' }}>
          <span onClick={onRefresh} style={{ cursor: 'pointer', padding: '2px 8px', backgroundColor: 'rgba(0,0,0,0.2)', borderRadius: '3px', fontSize: '11px' }}>↻ ОБНОВИТЬ</span>
          <span onClick={onClose} style={{ cursor: 'pointer', padding: '2px 8px', backgroundColor: 'rgba(255,0,0,0.3)', borderRadius: '3px', fontSize: '11px' }}>✖ ЗАКРЫТЬ</span>
        </div>
      </div>

      {/* Контейнер видео */}
      <video
        ref={videoRef}
        style={{ width: '100%', aspectRatio: '16/9', display: 'block', backgroundColor: '#000', objectFit: 'contain' }}
        muted
        autoPlay
        onTimeUpdate={(e) => {
          if (playingRecord && tab === 'archive') {
            const startMs = new Date(playingRecord.startTime).getTime();
            const endMs = new Date(playingRecord.endTime).getTime();
            const durationMs = endMs - startMs;

            // Текущее время = смещение от перемотки + сколько секунд прошло с момента запуска потока
            const currentMs = seekOffsetMs + (e.target.currentTime * 1000);
            const percent = Math.min(100, Math.max(0, (currentMs / durationMs) * 100));

            setProgressPercent(percent);
          }
        }}
      />

      {/* Кастомный Таймлайн для перемотки Архива */}
      {playingRecord && tab === 'archive' && (
        <div style={{ backgroundColor: '#111', padding: '5px 10px', borderTop: '1px solid #333' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '9px', color: '#00f0ff', marginBottom: '4px' }}>
            <span>{formatTime(playingRecord.startTime)}</span>
            <span>НАЖМИТЕ НА ПОЛОСУ ДЛЯ ПЕРЕМОТКИ</span>
            <span>{formatTime(playingRecord.endTime)}</span>
          </div>
          <div
            onClick={(e) => {
              if (!playingRecord || !playingRecord.playbackUri) return;

              // Высчитываем процент клика
              const rect = e.currentTarget.getBoundingClientRect();
              const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));

              // Высчитываем миллисекунды
              const startMs = new Date(playingRecord.startTime).getTime();
              const endMs = new Date(playingRecord.endTime).getTime();
              const targetMs = startMs + (endMs - startMs) * percent;

              // Обновляем состояния для UI моментально
              setSeekOffsetMs(targetMs - startMs);
              setProgressPercent(percent * 100);

              // Форматируем в ISAPI формат: YYYYMMDDTHHMMSSZ
              const targetDate = new Date(targetMs);
              const isapiTime = targetDate.toISOString().replace(/[-:]/g, '').split('.')[0] + 'Z';

              // Подменяем starttime в URL
              const newUri = playingRecord.playbackUri.replace(/starttime=[^&]+/i, `starttime=${isapiTime}`);

              console.log('[SEEK] Перемотка на:', targetDate.toLocaleString(), newUri);
              onPlayArchive(newUri);
            }}
            style={{ width: '100%', height: '12px', backgroundColor: '#333', cursor: 'pointer', position: 'relative', borderRadius: '2px' }}
          >
            {/* АНИМИРОВАННАЯ визуальная подсказка */}
            <div style={{ position: 'absolute', top: 0, left: 0, height: '100%', width: `${progressPercent}%`, backgroundColor: '#00f0ff', pointerEvents: 'none' }} />
          </div>
        </div>
      )}

      {/* Панель архива (Открывается при выборе вкладки) */}
      {tab === 'archive' && (
        <div style={{ backgroundColor: '#0a0a0c', borderTop: '1px solid #00f0ff', padding: '10px' }}>
          <div style={{ display: 'flex', gap: '8px', marginBottom: '10px' }}>
            <input
              type="date"
              value={archiveDate}
              onChange={(e) => setArchiveDate(e.target.value)}
              style={{ flex: 1, backgroundColor: '#000', color: '#00f0ff', border: '1px solid #00f0ff', padding: '6px', fontSize: '12px', colorScheme: 'dark' }}
            />
            <button
              onClick={handleSearchArchive}
              disabled={loadingArchive}
              style={{ backgroundColor: '#1a4a4a', color: '#00f0ff', border: '1px solid #00f0ff', padding: '6px 15px', cursor: 'pointer', fontWeight: 'bold', fontSize: '11px' }}
            >
              {loadingArchive ? 'ПОИСК...' : 'НАЙТИ'}
            </button>
          </div>

          <div style={{ maxHeight: '150px', overflowY: 'auto', border: '1px solid #222', padding: '5px' }}>
            {records.length === 0 && !loadingArchive && <div style={{ color: '#555', fontSize: '11px', textAlign: 'center', padding: '10px' }}>Нет записей за эту дату</div>}
            {records.map((rec, idx) => (
              <div key={idx} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '6px', borderBottom: '1px solid #111' }}>
                <span style={{ color: '#aaa', fontSize: '11px' }}>
                  {rec.label ? rec.label : `${formatTime(rec.startTime)} — ${formatTime(rec.endTime)}`}
                </span>
                <button
                  onClick={() => {
                    setPlayingRecord(rec);
                    setSeekOffsetMs(0); // Сброс при новом видео
                    setProgressPercent(0);
                    onPlayArchive(rec.playbackUri);
                  }}
                  disabled={!rec.playbackUri}
                  style={{ backgroundColor: '#00f0ff', color: '#000', border: 'none', padding: '3px 8px', cursor: 'pointer', fontSize: '10px', fontWeight: 'bold', opacity: rec.playbackUri ? 1 : 0.5 }}
                >
                  ▶ ПЛЕЙ
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
