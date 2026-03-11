import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

/*
 * NEMESIS ARCHIVE TERMINAL
 * Layout: точная копия Hikvision download.asp
 * Left panel: камера, тип файла, тип потока, время начала, время окончания, кнопка Поиск
 * Right panel: таблица (№, имя файла, время начала, время окончания, размер, прогресс)
 * Style: cyberpunk (scan-lines, neon, dark)
 *
 * Props: target = { host, login, password, name, channels }, onClose = fn
 */
export default function NemesisArchiveTerminal({ target, onClose }) {
  const [phase, setPhase] = useState('connecting');
  const [statusText, setStatusText] = useState('УСТАНОВКА СВЯЗИ...');
  const [camera, setCamera] = useState('101');
  const [fileType, setFileType] = useState('all');
  const [streamType, setStreamType] = useState('main');
  const [timeFrom, setTimeFrom] = useState('');
  const [timeTo, setTimeTo] = useState('');
  const [records, setRecords] = useState([]);
  const [scanning, setScanning] = useState(false);
  const [bulkRunning, setBulkRunning] = useState(false);
  const [activeDownloads, setActiveDownloads] = useState({});
  const [selectedIndexes, setSelectedIndexes] = useState({});
  const [logs, setLogs] = useState([]);
  const logRef = useRef(null);
  const bootRef = useRef(false);

  const log = (msg, type = 'info') => {
    const ts = new Date().toLocaleTimeString('ru-RU', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    setLogs(p => [...p.slice(-60), { ts, msg, type }]);
  };

  useEffect(() => { if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight; }, [logs]);

  useEffect(() => {
    const now = new Date();
    const fmt = d => `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
    setTimeFrom(`${fmt(now)} 00:00:00`);
    setTimeTo(`${fmt(now)} 23:59:59`);
    if (target.channels?.length) setCamera(String(target.channels[0]?.index || target.channels[0]?.id || '101'));
  }, []);

  useEffect(() => {
    if (bootRef.current) return; bootRef.current = true;
    (async () => {
      log('\u2622 NEMESIS ARCHIVE LINK INITIATED', 'sys');
      log(`\u0426\u0435\u043b\u044c: ${target.name || target.host}`, 'sys');
      await new Promise(r => setTimeout(r, 400));
      log('\u0417\u043e\u043d\u0434\u0438\u0440\u043e\u0432\u0430\u043d\u0438\u0435 \u043f\u043e\u0440\u0442\u0430 2019...', 'info');
      try {
        const info = await invoke('fetch_nvr_device_info', { host: target.host, login: target.login || 'admin', pass: target.password || '' });
        log(`\u0423\u0437\u0435\u043b: ${info?.bodyPreview?.substring(0, 50) || 'OK'}`, 'ok');
      } catch (e) { log(`Device info: ${e}`, 'warn'); }
      setPhase('ready'); setStatusText('\u041a\u0410\u041d\u0410\u041b \u0410\u041a\u0422\u0418\u0412\u0415\u041d');
      log('\ud83d\udd17 \u0410\u0420\u0425\u0418\u0412\u041d\u042b\u0419 \u041a\u041e\u041d\u0422\u0423\u0420 \u0413\u041e\u0422\u041e\u0412', 'ok');
    })();
  }, []);

  const isDownloadableRecord = (item) => (typeof item?.downloadable === 'boolean' ? item.downloadable : Boolean(item?.playbackUri));
  const isPlayableRecord = (item) => (typeof item?.playable === 'boolean' ? item.playable : Boolean(item?.playbackUri));
  const normalizePlaybackUri = (uri) => String(uri || '').replace(/&amp;/g, '&').trim();
  const getIsapiFilenameHint = (uri, fallback = 'capture.mp4') => {
    const clean = normalizePlaybackUri(uri);
    const sanitize = (name) => String(name || '')
      .replace(/[\/:*?"<>|\u0000-\u001F]/g, '_')
      .replace(/\s+/g, '_')
      .replace(/_+/g, '_')
      .replace(/^\.+/, '')
      .slice(0, 180);

    const fromName = clean.match(/[?&]name=([^&]+)/i)?.[1];
    if (fromName) {
      const decoded = (() => { try { return decodeURIComponent(fromName.replace(/\+/g, '%20')); } catch { return fromName; } })();
      const safe = sanitize(decoded);
      const base = safe || fallback;
      return /\.[a-z0-9]{2,5}$/i.test(base) ? base : `${base}.mp4`;
    }
    const tail = clean.split('/').pop()?.split('?')[0];
    const safeTail = sanitize(tail);
    const base = safeTail || fallback;
    return /\.[a-z0-9]{2,5}$/i.test(base) ? base : `${base}.mp4`;
  };
  const getIsapiCaptureDurationSeconds = (item) => {
    const parseTs = (v) => {
      if (!v) return null;
      const n = new Date(v).getTime();
      return Number.isFinite(n) ? n : null;
    };
    const start = parseTs(item?.startTime);
    const end = parseTs(item?.endTime);
    if (start && end && end > start) return Math.min(1800, Math.max(30, Math.floor((end - start) / 1000) + 15));
    return 120;
  };

  const parseTimeToUtcMs = (value) => {
    const raw = String(value || '').trim();
    if (!raw) return null;
    const compact = raw.match(/^(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})Z?$/i);
    if (compact) {
      const [, y, mo, d, h, mi, s] = compact;
      const ms = Date.UTC(Number(y), Number(mo) - 1, Number(d), Number(h), Number(mi), Number(s));
      return Number.isFinite(ms) ? ms : null;
    }
    const normalized = /Z$/i.test(raw) ? raw : `${raw.replace(' ', 'T')}Z`;
    const ms = new Date(normalized).getTime();
    return Number.isFinite(ms) ? ms : null;
  };

  const fmtCompactUtc = (ms) => {
    const d = new Date(ms);
    const pad = (n) => String(n).padStart(2, '0');
    return `${d.getUTCFullYear()}${pad(d.getUTCMonth() + 1)}${pad(d.getUTCDate())}T${pad(d.getUTCHours())}${pad(d.getUTCMinutes())}${pad(d.getUTCSeconds())}Z`;
  };

  const fmtIsoUtc = (ms) => {
    const d = new Date(ms);
    const pad = (n) => String(n).padStart(2, '0');
    return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}T${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}:${pad(d.getUTCSeconds())}Z`;
  };

  const withPlaybackUriWindow = (uri, startMs, endMs) => {
    const clean = normalizePlaybackUri(uri);
    if (!clean) return clean;
    const start = fmtCompactUtc(startMs);
    const end = fmtCompactUtc(endMs);
    const hasQuery = clean.includes('?');
    const hasStart = /([?&]starttime=)[^&]*/i.test(clean);
    const hasEnd = /([?&]endtime=)[^&]*/i.test(clean);
    let next = clean;
    if (hasStart) next = next.replace(/([?&]starttime=)[^&]*/i, `$1${start}`);
    if (hasEnd) next = next.replace(/([?&]endtime=)[^&]*/i, `$1${end}`);
    if (!hasStart) next += `${hasQuery ? '&' : '?'}starttime=${start}`;
    if (!hasEnd) next += `${next.includes('?') ? '&' : '?'}endtime=${end}`;
    return next;
  };

  const splitPlaybackUriByMinutes = (uri, chunkMinutes = 30, fallbackStartTime, fallbackEndTime) => {
    const clean = normalizePlaybackUri(uri);
    const startMatch = clean.match(/[?&]starttime=([^&]+)/i);
    const endMatch = clean.match(/[?&]endtime=([^&]+)/i);
    const startMs = parseTimeToUtcMs(startMatch?.[1] || fallbackStartTime);
    const endMs = parseTimeToUtcMs(endMatch?.[1] || fallbackEndTime);
    if (!clean || !startMs || !endMs || endMs <= startMs) return [clean];

    const chunkMs = Math.max(1, Number(chunkMinutes) || 30) * 60 * 1000;
    if ((endMs - startMs) <= chunkMs) return [withPlaybackUriWindow(clean, startMs, endMs)];

    const chunks = [];
    let cursor = startMs;
    while (cursor < endMs) {
      const next = Math.min(cursor + chunkMs, endMs);
      chunks.push(withPlaybackUriWindow(clean, cursor, next));
      cursor = next;
    }
    return chunks.length ? chunks : [withPlaybackUriWindow(clean, startMs, endMs)];
  };

  const splitRecordIntoChunks = (item, chunkMinutes = 30) => {
    if (!item) return [item];
    const baseUri = normalizePlaybackUri(item.playbackUri || '');
    const startMs = parseTimeToUtcMs(item.startTime);
    const endMs = parseTimeToUtcMs(item.endTime);
    const fallbackStart = startMs ? fmtCompactUtc(startMs) : undefined;
    const fallbackEnd = endMs ? fmtCompactUtc(endMs) : undefined;

    const chunkUris = baseUri
      ? splitPlaybackUriByMinutes(baseUri, chunkMinutes, fallbackStart, fallbackEnd)
      : [baseUri];

    if (chunkUris.length <= 1 && (!startMs || !endMs)) return [item];

    const chunks = [];
    const chunkMs = Math.max(1, Number(chunkMinutes) || 30) * 60 * 1000;
    let ranges = [];
    if (startMs && endMs && endMs > startMs) {
      let cursor = startMs;
      while (cursor < endMs) {
        const next = Math.min(cursor + chunkMs, endMs);
        ranges.push([cursor, next]);
        cursor = next;
      }
    } else {
      ranges = chunkUris.map(() => [startMs, endMs]);
    }

    const total = Math.max(chunkUris.length, ranges.length);
    for (let i = 0; i < total; i += 1) {
      const [rs, re] = ranges[i] || ranges[ranges.length - 1] || [startMs, endMs];
      chunks.push({
        ...item,
        playbackUri: chunkUris[i] || chunkUris[chunkUris.length - 1] || item.playbackUri,
        startTime: rs ? fmtIsoUtc(rs) : item.startTime,
        endTime: re ? fmtIsoUtc(re) : item.endTime,
        chunkIndex: i + 1,
        chunkTotal: total,
      });
    }

    return chunks.length ? chunks : [item];
  };

  const handleSearch = async () => {
    if (bulkRunning) {
      log('⚠ Дождитесь завершения BULK очереди перед новым поиском', 'warn');
      return;
    }
    if (Object.values(activeDownloads).some((st) => st === 'working')) {
      log('⚠ Дождитесь завершения активных загрузок перед новым поиском', 'warn');
      return;
    }
    setScanning(true); setRecords([]); setSelectedIndexes({}); setActiveDownloads({}); setPhase('scanning'); setStatusText('\u0421\u041a\u0410\u041d\u0418\u0420\u041e\u0412\u0410\u041d\u0418\u0415...');
    const from = timeFrom.replace(' ', 'T') + (timeFrom.includes('Z') ? '' : 'Z');
    const to = timeTo.replace(' ', 'T') + (timeTo.includes('Z') ? '' : 'Z');
    log(`\ud83d\udd0e \u041f\u041e\u0418\u0421\u041a: ${timeFrom} \u2192 ${timeTo} | cam:${camera}`, 'info');
    try {
      const channelNum = Number(String(camera).replace(/[^0-9]/g, ''));
      const cameraChannelId = Number.isFinite(channelNum) && channelNum > 0
        ? (channelNum >= 100 ? Math.floor(channelNum / 100) : channelNum)
        : undefined;
      const items = await invoke('search_isapi_recordings', {
        host: target.host,
        login: target.login || 'admin',
        pass: target.password || '',
        fromTime: from,
        toTime: to,
        cameraChannelId,
        streamType: 1,
      });
      const filtered = (items || []).filter(i => i.playbackUri || i.startTime);
      const expanded = filtered.flatMap((item) => splitRecordIntoChunks(item, 30));
      const downloadableCount = expanded.filter(isDownloadableRecord).length;
      const playableCount = expanded.filter(isPlayableRecord).length;
      const maxConfidence = expanded.reduce((m, x) => Math.max(m, Number(x?.confidence ?? 0) || 0), 0);
      const splitExtra = Math.max(expanded.length - filtered.length, 0);
      setRecords(expanded); setPhase('ready'); setStatusText(`НАЙДЕНО: ${expanded.length}`);
      log(`✅ Результат: ${expanded.length} записей | playable=${playableCount} | downloadable=${downloadableCount} | maxConf=${maxConfidence}`, 'ok');
      if (splitExtra > 0) log(`ℹ Длинные записи автоматически разбиты на 30-мин сегменты (+${splitExtra})`, 'sys');
      log(`ℹ Выбрано: 0 / ${playableCount} playable | осталось выбрать: ${playableCount}`, 'sys');
      log('ℹ Шаги: выбери дату/время → Поиск → отметь записи → Загрузка из сети (или CAPTURE).', 'sys');
    } catch (err) {
      setPhase('ready'); setStatusText('\u041e\u0428\u0418\u0411\u041a\u0410'); log(`\u274c ${err}`, 'err');
    } finally { setScanning(false); }
  };

  const logBulkProgressSnapshot = (downloadsState) => {
    const scopeIndexes = effectiveActionIndexes.length ? effectiveActionIndexes : actionableIndexes;
    const inScope = scopeIndexes.length;
    const done = scopeIndexes.filter((i) => downloadsState[`dl_${i}`] === 'done').length;
    const err = scopeIndexes.filter((i) => downloadsState[`dl_${i}`] === 'error').length;
    const left = Math.max(inScope - done - err, 0);
    log(`ℹ BULK SNAPSHOT: done=${done}/${inScope} err=${err} left=${left}`, 'sys');
  };

  const updateDownloadStatus = (key, status) => {
    setActiveDownloads((prev) => {
      const next = { ...prev, [key]: status };
      if (status === 'done' || status === 'error') {
        logBulkProgressSnapshot(next);
      }
      return next;
    });
  };

  const handleDownload = async (item, idx) => {
    if (!isDownloadableRecord(item)) {
      log('⚠ запись помечена как non-downloadable; используй CAPTURE', 'warn');
      return false;
    }
    const normalizedUri = normalizePlaybackUri(item.playbackUri);
    if (!normalizedUri) {
      updateDownloadStatus(`dl_${idx}`, 'error');
      log('❌ Некорректный playback URI для download', 'err');
      return false;
    }

    const k = `dl_${idx}`; updateDownloadStatus(k, 'working');
    log(`⬇ ЗАГРУЗКА #${idx+1}...`, 'info');
    const taskId = `nem_${Date.now()}_${idx}`;
    try {
      const chunkItems = splitRecordIntoChunks({ ...item, playbackUri: normalizedUri }, 30);
      if (chunkItems.length > 1) {
        log(`ℹ long segment detected: split into ${chunkItems.length} × 30min chunks`, 'sys');
      }
      let lastReport = null;
      for (let i = 0; i < chunkItems.length; i += 1) {
        const chunkTaskId = `${taskId}_p${i + 1}`;
        const chunkNameSuffix = chunkItems.length > 1 ? `_p${String(i + 1).padStart(2, '0')}` : '';
        const filenameHint = `${target.host.replace(/\./g, '_')}_cam${camera}_${idx}${chunkNameSuffix}.mp4`;
        // eslint-disable-next-line no-await-in-loop
        const job = await invoke('start_archive_export_job', {
          playbackUri: chunkItems[i]?.playbackUri || normalizedUri,
          login: target.login || 'admin',
          pass: target.password || '',
          sourceHost: target.host || '',
          filenameHint,
          taskId: chunkTaskId,
        });
        if (!job?.report) {
          const reason = job?.finalReason || (job?.stages || []).filter((s) => !s.success).map((s) => `${s.stage}: ${s.reason || 'failed'}`).join(' || ');
          throw new Error(`chunk ${i + 1}/${chunkItems.length}: ${reason || 'Archive export failed'}`);
        }
        lastReport = job.report;
        log(`✅ chunk ${i + 1}/${chunkItems.length}: ${job.report.filename} (${(job.report.bytesWritten / 1048576).toFixed(1)} MB)`, 'ok');
      }

      const r = lastReport;
      updateDownloadStatus(k, 'done');
      log(`✅ download complete: ${r.filename} (${(r.bytesWritten/1048576).toFixed(1)} MB)`, 'ok');
      return true;
    } catch (e) {
      updateDownloadStatus(k, 'error');
      log(`❌ ${e}`, 'err');
      return false;
    }
  };

  const runBulkQueue = async (queue, modeLabel) => {
    if (!queue.length) return { okCount: 0, errCount: 0 };
    setBulkRunning(true);
    log(`⬇ BULK START: ${queue.length} записей (${modeLabel})`, 'info');
    let okCount = 0;
    let errCount = 0;
    try {
      for (let pos = 0; pos < queue.length; pos += 1) {
        const idx = queue[pos];
        log(`ℹ BULK STEP ${pos + 1}/${queue.length}: запись #${idx + 1}`, 'sys');
        // Последовательная очередь снижает пиковую нагрузку на NVR/канал.
        // eslint-disable-next-line no-await-in-loop
        const record = records[idx];
        const success = isDownloadableRecord(record)
          ? await handleDownload(record, idx)
          : await handleCapture(record, idx);
        if (success) okCount += 1; else errCount += 1;
      }
      log(`✅ BULK FINISH: очередь завершена | ok=${okCount} err=${errCount}`, 'ok');
      return { okCount, errCount };
    } finally {
      setBulkRunning(false);
    }
  };

  const handleBulkDownload = async () => {
    const scopeIndexes = [...effectiveActionIndexes];
    const queue = scopeIndexes.filter((idx) => {
      const st = activeDownloads[`dl_${idx}`];
      return st !== 'working' && st !== 'done';
    });
    const skippedDoneCount = scopeIndexes.length - queue.length;
    if (!queue.length) {
      log('⚠ Нет доступных записей для пакетной загрузки (все уже завершены/в процессе)', 'warn');
      return;
    }
    if (skippedDoneCount > 0) {
      log(`ℹ BULK SKIP: пропущено уже завершённых/активных записей: ${skippedDoneCount}`, 'sys');
    }
    const summary = await runBulkQueue(queue, hasAnySelection ? 'только выбранные' : 'все playable');
    if (summary?.errCount) {
      log(`ℹ Для повтора ошибок используй ↻ Retry ERR (${summary.errCount})`, 'sys');
    }
  };

  const handleRetryErrors = async () => {
    const errorQueue = effectiveActionIndexes.filter((idx) => activeDownloads[`dl_${idx}`] === 'error');
    if (!errorQueue.length) {
      log('⚠ Нет ошибочных записей для повтора', 'warn');
      return;
    }
    log(`ℹ RETRY MODE: перезапуск только ERR записей (${errorQueue.length})`, 'sys');
    const summary = await runBulkQueue(errorQueue, 'retry ERR');
    if (summary?.errCount === 0) {
      log('✅ RETRY MODE: все ошибочные записи успешно перекачаны', 'ok');
    }
  };

  const handleCapture = async (item, idx = null) => {
    const k = idx !== null ? `dl_${idx}` : null;
    if (k) updateDownloadStatus(k, 'working');
    log('🎯 ЗАХВАТ СЕГМЕНТА...', 'info');
    try {
      const normalizedUri = normalizePlaybackUri(item.playbackUri);
      if (!normalizedUri) throw new Error('Некорректный playback URI для capture');
      const captureDurationSeconds = getIsapiCaptureDurationSeconds(item);
      const captureHint = getIsapiFilenameHint(normalizedUri, `capture_${Date.now()}.mp4`);
      log(`ℹ capture policy: duration=${captureDurationSeconds}s file=${captureHint}`, 'sys');
      const r = await invoke('capture_archive_segment', { sourceUrl: normalizedUri, filenameHint: captureHint, durationSeconds: captureDurationSeconds, taskId: `cap_${Date.now()}` });
      if (k) updateDownloadStatus(k, 'done');
      log(`✅ ${r.filename} (${(r.bytesWritten/1048576).toFixed(1)} MB)`, 'ok');
      return true;
    } catch (e) {
      if (k) updateDownloadStatus(k, 'error');
      log(`❌ ${e}`, 'err');
      return false;
    }
  };


  const chOpts = target.channels?.length
    ? target.channels.map(c => ({ v: String(c.index ?? c.id ?? '101'), l: c.name||`\u041a\u0430\u043d\u0430\u043b ${c.index ?? c.id}` }))
    : [{v:'101',l:'[A1] pod 1'},{v:'201',l:'[A2] pod 2'},{v:'301',l:'[A3] pod 3'},{v:'401',l:'[A4] pod 4'}];

  const pc = phase==='ready'?'#00ff9c':phase==='scanning'?'#00f0ff':phase==='error'?'#ff003c':'#ff9900';
  const actionableIndexes = records
    .map((record, index) => (isPlayableRecord(record) ? index : -1))
    .filter((index) => index >= 0);
  const selectedActionableIndexes = actionableIndexes.filter((idx) => Boolean(selectedIndexes[idx]));
  const hasAnySelection = selectedActionableIndexes.length > 0;
  const effectiveActionIndexes = hasAnySelection ? selectedActionableIndexes : actionableIndexes;
  const areAllActionableSelected = actionableIndexes.length > 0 && selectedActionableIndexes.length === actionableIndexes.length;
  const remainingActionableCount = Math.max(actionableIndexes.length - selectedActionableIndexes.length, 0);
  const bulkModeLabel = hasAnySelection ? 'режим: только выбранные' : 'режим: все playable';
  const downloadedInScopeCount = effectiveActionIndexes.filter((idx) => activeDownloads[`dl_${idx}`] === 'done').length;
  const failedInScopeCount = effectiveActionIndexes.filter((idx) => activeDownloads[`dl_${idx}`] === 'error').length;
  const workingInScopeCount = effectiveActionIndexes.filter((idx) => activeDownloads[`dl_${idx}`] === 'working').length;
  const activeWorkingCount = Object.values(activeDownloads).filter((st) => st === 'working').length;
  const hasActiveTransfers = bulkRunning || activeWorkingCount > 0;
  const failedIndexesInScope = effectiveActionIndexes.filter((idx) => activeDownloads[`dl_${idx}`] === 'error');
  const toggleAllActionableSelection = (checked) => {
    setSelectedIndexes(() => {
      const next = {};
      actionableIndexes.forEach((idx) => {
        next[idx] = checked;
      });
      return next;
    });
  };

  return (
    <div style={{ position:'fixed',inset:0,background:'rgba(0,0,0,.88)',backdropFilter:'blur(3px)',zIndex:10000,display:'flex',alignItems:'center',justifyContent:'center',animation:'nemIn .25s ease-out' }}>
      <style>{`
        @keyframes nemIn{from{opacity:0;transform:scale(.97)}to{opacity:1;transform:scale(1)}}
        @keyframes nemP{0%,100%{opacity:.5}50%{opacity:1}}
        @keyframes nemS{0%{top:-2px}100%{top:100%}}
        .ni{background:#000;color:#00f0ff;border:1px solid #1a2a33;padding:6px 8px;font:11px/1.3 Consolas,monospace;outline:none;box-sizing:border-box;width:100%}
        .ni:focus{border-color:#00f0ff;box-shadow:0 0 6px #00f0ff33}
        .ns{background:#000;color:#00f0ff;border:1px solid #1a2a33;padding:6px 8px;font:11px/1.3 Consolas,monospace;outline:none;box-sizing:border-box;width:100%;cursor:pointer}
        .nr{display:flex;align-items:center;gap:10px;padding:7px 14px;border-bottom:1px solid #0e0e12;transition:background .12s}
        .nr:hover{background:#0a1218}
        .nb{background:#0a1a0a;color:#00ff9c;border:1px solid #00ff9c44;padding:2px 8px;font:9px Consolas,monospace;cursor:pointer;transition:all .12s}
        .nb:hover{background:#00ff9c;color:#000}
        .nc{background:#0a0a1a;color:#00f0ff;border:1px solid #00f0ff44;padding:2px 8px;font:9px Consolas,monospace;cursor:pointer;transition:all .12s}
        .nc:hover{background:#00f0ff;color:#000}
      `}</style>

      <div style={{ width:880,maxHeight:'92vh',background:'#08080c',border:`1px solid ${pc}55`,boxShadow:`0 0 40px ${pc}15`,display:'flex',flexDirection:'column',overflow:'hidden',position:'relative' }}>
        {/* scanlines */}
        <div style={{ position:'absolute',inset:0,pointerEvents:'none',zIndex:1,background:'repeating-linear-gradient(0deg,transparent,transparent 3px,rgba(0,240,255,.008) 3px,rgba(0,240,255,.008) 4px)' }}/>
        <div style={{ position:'absolute',left:0,right:0,height:1,background:`linear-gradient(90deg,transparent,${pc}44,transparent)`,animation:'nemS 4s linear infinite',pointerEvents:'none',zIndex:1 }}/>

        {/* HEADER */}
        <div style={{ background:`linear-gradient(90deg,${pc}0a,transparent 30%,transparent 70%,${pc}0a)`,borderBottom:`1px solid ${pc}33`,padding:'10px 16px',display:'flex',justifyContent:'space-between',alignItems:'center',zIndex:2 }}>
          <div style={{ display:'flex',alignItems:'center',gap:10 }}>
            <div style={{ width:7,height:7,borderRadius:'50%',background:pc,boxShadow:`0 0 8px ${pc}`,animation:scanning?'nemP .6s infinite':'none' }}/>
            <div>
              <div style={{ color:pc,fontSize:12,fontWeight:'bold',fontFamily:'Consolas,monospace',letterSpacing:2 }}>{'\u2622'} ЗАГРУЗКА ИЗ СЕТИ</div>
              <div style={{ color:'#444',fontSize:9,fontFamily:'monospace',letterSpacing:1 }}>NEMESIS ARCHIVE TERMINAL // {target.host}:2019</div>
            </div>
          </div>
          <div style={{ display:'flex',alignItems:'center',gap:10 }}>
            <span style={{ color:pc,fontSize:10,fontFamily:'monospace' }}>{statusText}</span>
            <button onClick={onClose} style={{ background:'none',border:'1px solid #ff003c44',color:'#ff003c',padding:'3px 10px',cursor:'pointer',fontSize:11,fontFamily:'monospace',transition:'all .15s' }}
              onMouseEnter={e=>{e.target.style.background='#ff003c';e.target.style.color='#000'}}
              onMouseLeave={e=>{e.target.style.background='none';e.target.style.color='#ff003c'}}>{'\u2716'}</button>
          </div>
        </div>

        {/* TABS */}
        <div style={{ borderBottom:'1px solid #151518',display:'flex',zIndex:2 }}>
          <div style={{ padding:'7px 18px',fontSize:11,fontFamily:'monospace',color:'#ff003c',borderBottom:'2px solid #ff003c',cursor:'pointer',letterSpacing:1 }}>СКАЧАТЬ ПО ФАЙЛАМ</div>
          <div style={{ padding:'7px 18px',fontSize:11,fontFamily:'monospace',color:'#333',cursor:'pointer',letterSpacing:1 }}>СКАЧИВАТЬ ПО ДАТЕ</div>
        </div>

        {/* BODY */}
        <div style={{ display:'flex',flex:1,overflow:'hidden',zIndex:2 }}>

          {/* LEFT PANEL */}
          <div style={{ width:195,borderRight:'1px solid #151518',padding:'14px 12px',background:'#06060a',display:'flex',flexDirection:'column',gap:14,flexShrink:0 }}>
            <div style={{ color:'#555',fontSize:9,fontFamily:'monospace',letterSpacing:1,borderBottom:'1px solid #151518',paddingBottom:6 }}>УСЛОВИЕ ПОИСКА</div>

            <div>
              <label style={{ color:'#4a5a66',fontSize:9,fontFamily:'monospace',display:'block',marginBottom:3 }}>Камера</label>
              <select className="ns" value={camera} onChange={e=>setCamera(e.target.value)}>
                {chOpts.map(o=><option key={o.v} value={o.v}>{o.l}</option>)}
              </select>
            </div>

            <div>
              <label style={{ color:'#4a5a66',fontSize:9,fontFamily:'monospace',display:'block',marginBottom:3 }}>Тип файла</label>
              <select className="ns" value={fileType} onChange={e=>setFileType(e.target.value)}>
                <option value="all">Все</option>
                <option value="timing">По расписанию</option>
                <option value="alarm">Тревожные</option>
                <option value="manual">Вручную</option>
              </select>
            </div>

            <div>
              <label style={{ color:'#4a5a66',fontSize:9,fontFamily:'monospace',display:'block',marginBottom:3 }}>Тип потока:</label>
              <select className="ns" value={streamType} onChange={e=>setStreamType(e.target.value)}>
                <option value="main">Основной поток</option>
                <option value="sub">Дополнительный</option>
              </select>
            </div>

            <div>
              <label style={{ color:'#4a5a66',fontSize:9,fontFamily:'monospace',display:'block',marginBottom:3 }}>Время начала</label>
              <input className="ni" value={timeFrom} onChange={e=>setTimeFrom(e.target.value)}/>
            </div>

            <div>
              <label style={{ color:'#4a5a66',fontSize:9,fontFamily:'monospace',display:'block',marginBottom:3 }}>Время окончания</label>
              <input className="ni" value={timeTo} onChange={e=>setTimeTo(e.target.value)}/>
            </div>

            <button onClick={handleSearch} disabled={scanning||phase==='connecting'||hasActiveTransfers} style={{
              width:'100%',padding:'10px 0',marginTop:'auto',
              background:scanning?'#1a0a0a':'linear-gradient(180deg,#3a0a0a,#1a0505)',
              color:'#ff003c',border:'1px solid #ff003c',fontSize:12,fontFamily:'Consolas,monospace',
              fontWeight:'bold',cursor:(scanning||hasActiveTransfers)?'not-allowed':'pointer',letterSpacing:1,
              transition:'all .2s',opacity:(scanning||hasActiveTransfers)?.5:1
            }}
              onMouseEnter={e=>{if(!scanning && !hasActiveTransfers){e.target.style.background='#ff003c';e.target.style.color='#000'}}}
              onMouseLeave={e=>{e.target.style.background='linear-gradient(180deg,#3a0a0a,#1a0505)';e.target.style.color='#ff003c'}}
            >{scanning?'\u25c9 \u041f\u041e\u0418\u0421\u041a...':(hasActiveTransfers?'⏳ DOWNLOAD ACTIVE':'\ud83d\udd0e \u041f\u043e\u0438\u0441\u043a')}</button>
          </div>

          {/* RIGHT TABLE */}
          <div style={{ flex:1,display:'flex',flexDirection:'column',overflow:'hidden' }}>
            {/* toolbar */}
            <div style={{ display:'flex',alignItems:'center',justifyContent:'space-between',padding:'6px 14px',borderBottom:'1px solid #151518',background:'#0a0a0e' }}>
              <span style={{ color:'#555',fontSize:10,fontFamily:'monospace',letterSpacing:1 }}>СПИСОК ФАЙЛОВ</span>
              <div style={{ display:'flex',gap:6,alignItems:'center' }}>
                <button onClick={handleBulkDownload} disabled={!effectiveActionIndexes.length || hasActiveTransfers}
                  style={{ background:'#0a1a0a',color:'#00ff9c',border:'1px solid #00ff9c55',padding:'4px 12px',fontSize:10,fontFamily:'monospace',cursor:(!effectiveActionIndexes.length || hasActiveTransfers)?'not-allowed':'pointer',transition:'all .15s',opacity:(!effectiveActionIndexes.length || hasActiveTransfers)?.6:1 }}
                  onMouseEnter={e=>{if (!hasActiveTransfers && effectiveActionIndexes.length) { e.target.style.background='#00ff9c';e.target.style.color='#000'; }}}
                  onMouseLeave={e=>{e.target.style.background='#0a1a0a';e.target.style.color='#00ff9c'}}
                  title={hasActiveTransfers ? 'Недоступно: есть активные передачи' : (hasAnySelection ? 'Скачать только выбранные playable записи (RTSP будет через CAPTURE)' : 'Скачать все playable записи (RTSP будет через CAPTURE)')}
                >{hasActiveTransfers ? '⏳ DOWNLOAD ACTIVE...' : `⬇ Загрузка из сети${hasAnySelection ? ` (${selectedActionableIndexes.length} из выбранных)` : ''}`}</button>
                <button onClick={handleRetryErrors} disabled={hasActiveTransfers||!failedIndexesInScope.length}
                  style={{ background:'#2a1200',color:'#ffb266',border:'1px solid #ff990055',padding:'4px 10px',fontSize:10,fontFamily:'monospace',cursor:(hasActiveTransfers||!failedIndexesInScope.length)?'not-allowed':'pointer',transition:'all .15s',opacity:(hasActiveTransfers||!failedIndexesInScope.length)?.6:1 }}
                  title={hasActiveTransfers ? 'Недоступно: есть активные передачи' : 'Повторить только записи со статусом ERR'}
                >↻ Retry ERR ({failedIndexesInScope.length})</button>
                <button onClick={()=>setSelectedIndexes({})} disabled={hasActiveTransfers||!hasAnySelection}
                  style={{ background:'#141418',color:hasAnySelection?'#9faec0':'#5a6370',border:'1px solid #2a3240',padding:'4px 10px',fontSize:10,fontFamily:'monospace',cursor:(hasActiveTransfers||!hasAnySelection)?'not-allowed':'pointer',opacity:(hasActiveTransfers||!hasAnySelection)?.6:1 }}
                 title={hasActiveTransfers ? 'Недоступно: есть активные передачи' : 'Снять текущий выбор'}>Снять выбор</button>
              </div>
            </div>
            <div style={{ padding:'6px 14px',borderBottom:'1px solid #121216',background:'#09090d',color:'#4e6475',fontSize:9,fontFamily:'monospace',lineHeight:1.5 }}>
              HYPERION ARCHIVE FLOW: 1) выбери дату/время 2) Поиск 3) отметь записи 4) загрузи выбранные (обычно до ~1050MB) или используй CAPTURE.
            </div>
            <div style={{ padding:'4px 14px',borderBottom:'1px solid #111',background:'#08080c',color:'#3f5566',fontSize:9,fontFamily:'monospace' }}>Чекбокс в заголовке отмечает/снимает все playable записи текущего списка.</div>
            <div style={{ padding:'3px 14px',borderBottom:'1px solid #111',background:'#07070b',color:'#4d6779',fontSize:9,fontFamily:'monospace' }}>SELECTED PLAYABLE: {selectedActionableIndexes.length}/{actionableIndexes.length} | ОСТАЛОСЬ: {remainingActionableCount} | СКАЧАНО: {downloadedInScopeCount}/{effectiveActionIndexes.length || actionableIndexes.length} | WORKING(scope/all): {workingInScopeCount}/{activeWorkingCount} | ERR: {failedInScopeCount}</div>
            <div style={{ padding:'2px 14px',borderBottom:'1px solid #111',background:'#06060a',color:'#3e5567',fontSize:9,fontFamily:'monospace' }}>{bulkModeLabel}</div>

            {/* col headers */}
            <div style={{ display:'flex',alignItems:'center',padding:'5px 14px',borderBottom:'1px solid #1a1a1e',background:'#0c0c10',fontSize:9,fontFamily:'monospace',color:'#555',letterSpacing:1,flexShrink:0 }}>
              <div style={{ width:26 }}><input type="checkbox" disabled={hasActiveTransfers} checked={areAllActionableSelected} onChange={(e)=>toggleAllActionableSelection(e.target.checked)} style={{ accentColor:'#00f0ff' }} title="Выбрать все playable" /></div>
              <div style={{ width:32 }}>№</div>
              <div style={{ flex:2 }}>Имя файла</div>
              <div style={{ flex:1 }}>Время начала</div>
              <div style={{ flex:1 }}>Время окончания</div>
              <div style={{ width:65,textAlign:'right' }}>Размер</div>
              <div style={{ width:100,textAlign:'center' }}>Прогресс</div>
            </div>

            {/* rows */}
            <div style={{ flex:1,overflowY:'auto' }}>
              {records.length===0&&!scanning&&(
                <div style={{ padding:40,textAlign:'center' }}>
                  <div style={{ color:'#1a1a22',fontSize:32,marginBottom:8 }}>{'\u2622'}</div>
                  <div style={{ color:'#333',fontSize:11,fontFamily:'monospace' }}>{phase==='connecting'?'УСТАНОВКА СВЯЗИ...':'Нажмите «Поиск» чтобы найти записи'}</div>
                </div>
              )}
              {scanning&&(
                <div style={{ padding:40,textAlign:'center' }}>
                  <div style={{ color:'#00f0ff',fontSize:16,animation:'nemP .8s infinite' }}>{'\u25c9'}</div>
                  <div style={{ color:'#00f0ff',fontSize:11,fontFamily:'monospace',marginTop:6 }}>ИДЁТ ПОИСК...</div>
                </div>
              )}
              {records.map((item,idx)=>{
                const k=`dl_${idx}`;const ds=activeDownloads[k];
                const fid=item.playbackUri?.match(/name=([^&]+)/)?.[1]||item.playbackUri?.match(/(\d{10,})/)?.[1]||`rec_${idx}`;
                const chunkLabel = item.chunkTotal > 1 ? ` [${item.chunkIndex}/${item.chunkTotal}]` : '';
                return (
                  <div key={idx} className="nr">
                    <div style={{ width:26 }}><input type="checkbox" disabled={hasActiveTransfers||!isPlayableRecord(item)} title={hasActiveTransfers ? 'Недоступно: выполняется активная загрузка' : (isPlayableRecord(item) ? 'Выбрать для загрузки/capture' : 'Недоступно: нет playback URI')} checked={Boolean(selectedIndexes[idx])} onChange={(e)=>setSelectedIndexes((prev)=>({ ...prev, [idx]: e.target.checked }))} style={{ accentColor:'#00f0ff', opacity:(!hasActiveTransfers&&isPlayableRecord(item))?1:.45, cursor:(!hasActiveTransfers&&isPlayableRecord(item))?'pointer':'not-allowed' }}/></div>
                    <div style={{ width:32,color:'#444',fontSize:10,fontFamily:'monospace' }}>{idx+1}</div>
                    <div style={{ flex:2,color:'#9fd7ff',fontSize:10,fontFamily:'monospace',overflow:'hidden',textOverflow:'ellipsis',whiteSpace:'nowrap' }} title={`transport=${item.transport||'-'} conf=${item.confidence??0} playable=${String(isPlayableRecord(item))} downloadable=${String(isDownloadableRecord(item))}`}>{`${fid}${chunkLabel}`}</div>
                    <div style={{ flex:1,color:'#7fa9cb',fontSize:10,fontFamily:'monospace' }}>{item.startTime?.replace('T',' ').replace('Z','')||'\u2014'}</div>
                    <div style={{ flex:1,color:'#7fa9cb',fontSize:10,fontFamily:'monospace' }}>{item.endTime?.replace('T',' ').replace('Z','')||'\u2014'}</div>
                    <div style={{ width:65,textAlign:'right',color:'#ff9900',fontSize:10,fontFamily:'monospace' }}>\u2014</div>
                    <div style={{ width:100,display:'flex',gap:4,justifyContent:'center' }}>
                      {ds==='done'?<span style={{color:'#00ff9c',fontSize:9,fontFamily:'monospace'}}>{'\u2713'} OK</span>
                       :ds==='error'?<span style={{color:'#ff003c',fontSize:9,fontFamily:'monospace'}}>{'\u2716'} ERR</span>
                       :ds==='working'?<span style={{color:'#00f0ff',fontSize:9,fontFamily:'monospace',animation:'nemP .6s infinite'}}>{'\u25cc'} ...</span>
                       :item.playbackUri?<>{isDownloadableRecord(item)?<button className="nb" disabled={hasActiveTransfers} onClick={()=>handleDownload(item,idx)}>{'\u2b07'}</button>:<span style={{color:'#6a4a3f',fontSize:8,fontFamily:'monospace'}}>NO-DL</span>}<button className="nc" disabled={hasActiveTransfers} onClick={()=>handleCapture(item,idx)}>{'\u25c9'}</button></>
                       :<span style={{color:'#222',fontSize:9,fontFamily:'monospace'}}>\u2014</span>}
                    </div>
                  </div>
                );
              })}
            </div>

            {/* pagination */}
            <div style={{ borderTop:'1px solid #151518',padding:'5px 14px',background:'#0a0a0e',display:'flex',justifyContent:'flex-end',alignItems:'center',gap:6,fontSize:10,fontFamily:'monospace',color:'#555',flexShrink:0 }}>
              <span>Всего {records.length} Страница</span>
              <span style={{ color:'#00f0ff' }}>1/1</span>
            </div>
          </div>
        </div>

        {/* TERMINAL LOG */}
        <div style={{ borderTop:'1px solid #151518',background:'#040406',height:75,zIndex:2,display:'flex',flexDirection:'column' }}>
          <div style={{ padding:'3px 14px',borderBottom:'1px solid #0e0e12' }}>
            <span style={{ color:'#222',fontSize:8,fontFamily:'monospace',letterSpacing:1 }}>{'\u25b8'} ТЕРМИНАЛ</span>
          </div>
          <div ref={logRef} style={{ flex:1,overflowY:'auto',padding:'2px 14px' }}>
            {logs.map((l,i)=>(
              <div key={i} style={{ color:l.type==='err'?'#ff003c':l.type==='ok'?'#00ff9c':l.type==='warn'?'#ff9900':l.type==='sys'?'#b366ff':'#3a5a6a',fontSize:9,fontFamily:'Consolas,Courier New,monospace',lineHeight:1.6 }}>
                <span style={{ color:'#222' }}>[{l.ts}]</span> {l.msg}
              </div>
            ))}
          </div>
        </div>

        {/* FOOTER */}
        <div style={{ borderTop:'1px solid #0e0e12',padding:'3px 14px',background:'#06060a',display:'flex',justifyContent:'space-between',zIndex:2 }}>
          <span style={{ color:'#1a1a22',fontSize:8,fontFamily:'monospace' }}>HYPERION NEMESIS ENGINE v2.0</span>
          <span style={{ color:'#1a1a22',fontSize:8,fontFamily:'monospace' }}>{target.host}:2019</span>
        </div>
      </div>
    </div>
  );
}
