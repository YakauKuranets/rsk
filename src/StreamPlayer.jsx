import React, { useEffect, useRef, useState } from 'react';
import mpegts from 'mpegts.js';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

export default function StreamPlayer({ streamUrl, cameraName, terminal, channel, hubCookie, onRefresh, onClose, onPlayArchive }) {
  const containerRef = useRef(null);
  const videoRef = useRef(null);
  const playerRef = useRef(null);
  const timelineContainerRef = useRef(null);

  const [tab, setTab] = useState('live');
  const [archiveDate, setArchiveDate] = useState(() => new Date().toISOString().split('T')[0]);
  const [records, setRecords] = useState([]);
  const [loadingArchive, setLoadingArchive] = useState(false);

  // Состояния для Архива и Ползунка
  const [playingRecord, setPlayingRecord] = useState(null);
  const [seekOffsetMs, setSeekOffsetMs] = useState(0);
  const [progressPercent, setProgressPercent] = useState(0);

  // Стейты для ИИ и Таймлайна
  const [aiAnalyzing, setAiAnalyzing] = useState(false);
  const [archiveEvents, setArchiveEvents] = useState([]);
  const [timelineZoom, setTimelineZoom] = useState(1.0);
  
  // Новые стейты для UI
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isPinned, setIsPinned] = useState(false);

  // Слушатель перехода в полноэкранный режим
  useEffect(() => {
    const handleFullscreenChange = () => {
      setIsFullscreen(!!document.fullscreenElement);
    };
    document.addEventListener('fullscreenchange', handleFullscreenChange);
    return () => document.removeEventListener('fullscreenchange', handleFullscreenChange);
  }, []);

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


  const startAiAnalysis = async () => {
    if (!playingRecord || !playingRecord.playbackUri) return;
    setAiAnalyzing(true);
    setArchiveEvents([]); 

    const startMs = new Date(playingRecord.startTime).getTime();
    const endMs = new Date(playingRecord.endTime).getTime();
    const durationMs = endMs - startMs;

    try {
      await invoke('start_archive_analysis', {
        playbackUri: playingRecord.playbackUri,
        durationMs: durationMs
      });
    } catch (err) {
      alert("Ошибка запуска ИИ: " + err);
      setAiAnalyzing(false);
    }
  };

  useEffect(() => {
    let unlistenEvent;
    let unlistenDone;

    const setupListeners = async () => {
      unlistenEvent = await listen('ai-archive-event', (event) => {
        const { timestamp_ms, class: objClass } = event.payload;
        if (!playingRecord) return;

        const startMs = new Date(playingRecord.startTime).getTime();
        const absoluteTime = new Date(startMs + timestamp_ms).toISOString();

        setArchiveEvents(prev => [...prev, { time: absoluteTime, type: objClass === 'person' ? 'motion' : 'line' }]);
      });

      unlistenDone = await listen('ai-archive-done', () => {
        setAiAnalyzing(false);
      });
    };

    if (playingRecord) setupListeners();

    return () => {
      if (unlistenEvent) unlistenEvent();
      if (unlistenDone) unlistenDone();
    };
  }, [playingRecord]);

  // Плавный зум таймлайна колесиком мыши
  useEffect(() => {
    const container = timelineContainerRef.current;
    if (!container) return;

    const handleWheel = (e) => {
      e.preventDefault(); // Блокируем стандартную прокрутку страницы

      setTimelineZoom(prevZoom => {
        // Определяем направление скролла (вверх - приблизить, вниз - отдалить)
        const zoomFactor = e.deltaY < 0 ? 1.25 : 0.8;
        // Ограничиваем зум: от 1x (весь архив) до 1000x (детально до секунд)
        const newZoom = Math.max(1, Math.min(prevZoom * zoomFactor, 1000));

        // Математика для сохранения позиции под курсором мыши
        const rect = container.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const scrollX = container.scrollLeft;
        const currentTotalWidth = container.scrollWidth;
        const ratio = (mouseX + scrollX) / currentTotalWidth;

        // Применяем новый скролл после обновления ширины (в следующем кадре)
        setTimeout(() => {
          if (timelineContainerRef.current) {
            const newTotalWidth = timelineContainerRef.current.scrollWidth;
            timelineContainerRef.current.scrollLeft = (newTotalWidth * ratio) - mouseX;
          }
        }, 0);

        return newZoom;
      });
    };

    container.addEventListener('wheel', handleWheel, { passive: false });
    return () => container.removeEventListener('wheel', handleWheel);
  }, [playingRecord, tab]);

  const stopAiAnalysis = () => {
    setAiAnalyzing(false);
    // В будущем здесь будет: invoke('stop_archive_analysis');
  };

  const handleSearchArchive = async () => {
    if (!terminal) return alert('Ошибка: данные камеры не переданы в плеер!');

    setLoadingArchive(true);
    setRecords([]);

    try {
      if (terminal.type === 'hub') {
        const adminHash = hubCookie ? hubCookie.split('admin=')[1]?.split(';')[0]?.trim() : '';
        const results = await invoke('recon_hub_archive_routes', {
          userId: terminal.hub_id.toString(),
          channelId: channel ? channel.index.toString() : '0',
          adminHash: adminHash || '',
          targetDate: archiveDate,
          targetFtpPath: null,
        });

        const videoRoutes = results.filter(r => r.isVideo).map(r => ({
          startTime: `${archiveDate}T00:00:00Z`,
          endTime: `${archiveDate}T23:59:59Z`,
          playbackUri: r.url,
          label: r.verdict || 'Запись Хаба'
        }));

        if (videoRoutes.length === 0) alert('Записей на Хабе за эту дату не найдено.');
        setRecords(videoRoutes);

      } else {
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

  const handleTimeUpdate = () => {
    if (playingRecord && tab === 'archive' && videoRef.current) {
      const startMs = new Date(playingRecord.startTime).getTime();
      const endMs = new Date(playingRecord.endTime).getTime();
      const durationMs = endMs - startMs;

      if (durationMs > 0) {
        const currentMs = seekOffsetMs + (videoRef.current.currentTime * 1000);
        const percent = Math.min(100, Math.max(0, (currentMs / durationMs) * 100));
        setProgressPercent(percent);
      }
    }
  };

  // --- НОВЫЕ ФУНКЦИИ УПРАВЛЕНИЯ ---
  const toggleFullscreen = () => {
    if (!document.fullscreenElement) {
      containerRef.current?.requestFullscreen().catch(err => console.error(err));
    } else {
      document.exitFullscreen().catch(err => console.error(err));
    }
  };

  const togglePin = async () => {
    try {
      const appWindow = getCurrentWindow();
      const newState = !isPinned;
      await appWindow.setAlwaysOnTop(newState);
      setIsPinned(newState); // Если всё ок, меняем цвет кнопки
    } catch (err) {
      alert("Ошибка закрепления окна Tauri:\n" + err);
      console.error('Tauri window pin error:', err);
    }
  };

  if (!streamUrl) return null;

  return (
    <div 
      ref={containerRef}
      style={{ 
        position: isFullscreen ? 'fixed' : 'absolute', 
        bottom: isFullscreen ? 0 : 20, 
        left: isFullscreen ? 0 : 20, 
        width: isFullscreen ? '100vw' : '520px', 
        height: isFullscreen ? '100vh' : 'auto',
        border: isFullscreen ? 'none' : '2px solid #00f0ff', 
        zIndex: 9999, 
        backgroundColor: '#050505', 
        boxShadow: isFullscreen ? 'none' : '0 0 20px rgba(0,240,255,0.3)', 
        display: 'flex', 
        flexDirection: 'column' 
      }}
    >
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

        <div style={{ display: 'flex', gap: '6px', marginRight: '8px' }}>
          <span 
            onClick={togglePin} 
            style={{ 
              cursor: 'pointer', 
              padding: '2px 8px', 
              backgroundColor: isPinned ? 'rgba(0,240,255,0.4)' : 'rgba(0,0,0,0.2)', 
              borderRadius: '3px', 
              fontSize: '11px',
              color: isPinned ? '#fff' : 'inherit'
            }} 
            title="Закрепить окно поверх остальных программ"
          >
            {isPinned ? '📌 ОТКРЕПИТЬ' : '📌 ПОВЕРХ ОКОН'}
          </span>
          <span onClick={toggleFullscreen} style={{ cursor: 'pointer', padding: '2px 8px', backgroundColor: 'rgba(0,0,0,0.2)', borderRadius: '3px', fontSize: '11px' }} title="На весь экран">
            {isFullscreen ? '🗗 СВЕРНУТЬ' : '⛶ ПОЛНЫЙ'}
          </span>
          <span onClick={onRefresh} style={{ cursor: 'pointer', padding: '2px 8px', backgroundColor: 'rgba(0,0,0,0.2)', borderRadius: '3px', fontSize: '11px' }} title="Перезапустить поток">
            ↻
          </span>
          <span onClick={onClose} style={{ cursor: 'pointer', padding: '2px 8px', backgroundColor: 'rgba(255,0,0,0.3)', borderRadius: '3px', fontSize: '11px' }}>
            ✖
          </span>
        </div>
      </div>

      {/* Контейнер видео */}
      <video
        ref={videoRef}
        style={{ width: '100%', flexGrow: 1, maxHeight: isFullscreen ? (tab === 'archive' ? 'calc(100vh - 250px)' : 'calc(100vh - 30px)') : 'auto', aspectRatio: isFullscreen ? 'auto' : '16/9', display: 'block', backgroundColor: '#000', objectFit: 'contain' }}
        muted
        autoPlay
        onTimeUpdate={handleTimeUpdate}
      />

      {/* Продвинутый Таймлайн с Mouse Wheel Zoom */}
      {playingRecord && tab === 'archive' && (
        <div style={{ backgroundColor: '#111', padding: '5px 10px', borderTop: '1px solid #333' }}>

          {/* Панель управления */}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
            <button
              onClick={aiAnalyzing ? stopAiAnalysis : startAiAnalysis}
              style={{
                backgroundColor: aiAnalyzing ? '#ff0044' : '#7000ff',
                color: '#fff', border: aiAnalyzing ? '1px solid #ff0044' : '1px solid #a055ff',
                padding: '4px 10px', fontSize: '10px', fontWeight: 'bold', cursor: 'pointer', borderRadius: '3px',
                boxShadow: aiAnalyzing ? '0 0 8px #ff0044' : '0 0 8px #7000ff'
              }}
            >
              {aiAnalyzing ? '🛑 ОСТАНОВИТЬ ИИ' : '🧠 УМНЫЙ АНАЛИЗ'}
            </button>

            <span style={{ color: '#aaa', fontSize: '10px' }}>
              ЗУМ: {timelineZoom.toFixed(1)}x (Крутите колесико мыши по полосе)
            </span>
          </div>

          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '9px', color: '#00f0ff', marginBottom: '4px' }}>
            <span>{formatTime(playingRecord.startTime)}</span>
            <span>{formatTime(playingRecord.endTime)}</span>
          </div>

          {/* Контейнер таймлайна с Ref для перехвата скролла */}
          <div
            ref={timelineContainerRef}
            style={{ width: '100%', overflowX: 'auto', overflowY: 'hidden', backgroundColor: '#000', border: '1px solid #333', position: 'relative' }}
          >
            <div
              onClick={(e) => {
                // ПЕРЕМОТКА: Разрешена всегда, даже при работающем ИИ!
                if (!playingRecord || !playingRecord.playbackUri) return;
                const rect = e.currentTarget.getBoundingClientRect();
                const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
                const startMs = new Date(playingRecord.startTime).getTime();
                const endMs = new Date(playingRecord.endTime).getTime();
                const targetMs = startMs + (endMs - startMs) * percent;

                setSeekOffsetMs(targetMs - startMs);
                setProgressPercent(percent * 100);

                const targetDate = new Date(targetMs);
                const isapiTime = targetDate.toISOString().replace(/[-:]/g, '').split('.')[0] + 'Z';
                const newUri = playingRecord.playbackUri.replace(/starttime=[^&]+/i, `starttime=${isapiTime}`);

                onPlayArchive(newUri);
              }}
              style={{
                width: `${timelineZoom * 100}%`,
                height: '24px',
                backgroundColor: '#222', cursor: 'pointer', position: 'relative'
              }}
            >
              {/* Полоса просмотренного */}
              <div style={{ position: 'absolute', top: 0, left: 0, height: '100%', width: `${progressPercent}%`, backgroundColor: '#00f0ff', pointerEvents: 'none', transition: 'width 0.2s linear', opacity: 0.3 }} />

              {/* Метки событий от ИИ */}
              {archiveEvents.map((evt, idx) => {
                const startMs = new Date(playingRecord.startTime).getTime();
                const endMs = new Date(playingRecord.endTime).getTime();
                const evtMs = new Date(evt.time).getTime();
                const posPercent = ((evtMs - startMs) / (endMs - startMs)) * 100;

                if (posPercent >= 0 && posPercent <= 100) {
                  return (
                    <div
                      key={idx}
                      style={{
                        position: 'absolute', left: `${posPercent}%`, top: 0, height: '100%', width: '2px',
                        backgroundColor: evt.type === 'motion' ? '#ff0044' : '#ffaa00',
                        boxShadow: `0 0 5px ${evt.type === 'motion' ? '#ff0044' : '#ffaa00'}`,
                        pointerEvents: 'none'
                      }}
                    />
                  );
                }
                return null;
              })}
            </div>
          </div>
        </div>
      )}

      {/* Панель архива */}
      {tab === 'archive' && (
        <div style={{ backgroundColor: '#0a0a0c', borderTop: '1px solid #00f0ff', padding: '10px' }}>
          <div style={{ display: 'flex', gap: '8px', marginBottom: '10px', maxWidth: '500px' }}>
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

          <div style={{ maxHeight: isFullscreen ? '200px' : '150px', overflowY: 'auto', border: '1px solid #222', padding: '5px' }}>
            {records.length === 0 && !loadingArchive && <div style={{ color: '#555', fontSize: '11px', textAlign: 'center', padding: '10px' }}>Нет записей за эту дату</div>}
            {records.map((rec, idx) => (
              <div key={idx} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '6px', borderBottom: '1px solid #111' }}>
                <span style={{ color: '#aaa', fontSize: '11px' }}>
                  {rec.label ? rec.label : `${formatTime(rec.startTime)} — ${formatTime(rec.endTime)}`}
                </span>
                <button
                  onClick={() => {
                    setPlayingRecord(rec);
                    setSeekOffsetMs(0);
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
