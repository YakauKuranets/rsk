import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { scanHostPorts } from './api/tauri';
import { listen } from '@tauri-apps/api/event';
import 'leaflet/dist/leaflet.css';
import './App.css';
import NemesisArchiveTerminal from './NemesisArchiveTerminal';
import ArchiveViewer from './features/archive/ArchiveViewer';
import StreamGrid from './features/streams/StreamGrid';
import MultiStreamGrid from './features/streams/MultiStreamGrid';
import StreamPlayer from './StreamPlayer';
import { useNvrPanel } from './hooks/useNvrPanel';
import { useCapturePanel } from './hooks/useCapturePanel';
import { useHubRecon } from './hooks/useHubRecon';
import { useAppStore } from './store/appStore';
import ToastHost from './components/ToastHost';
import { toast } from './utils/toast';
import Sidebar from './features/ui/Sidebar';

function normalizeTargetRecords(rawTargets) {
  const normalized = [];

  for (const raw of rawTargets) {
    if (!raw || typeof raw !== 'object') continue;

    if (Array.isArray(raw.terminals) && raw.terminals.length > 0) {
      for (const terminal of raw.terminals) {
        if (!terminal || typeof terminal !== 'object') continue;
        normalized.push({
          ...terminal,
          id: terminal.id || `${raw.id || 'site'}_${terminal.host || Date.now()}`,
          name: terminal.name || raw.name || 'Терминал',
          lat: terminal.lat ?? raw.lat,
          lng: terminal.lng ?? raw.lng,
          siteId: raw.id || terminal.siteId,
          siteName: raw.name || terminal.siteName,
        });
      }
      continue;
    }

    normalized.push(raw);
  }

  return normalized;
}

export default function App() {
  const [targets, setTargets] = useState([]);
  const [activeStream, setActiveStream] = useState(null);
  const [streamType, setStreamType] = useState('ws-flv');
  const [activeTargetId, setActiveTargetId] = useState(null);
  const [activeCameraName, setActiveCameraName] = useState('');

  const [loading, setLoading] = useState(false);
  const [radarStatus, setRadarStatus] = useState('');

  // --- СТРИМ-РЕКАВЕРИ ---
  const [streamRtspUrl, setStreamRtspUrl] = useState('');
  const [streamTerminal, setStreamTerminal] = useState(null);
  const [streamChannel, setStreamChannel] = useState(null);

  const [streamViewMode, setStreamViewMode] = useState('none'); // 'none' | 'single' | 'multi'
  const [pendingCameraStream, setPendingCameraStream] = useState(null);
  const [singleStreamCamera, setSingleStreamCamera] = useState(null);
  const [singleStreamSession, setSingleStreamSession] = useState(null);
  const [selectedTerminal, setSelectedTerminal] = useState(null);
  const [mapCenter, setMapCenter] = useState([53.9, 27.56]);
  const [form, setForm] = useState({ name: '', host: '', login: 'admin', password: '', lat: 53.9, lng: 27.56, channelCount: 4 });

  // --- GLOBAL STORE (Zustand) ---
  const ftpBrowserOpen = useAppStore((s) => s.ftpBrowserOpen);
  const setFtpBrowserOpen = useAppStore((s) => s.setFtpBrowserOpen);
  const activeServerAlias = useAppStore((s) => s.activeServerAlias);
  const setActiveServerAlias = useAppStore((s) => s.setActiveServerAlias);
  const ftpPath = useAppStore((s) => s.ftpPath);
  const setFtpPath = useAppStore((s) => s.setFtpPath);
  const ftpItems = useAppStore((s) => s.ftpItems);
  const setFtpItems = useAppStore((s) => s.setFtpItems);
  const fuzzLogin = useAppStore((s) => s.fuzzLogin);
  const setFuzzLogin = useAppStore((s) => s.setFuzzLogin);
  const fuzzPassword = useAppStore((s) => s.fuzzPassword);
  const setFuzzPassword = useAppStore((s) => s.setFuzzPassword);
  const fuzzPath = useAppStore((s) => s.fuzzPath);
  const setFuzzPath = useAppStore((s) => s.setFuzzPath);
  const targetInput = useAppStore((s) => s.targetInput);
  const setTargetInput = useAppStore((s) => s.setTargetInput);
  const attackType = useAppStore((s) => s.attackType);
  const setAttackType = useAppStore((s) => s.setAttackType);
  const fuzzResults = useAppStore((s) => s.fuzzResults);
  const setFuzzResults = useAppStore((s) => s.setFuzzResults);
  const spiderMaxDepth = useAppStore((s) => s.spiderMaxDepth);
  const spiderMaxPages = useAppStore((s) => s.spiderMaxPages);
  const spiderDirBrute = useAppStore((s) => s.spiderDirBrute);
  const spiderEnableVulnVerification = useAppStore((s) => s.spiderEnableVulnVerification);
  const spiderEnableOsintImport = useAppStore((s) => s.spiderEnableOsintImport);
  const spiderEnableTopologyDiscovery = useAppStore((s) => s.spiderEnableTopologyDiscovery);
  const spiderEnableSnapshotRefresh = useAppStore((s) => s.spiderEnableSnapshotRefresh);
  const spiderEnableVideoStreamAnalyzer = useAppStore((s) => s.spiderEnableVideoStreamAnalyzer);
  const spiderEnableCredentialDepthAudit = useAppStore((s) => s.spiderEnableCredentialDepthAudit);
  const spiderEnablePassiveArpDiscovery = useAppStore((s) => s.spiderEnablePassiveArpDiscovery);
  const spiderEnableUptimeMonitoring = useAppStore((s) => s.spiderEnableUptimeMonitoring);
  const spiderEnableNeighborDiscovery = useAppStore((s) => s.spiderEnableNeighborDiscovery);
  const spiderEnableThreatIntel = useAppStore((s) => s.spiderEnableThreatIntel);
  const spiderEnableScheduledAudits = useAppStore((s) => s.spiderEnableScheduledAudits);
  const setSourceAnalysis = useAppStore((s) => s.setSourceAnalysis);
  const setHubCookie = useAppStore((s) => s.setHubCookie);

  const [runtimeLogs, setRuntimeLogs] = useState([]);
  const [targetSearch, setTargetSearch] = useState('');
  const [targetTypeFilter, setTargetTypeFilter] = useState('all');
  const [archiveOnly, setArchiveOnly] = useState(false);
  const [labels, setLabels] = useState(() => {
    try { return JSON.parse(localStorage.getItem('hyperion_labels') || '[]'); }
    catch { return []; }
  });
  const [nemesisTarget, setNemesisTarget] = useState(null);
  const [downloadTasks, setDownloadTasks] = useState(() => {
    try {
      const raw = localStorage.getItem('hyperion_download_tasks');
      return raw ? JSON.parse(raw) : [];
    } catch {
      return [];
    }
  });
  const [resumeDownloads, setResumeDownloads] = useState(() => {
    try {
      const raw = localStorage.getItem('hyperion_resume_downloads');
      return raw == null ? true : raw === '1';
    } catch {
      return true;
    }
  });
  const nvr = useNvrPanel();
  const [implementationStatus, setImplementationStatus] = useState(null);
  const [auditResults, setAuditResults] = useState([]);
  const [interceptLogs, setInterceptLogs] = useState([]);
  const [isSniffing, setIsSniffing] = useState(false);
  const [agentScope, setAgentScope] = useState('demo.local');
  const [agentPacket, setAgentPacket] = useState(null);
  const [agentStatus, setAgentStatus] = useState('');
  const capture = useCapturePanel();
  const hubRecon = useHubRecon();

  const hubConfig = { cookie: '' };

  const pollIntervalRef = useRef(null);
  const healthCheckRef = useRef(null);
  const activeTargetIdRef = useRef(null);

  useEffect(() => {
    // Migrate legacy SHA256-encrypted targets to current Argon2id scheme.
    // Safe to call on every start — skips already-migrated entries.
    invoke('migrate_legacy_vault').then((msg) => {
      console.log("[vault migration]", msg);
    }).catch((e) => {
      console.warn("[vault migration failed]", e);
    }).finally(() => {
      loadTargets();
    });
  }, []);

  useEffect(() => {
    setHubCookie(hubConfig.cookie || '');
  }, [setHubCookie]);

  // Сохранять метки при каждом изменении
  useEffect(() => {
    localStorage.setItem('hyperion_labels', JSON.stringify(labels));
  }, [labels]);

  useEffect(() => {
    invoke('get_implementation_status')
      .then((status) => setImplementationStatus(status))
      .catch(() => {});
  }, []);

  useEffect(() => {
    try {
      localStorage.setItem('hyperion_resume_downloads', resumeDownloads ? '1' : '0');
    } catch {}
  }, [resumeDownloads]);

  useEffect(() => {
    try {
      localStorage.setItem('hyperion_download_tasks', JSON.stringify(downloadTasks.slice(0, 50)));
    } catch {}
  }, [downloadTasks]);

  async function handleRunReconAgent() {
    try {
      setAgentStatus('Recon agent running...');
      const packet = await invoke('run_recon_agent', {
        scope: agentScope,
        shodanKey: '',
        pipelineId: `pipeline-${Date.now()}`,
      });
      setAgentPacket(packet);
      setAgentStatus(`Recon complete: ${packet.findings?.length || 0} findings`);
    } catch (error) {
      setAgentStatus(`Recon error: ${error}`);
    }
  }

  function handleAgentHandoff(packet) {
    setAgentPacket(packet);
    setAgentStatus(`Handoff confirmed for ${packet.pipelineId}`);
    toast('Agent handoff подтверждён');
  }

  useEffect(() => {
    let disposed = false;
    let unlistenFn = null;

    listen('hyperion-audit-event', (event) => {
      if (disposed) return;
      const line = String(event.payload || '');
      console.log('Получено событие от Паука:', line);
      setRuntimeLogs((prev) => [...prev, line].slice(-300));
    }).then((fn) => {
      unlistenFn = fn;
    }).catch(() => {});

    return () => {
      disposed = true;
      if (unlistenFn) unlistenFn();
    };
  }, []);


  useEffect(() => {
    let disposed = false;
    let unlistenFn = null;

    listen('intercepted_credential', (event) => {
      if (disposed) return;
      const payload = event?.payload || {};
      toast(`ПЕРЕХВАТ: ${payload.protocol || 'UNKNOWN'}`, 'error');
      setInterceptLogs((prev) => [...prev, payload].slice(-100));
    }).then((fn) => {
      unlistenFn = fn;
    }).catch(() => {});

    return () => {
      disposed = true;
      if (unlistenFn) unlistenFn();
    };
  }, []);

  const handleStartSniffer = async () => {
    try {
      setIsSniffing(true);
      const msg = await invoke('start_passive_sniffer');
      toast(msg, 'success');
    } catch (err) {
      toast('Ошибка сниффера (нужны админ права?): ' + err, 'error');
      setIsSniffing(false);
    }
  };

  useEffect(() => {
    const progressLines = runtimeLogs.filter((line) => String(line).includes('DOWNLOAD_PROGRESS|'));
    const cancelledLines = runtimeLogs.filter((line) => String(line).includes('DOWNLOAD_CANCELLED|'));

    if (progressLines.length === 0 && cancelledLines.length === 0) return;

    setDownloadTasks((prev) => {
      let next = [...prev];
      for (const line of progressLines) {
        const raw = String(line).split('DOWNLOAD_PROGRESS|')[1] || '';
        const [taskId, currentRaw, totalRaw] = raw.split('|');
        const current = Number(currentRaw || 0);
        const total = Number(totalRaw || 0);
        if (!taskId) continue;
        next = next.map((t) => {
          if (t.id !== taskId) return t;
          if (t.status === 'done' || t.status === 'cancelled') return t;
          if (total > 0) {
            return {
              ...t,
              status: 'running',
              percent: Math.max(1, Math.min(99, Math.round((current / total) * 100))),
              bytesWritten: current,
            };
          }
          return {
            ...t,
            status: 'running',
            bytesWritten: Math.max(t.bytesWritten || 0, current),
          };
        });
      }

      for (const line of cancelledLines) {
        const raw = String(line).split('DOWNLOAD_CANCELLED|')[1] || '';
        const [taskId] = raw.split('|');
        if (!taskId) continue;
        next = next.map((t) =>
          t.id === taskId ? { ...t, status: 'cancelled', error: 'Отменено пользователем' } : t,
        );
      }

      return next;
    });
  }, [runtimeLogs]);


  useEffect(() => {
    activeTargetIdRef.current = activeTargetId;
  }, [activeTargetId]);

  useEffect(() => {
    return () => {
      const targetId = activeTargetIdRef.current;
      if (targetId) {
        invoke('stop_stream', { targetId }).catch(() => {});
      }
    };
  }, []);

  const loadTargets = async () => {
    try {
      const keys = await invoke('get_all_targets');
      const loaded = [];
      for (const key of keys) {
        try {
          const jsonStr = await invoke('read_target', { targetId: key });
          const obj = typeof jsonStr === 'string' ? JSON.parse(jsonStr) : jsonStr;
          if (obj && typeof obj === 'object') loaded.push(obj);
        } catch (e) {
          // One bad record does NOT kill the rest
          console.warn('[vault] skipped unreadable target', key, e.message || e);
        }
      }
      setTargets(normalizeTargetRecords(loaded));
    } catch (err) {
      console.error('[vault] loadTargets failed', err);
    }
  };

  const handleSmartSave = async () => {
    if (!form.host) return toast("Требуется IP");
    const autoId = `nvr_${Date.now()}`;
    const channels = Array.from({length: form.channelCount}, (_, i) => ({ id: `ch${i+1}`, index: i+1, name: `Камера ${i+1}` }));
    const payload = JSON.stringify({ ...form, id: autoId, channels });
    await invoke('save_target', { targetId: autoId, payload });
    loadTargets();
  };

  const handleDeleteTarget = async (id) => {
    if (window.confirm(`Ликвидировать досье?`)) {
      await invoke('delete_target', { targetId: id });
      loadTargets();
    }
  };

  const handleGeocode = async () => {
    try {
      const [lat, lng] = await invoke('geocode_address', { address: hubRecon.addressQuery });
      setForm({ ...form, lat, lng }); setMapCenter([lat, lng]);
    } catch (err) { toast("Не найдено"); }
  };

  const handleStartStream = async (terminal, channel) => {
    try {
      if (activeTargetId) {
        await invoke('stop_stream', { targetId: activeTargetId });
      }
      if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
      if (healthCheckRef.current) clearInterval(healthCheckRef.current);

      setLoading(true);
      let streamSessionId = '';
      let rtspUrlForRecovery = '';

      if (terminal.type === 'hub') {
        setRadarStatus('ЗАПУСК FFMPEG-ТУННЕЛЯ ДЛЯ ХАБА...');
        streamSessionId = `hub_${terminal.hub_id}_${channel.index}`;
        const wsUrl = await invoke('start_hub_stream', {
            targetId: streamSessionId,
            userId: terminal.hub_id.toString(),
            channelId: channel.index.toString(),
            cookie: hubConfig.cookie
        });
        setActiveStream(wsUrl);
        rtspUrlForRecovery = 'hub'; // Маркер что это hub-стрим
      } else {
        setRadarStatus('РАЗВЕДКА МАРШРУТА...');
        let cleanHost = terminal.host.replace(/^(http:\/\/|https:\/\/|rtsp:\/\/)/i, '').split('/')[0];
        const activePath = await invoke('probe_rtsp_path', { host: cleanHost, login: terminal.login, pass: terminal.password });
        let rtspUrl = activePath;

        if (!activePath.toLowerCase().startsWith('rtsp://')) {
          const safePath = activePath.replace(/channel=1|ch1|Channels\/1/g, (match) => match.replace('1', channel.index));
          rtspUrl = `rtsp://${terminal.login}:${terminal.password}@${cleanHost}/${safePath.replace(/^\//, '')}`;
        } else {
          const encodedLogin = encodeURIComponent(terminal.login || 'admin');
          const encodedPass = encodeURIComponent(terminal.password || '');
          rtspUrl = activePath
            .replace(/channel=1\b/g, `channel=${channel.index}`)
            .replace(/ch1\b/g, `ch${channel.index}`)
            .replace(/Channels\/101\b/g, `Channels/${channel.index}01`)
            .replace(/\/11(\b|$)/g, `/${channel.index}1$1`)
            .replace('{login}', encodedLogin)
            .replace('{password}', encodedPass);
        }

        streamSessionId = `${terminal.id}_${channel.id}`;
        setRadarStatus('ЗАПУСК ЯДРА FFMPEG...');
        const wsUrl = await invoke('start_stream', { targetId: streamSessionId, rtspUrl });
        setActiveStream(wsUrl);
        rtspUrlForRecovery = rtspUrl;
      }

      // Сохраняем данные для рекавери/рефреша
      setStreamRtspUrl(rtspUrlForRecovery);
      setStreamTerminal(terminal);
      setStreamChannel(channel);

      setStreamType('ws-flv');
      setActiveTargetId(streamSessionId);
      setActiveCameraName(`${terminal.name} :: ${channel.name}`);
      setLoading(false);

      healthCheckRef.current = setInterval(async () => {
        try {
          const alive = await invoke('check_stream_alive', { targetId: streamSessionId });
          if (!alive) {
            clearInterval(healthCheckRef.current);
            console.warn('[STREAM] FFmpeg process died for', streamSessionId);
          }
        } catch (e) {}
      }, 5000);
    } catch (err) {
      toast("СБОЙ: " + err);
      setLoading(false);
    }
  };

  const handleStopStream = async () => {
    if (activeTargetId) {
      await invoke('stop_stream', { targetId: activeTargetId });
    }
    setActiveTargetId(null);
    setActiveStream(null);
    setActiveCameraName('');
    setStreamRtspUrl('');
    setStreamTerminal(null);
    setStreamChannel(null);
    if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
    if (healthCheckRef.current) clearInterval(healthCheckRef.current);
  };

  // --- ОБНОВИТЬ СТРИМ (кнопка в плеере) ---
  const handleRefreshStream = async () => {
    if (!activeTargetId || !streamRtspUrl) return;
    setLoading(true);
    setRadarStatus('ПЕРЕЗАПУСК ПОТОКА...');
    try {
      await invoke('stop_stream', { targetId: activeTargetId });
      await new Promise(r => setTimeout(r, 500));
      const wsUrl = await invoke('start_stream', { targetId: activeTargetId, rtspUrl: streamRtspUrl });

      // ИСПРАВЛЕНИЕ: Также убираем setActiveStream(null) и таймауты
      setActiveStream(wsUrl);

    } catch (err) {
      toast('Ошибка перезапуска: ' + err);
    }
    setLoading(false);
  };

  const handlePlayArchive = async (playbackUri) => {
    if (!activeTargetId) return;
    setLoading(true);
    setRadarStatus('ПОДКЛЮЧЕНИЕ К АРХИВУ...');
    try {
      await invoke('stop_stream', { targetId: activeTargetId });
      // Небольшая пауза, чтобы FFmpeg точно успел освободить ресурсы
      await new Promise(r => setTimeout(r, 500));

      let finalUri = playbackUri;

      // Внедряем креды в RTSP ссылку только если их там ещё нет
      const hasCreds = /^rtsp:\/\/[^/]*@/i.test(finalUri);
      if (streamTerminal && streamTerminal.login && finalUri.startsWith('rtsp://') && !hasCreds) {
        const user = encodeURIComponent(streamTerminal.login);
        const pass = encodeURIComponent(streamTerminal.password || '');
        finalUri = finalUri.replace('rtsp://', `rtsp://${user}:${pass}@`);
      }

      const wsUrl = await invoke('start_stream', { targetId: activeTargetId, rtspUrl: finalUri });
      setStreamRtspUrl(finalUri);

      // ИСПРАВЛЕНИЕ: Передаем новый URL без предварительного null.
      // Плеер не будет уничтожен, и вкладка "Архив" с таймлайном останутся открытыми!
      setActiveStream(wsUrl);

    } catch (err) {
      toast('Ошибка запуска архива: ' + err);
    }
    setLoading(false);
  };

  const handleHubSearch = async () => {
    try {
        setLoading(true);
        setRadarStatus('СКАНИРОВАНИЕ БАЗЫ ХАБА...');
        const res = await invoke('search_global_hub', { query: hubRecon.hubSearch, cookie: hubConfig.cookie });
        if (res.length === 0) toast("Поиск не дал результатов. Проверьте адрес или обновите PHPSESSID в коде!");
        hubRecon.setHubResults(res);
        setLoading(false);
    } catch (err) {
        toast("ОШИБКА РАЗВЕДКИ: " + err);
        setLoading(false);
    }
  };

  const handleHubStream = (userId, channelId, address) => {
      const fakeTerminal = { type: 'hub', hub_id: userId, name: `GLOBAL HUB :: ${address}` };
      const fakeChannel = { index: channelId, name: `Камера ${parseInt(channelId) + 1}` };
      handleStartStream(fakeTerminal, fakeChannel);
  };

  // --- FTP ПРОВОДНИК (с поддержкой relay) ---
  const fetchFtpRoot = async (serverAlias, path = "/") => {
    setFtpBrowserOpen(true);
    setActiveServerAlias(serverAlias);
    setFtpPath(path);

    setLoading(true);
    const relayUrl = localStorage.getItem('hyperion_relay_url') || '';
    const relayToken = localStorage.getItem('hyperion_relay_token') || '';

    // Если relay настроен — идём через него
    if (relayUrl.trim()) {
      setRadarStatus(`RELAY → ${serverAlias.toUpperCase()} : ${path}`);
      try {
        const folders = await invoke('relay_list_files', {
          relayUrl: relayUrl.trim(),
          relayToken: relayToken.trim() || null,
          serverAlias,
          folderPath: path,
        });
        setFtpItems(folders);
        setLoading(false);
        return;
      } catch (err) {
        // Relay не сработал — пробуем напрямую
        console.warn('Relay failed, trying direct FTP:', err);
        setRadarStatus(`RELAY FAILED → ПРЯМОЙ FTP: ${serverAlias.toUpperCase()}...`);
      }
    } else {
      setRadarStatus(`СОЕДИНЕНИЕ С АРХИВОМ: ${serverAlias.toUpperCase()}...`);
    }

    // Прямой FTP (fallback)
    try {
        const folders = await invoke('get_ftp_folders', { serverAlias, folderPath: path });
        setFtpItems(folders);
    } catch (err) {
        toast(`Сбой подключения к ${serverAlias.toUpperCase()}:\n${err}\n\nЕсли FTP недоступен с этого IP — настройте relay в панели ниже.`);
        setFtpItems([]);
    } finally {
        setLoading(false);
    }
  };

  const goBackFtp = () => {
      if (ftpPath === "/") return;
      const parts = ftpPath.split('/').filter(Boolean);
      parts.pop();
      const parent = parts.length > 0 ? "/" + parts.join('/') : "/";
      fetchFtpRoot(activeServerAlias, parent);
  };

  const handleDownloadFtp = async (serverAlias, folderPath, filename) => {
    const taskId = `${serverAlias}_${filename}_${Date.now()}`;
    setDownloadTasks(prev => ([
      {
        id: taskId,
        filename,
        serverAlias,
        folderPath,
        status: 'running',
        percent: null,
        bytesWritten: 0,
        speedBytesSec: 0,
      },
      ...prev.slice(0, 19),
    ]));

    setLoading(true);
    setRadarStatus(`СКАЧИВАНИЕ ФАЙЛА: ${filename}...`);
    try {
        let report;
        const relayUrl = localStorage.getItem('hyperion_relay_url') || '';
        const relayToken = localStorage.getItem('hyperion_relay_token') || '';

        // Relay или прямой FTP
        if (relayUrl.trim()) {
          setRadarStatus(`RELAY DOWNLOAD: ${filename}...`);
          report = await invoke('relay_download_file', {
            relayUrl: relayUrl.trim(),
            relayToken: relayToken.trim() || null,
            serverAlias,
            folderPath,
            filename,
            taskId,
          });
        } else {
          report = await invoke('download_ftp_file', {
            serverAlias,
            folderPath,
            filename,
            resumeIfExists: resumeDownloads,
            taskId,
          });
        }

        const durationSec = Math.max((report.durationMs || 0) / 1000, 0.001);
        const speedBytesSec = Math.round((report.bytesWritten || 0) / durationSec);

        setDownloadTasks(prev => prev.map(t =>
          t.id === taskId
            ? {
                ...t,
                status: 'done',
                percent: 100,
                bytesWritten: report.totalBytes || report.bytesWritten || 0,
                speedBytesSec,
                savePath: report.savePath,
                resumed: !!report.resumed,
                skipped: !!report.skippedAsComplete,
              }
            : t,
        ));

        toast(`Файл ${filename} скачан в ${report.savePath || 'archives'}`);
    } catch (err) {
        setDownloadTasks(prev => prev.map(t =>
          t.id === taskId ? { ...t, status: 'error', percent: 0, error: String(err) } : t,
        ));
        toast(`Ошибка скачивания: ${err}`);
    } finally {
        setLoading(false);
    }
  };

  // --- NEMESIS: универсальный запуск по введенной цели и выбранному протоколу ---
  const handleStartNemesis = async () => {
    const target = String(targetInput || '').trim();
    if (!target) return toast('Введите IP или URL цели.');

    setLoading(true);
    setFuzzResults([`$ ${attackType} ${target}`]);
    setRadarStatus(`NEMESIS EXECUTE: ${attackType} -> ${target}`);

    try {
      if (attackType === 'CUSTOM_INJECT') {
        const report = await invoke('spider_full_scan', {
          targetUrl: target,
          cookie: hubConfig.cookie,
          maxDepth: spiderMaxDepth,
          maxPages: spiderMaxPages,
          dirBruteforce: spiderDirBrute,
          enableVulnVerification: spiderEnableVulnVerification,
          enableOsintImport: spiderEnableOsintImport,
          enableTopologyDiscovery: spiderEnableTopologyDiscovery,
          enableSnapshotRefresh: spiderEnableSnapshotRefresh,
          enableVideoStreamAnalyzer: spiderEnableVideoStreamAnalyzer,
          enableCredentialDepthAudit: spiderEnableCredentialDepthAudit,
          enablePassiveArpDiscovery: spiderEnablePassiveArpDiscovery,
          enableUptimeMonitoring: spiderEnableUptimeMonitoring,
          enableNeighborDiscovery: spiderEnableNeighborDiscovery,
          enableThreatIntel: spiderEnableThreatIntel,
          enableScheduledAudits: spiderEnableScheduledAudits,
        });
        setSpiderReport(report);
        const apiRows = Array.isArray(report?.apiFuzzResults) ? report.apiFuzzResults : [];
        setFuzzResults(apiRows.map((row) => {
          const code = Number.isFinite(Number(row?.statusCode)) ? ` [${row.statusCode}]` : '';
          return `${row?.protocol || 'GENERIC'}${code} ${row?.endpoint || ''} ${row?.verdict || ''}`.trim();
        }));
      } else {
        const attackMap = {
          RTSP_BRUTE: 'rtsp',
          CGI_EXPLOIT: 'tdkcgi',
          CUSTOM_INJECT: 'generic',
        };
        const results = await invoke('fuzz_cctv_api', {
          targetInput: target,
          attackType: attackMap[attackType] || 'generic',
        });
        const rows = Array.isArray(results) ? results : [];
        setFuzzResults(rows.map((row) => {
          const code = Number.isFinite(Number(row?.statusCode)) ? ` [${row.statusCode}]` : '';
          return `${row?.protocol || attackType}${code} ${row?.endpoint || ''} ${row?.verdict || ''}`.trim();
        }));
      }
    } catch (err) {
      toast(`Ошибка Fuzzer: ${err}`);
    } finally {
      setLoading(false);
    }
  };


  const handlePlayFuzzedLink = async (rawResult) => {
    const urlMatch = rawResult.match(/(http|rtsp):\/\/[^\s]+/);
    if (!urlMatch) return;
    let url = urlMatch[0];

    // Автоматически меняем http на rtsp, так как это скрытые видеопотоки
    if (url.startsWith('http://')) {
      url = url.replace('http://', 'rtsp://');
    }

    // Подставляем креды из полей NEMESIS, если их там ещё нет
    if (fuzzLogin && !url.includes('@')) {
      const user = encodeURIComponent(fuzzLogin);
      const pass = encodeURIComponent(fuzzPassword);
      url = url.replace('rtsp://', `rtsp://${user}:${pass}@`);
    }

    setLoading(true);
    setRadarStatus(`ПЕРЕХВАТ ПОТОКА: ${url.split('@').pop()}...`);
    try {
      if (activeTargetId) {
        await invoke('stop_stream', { targetId: activeTargetId });
        await new Promise(r => setTimeout(r, 500));
      }

      const sessionId = `hijack_${Date.now()}`;
      const wsUrl = await invoke('start_stream', { targetId: sessionId, rtspUrl: url });

      // Обновляем состояния плеера
      setStreamRtspUrl(url);
      setStreamTerminal({ host: targetInput, login: fuzzLogin, password: fuzzPassword, name: 'NEMESIS HIJACK' });
      setStreamChannel({ index: 1, name: 'Fuzzed Stream' });
      setActiveTargetId(sessionId);
      setActiveCameraName(`HIJACK :: ${url.split('@').pop()}`);
      setActiveStream(wsUrl);

    } catch (err) {
      toast("Ошибка перехвата потока: " + err);
    } finally {
      setLoading(false);
    }
  };

  // --- 🔥 НОВАЯ ФУНКЦИЯ: ЗАПУСК FUZZER-ПРОТОКОЛА NEMESIS ---
  const handleAnalyzeSources = async () => {
    if (!fuzzPassword) return toast("Нужен пароль для авторизации!");
    setLoading(true);
    setRadarStatus('АНАЛИЗ DOM-ДЕРЕВА И ПОИСК СКРЫТЫХ API...');
    try {
      const adminHash = await invoke('nemesis_auto_login', { username: fuzzLogin, password: fuzzPassword });

      // Бьем прямо в главную страницу или панель поиска
      const results = await invoke('nemesis_analyze_web_sources', {
        targetUrl: 'https://stream.example.local/check.php',
        adminHash
      });
      setSourceAnalysis(results);
    } catch (err) {
      toast(`Ошибка DOM-анализатора: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const formatBytes = (bytes) => {
    if (!bytes) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB'];
    let value = bytes;
    let idx = 0;
    while (value >= 1024 && idx < units.length - 1) {
      value /= 1024;
      idx++;
    }
    return `${value.toFixed(idx === 0 ? 0 : 1)} ${units[idx]}`;
  };

  const handleRetryDownloadTask = async (task) => {
    if (!task?.serverAlias || !task?.filename) return;
    await handleDownloadFtp(task.serverAlias, task.folderPath || '/', task.filename);
  };

  // =================================================================
  // УНИВЕРСАЛЬНАЯ ВЫГРУЗКА АРХИВА (работает когда FTP мёртв)
  // =================================================================

  // Захват через FFmpeg (RTSP/HTTP → MP4)
  const handleCaptureArchive = async (sourceUrl, filenameHint, durationSec = 60, extraHeaders = null) => {
    const taskId = `capture_${Date.now()}`;
    const displayName = filenameHint || sourceUrl.split('/').pop() || 'capture.mp4';
    const currentLogin = nvr.isapiSearchAuth.login || streamTerminal?.login || 'admin';
    const currentPass = nvr.isapiSearchAuth.pass || streamTerminal?.password || '';

    setDownloadTasks(prev => ([
      {
        id: taskId,
        filename: displayName,
        serverAlias: 'capture',
        folderPath: '',
        status: 'running',
        percent: null,
        bytesWritten: 0,
        speedBytesSec: 0,
        protocol: 'ffmpeg',
      },
      ...prev.slice(0, 29),
    ]));

    try {
      const report = await invoke('capture_archive_segment', {
        sourceUrl,
        filenameHint: displayName,
        durationSeconds: durationSec,
        extraHeaders,
        taskId,
        login: currentLogin,
        pass: currentPass,
      });

      const durationMs = Math.max((report.durationMs || 0) / 1000, 0.001);
      const speed = Math.round((report.bytesWritten || 0) / durationMs);

      setDownloadTasks(prev => prev.map(t =>
        t.id === taskId
          ? { ...t, status: 'done', percent: 100, bytesWritten: report.totalBytes || 0, speedBytesSec: speed, savePath: report.savePath }
          : t
      ));
    } catch (err) {
      setDownloadTasks(prev => prev.map(t =>
        t.id === taskId ? { ...t, status: 'error', percent: 0, error: String(err) } : t
      ));
    }
  };

  // HTTP-скачивание (ISAPI playback, прямые ссылки)
  const handleDownloadHttp = async (url, { login, pass, cookie, filenameHint } = {}) => {
    const taskId = `http_${Date.now()}`;
    const displayName = filenameHint || url.split('/').pop()?.split('?')[0] || 'download.mp4';

    setDownloadTasks(prev => ([
      {
        id: taskId,
        filename: displayName,
        serverAlias: 'http',
        folderPath: '',
        status: 'running',
        percent: null,
        bytesWritten: 0,
        speedBytesSec: 0,
        protocol: 'http',
      },
      ...prev.slice(0, 29),
    ]));

    try {
      const report = await invoke('download_http_archive', {
        url,
        login: login || null,
        pass: pass || null,
        extraCookie: cookie || null,
        filenameHint: displayName,
        taskId,
      });

      const durationMs = Math.max((report.durationMs || 0) / 1000, 0.001);
      const speed = Math.round((report.bytesWritten || 0) / durationMs);

      setDownloadTasks(prev => prev.map(t =>
        t.id === taskId
          ? { ...t, status: 'done', percent: 100, bytesWritten: report.totalBytes || 0, speedBytesSec: speed, savePath: report.savePath }
          : t
      ));
    } catch (err) {
      setDownloadTasks(prev => prev.map(t =>
        t.id === taskId ? { ...t, status: 'error', percent: 0, error: String(err) } : t
      ));
    }
  };

  // --- МАССОВАЯ ЗАГРУЗКА (ОЧЕРЕДЬ) ---
  const handleBatchDownload = async (items) => {
    // items = [{ type: 'capture'|'http'|'ftp', url, login, pass, cookie, filename, duration }, ...]
    const CONCURRENCY = 2; // Макс 2 одновременных загрузки
    let active = 0;
    let queue = [...items];

    const processNext = async () => {
      if (queue.length === 0) return;
      if (active >= CONCURRENCY) return;

      active++;
      const item = queue.shift();

      try {
        if (item.type === 'capture') {
          await handleCaptureArchive(item.url, item.filename, item.duration || 60, item.headers);
        } else if (item.type === 'http') {
          await handleDownloadHttp(item.url, {
            login: item.login, pass: item.pass, cookie: item.cookie, filenameHint: item.filename
          });
        } else if (item.type === 'ftp' && item.serverAlias) {
          await handleDownloadFtp(item.serverAlias, item.folderPath || '/', item.filename);
        }
      } catch (e) {
        console.error('[BATCH] Item failed:', e);
      }

      active--;
      processNext(); // Запускаем следующий
    };

    // Стартуем до CONCURRENCY загрузок параллельно
    for (let i = 0; i < Math.min(CONCURRENCY, items.length); i++) {
      processNext();
    }
  };

  const handleClearDownloads = () => {
    setDownloadTasks(p => p.filter(t => t.status === 'running'));
  };

  const handleCancelDownloadTask = async (task) => {
    if (!task?.id || task.status !== 'running') return;
    try {
      await invoke('cancel_download_task', { taskId: task.id });
      setDownloadTasks(prev => prev.map(t =>
        t.id === task.id ? { ...t, status: 'cancelled', error: 'Отменено пользователем' } : t,
      ));
    } catch (err) {
      toast(`Ошибка отмены загрузки: ${err}`);
    }
  };

  const clearFinishedDownloads = () => {
    setDownloadTasks(prev => prev.filter(t => t.status === 'running'));
  };

  const handleSaveHubToLocal = async (cam) => {
    const lat = mapCenter[0];
    const lng = mapCenter[1];
    const autoId = `hub_${cam.id}_${Date.now()}`;
    const channels = cam.channels.map(ch => ({ id: `ch${ch}`, index: ch, name: `Камера ${parseInt(ch) + 1}` }));
    const payload = JSON.stringify({
      id: autoId, name: `ХАБ: ${cam.ip}`, host: `streamhub_user${cam.id}`, hub_id: cam.id, type: 'hub', lat: lat, lng: lng, channels: channels
    });
    await invoke('save_target', { targetId: autoId, payload });
    loadTargets();
  };

  const handleLocalArchive = async (terminal) => {
    setLoading(true);
    setRadarStatus(`ПРОВЕРКА ПРОТОКОЛОВ NVR: ${terminal.host}`);
    try {
      const probes = await invoke('probe_nvr_protocols', {
        host: terminal.host,
        login: terminal.login || 'admin',
        pass: terminal.password || '',
      });

      nvr.setNvrProbeResults(probes);
      const detected = probes.filter(p => p.status === 'detected').length;
      toast(`ПРОВЕРКА NVR (${terminal.host})\n\nНайдено подтвержденных endpoint: ${detected} из ${probes.length}.\nДетали доступны в панели "NVR PROBE".`);
    } catch (err) {
      toast(`Ошибка проверки протоколов: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleFetchNvrDeviceInfo = async (terminal) => {
    setLoading(true);
    setRadarStatus(`ISAPI DEVICE INFO: ${terminal.host}`);
    try {
      const info = await invoke('fetch_nvr_device_info', {
        host: terminal.host,
        login: terminal.login || 'admin',
        pass: terminal.password || '',
      });
      nvr.setNvrDeviceInfo(info);
    } catch (err) {
      toast(`Ошибка ISAPI deviceInfo: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSearchIsapiRecordings = async (terminal) => {
    setLoading(true);
    setRadarStatus(`ISAPI SEARCH: ${terminal.host}`);
    try {
      const login = terminal.login || 'admin';
      const pass = terminal.password || '';
      const result = await invoke('search_isapi_recordings', {
        host: terminal.host,
        login,
        pass,
        fromTime: nvr.isapiFrom,
        toTime: nvr.isapiTo,
      });
      nvr.setIsapiSearchAuth({ login, pass });
      nvr.setIsapiSearchResults(result);
      const downloadableCount = (result || []).filter((x) => isDownloadableRecord(x)).length;
      const playableCount = (result || []).filter((x) => isPlayableRecord(x)).length;
      const confidences = (result || []).map((x) => Number(x?.confidence ?? 0)).filter((x) => Number.isFinite(x));
      const maxConfidence = confidences.length ? Math.max(...confidences) : 0;
      const avgConfidence = confidences.length ? Math.round(confidences.reduce((a, b) => a + b, 0) / confidences.length) : 0;
      toast(`ISAPI search (${terminal.host})
Найдено записей: ${result.length}
playable: ${playableCount}
downloadable: ${downloadableCount}
confidence(avg/max): ${avgConfidence}/${maxConfidence}`);
    } catch (err) {
      toast(`Ошибка ISAPI search: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleFetchOnvifDeviceInfo = async (terminal) => {
    setLoading(true);
    setRadarStatus(`ONVIF DEVICE INFO: ${terminal.host}`);
    try {
      const info = await invoke('fetch_onvif_device_info', {
        host: terminal.host,
        login: terminal.login || 'admin',
        pass: terminal.password || '',
      });
      nvr.setOnvifDeviceInfo(info);
    } catch (err) {
      toast(`Ошибка ONVIF deviceInfo: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const isDownloadableRecord = (item) => (typeof item?.downloadable === 'boolean' ? item.downloadable : Boolean(item?.playbackUri));
  const isPlayableRecord = (item) => (typeof item?.playable === 'boolean' ? item.playable : Boolean(item?.playbackUri));
  const normalizePlaybackUri = (uri) => String(uri || '').replace(/&amp;/g, '&').trim();
  const getIsapiFilenameHint = (uri, fallback = 'isapi_record.mp4') => {
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
    if (start && end && end > start) {
      return Math.min(1800, Math.max(30, Math.floor((end - start) / 1000) + 15));
    }
    return 120;
  };

  const handleCaptureIsapiPlayback = async (item) => {
    if (!item?.playbackUri) {
      return toast('Для этой записи отсутствует playback URI');
    }
    const durationSec = getIsapiCaptureDurationSeconds(item);
    const normalizedUri = normalizePlaybackUri(item.playbackUri);
    if (!normalizedUri) {
      return toast('Некорректный playback URI для capture');
    }
    await handleCaptureArchive(normalizedUri, getIsapiFilenameHint(normalizedUri, 'isapi_capture.mp4'), durationSec);
  };

  const handleDownloadIsapiPlayback = async (item) => {
    if (!item?.playbackUri) {
      return toast('Для этой записи отсутствует playback URI');
    }
    if (!isDownloadableRecord(item)) {
      return toast('Запись помечена как non-downloadable по probe-классификации. Используй fallback/capture.');
    }

    const normalizedUri = normalizePlaybackUri(item.playbackUri);
    if (!normalizedUri) {
      return toast('Некорректный playback URI для download');
    }
    const filenameHint = getIsapiFilenameHint(normalizedUri, 'isapi_record.mp4');
    const taskId = `isapi_${Date.now()}`;
    setDownloadTasks(prev => ([
      {
        id: taskId,
        filename: filenameHint,
        serverAlias: 'isapi',
        folderPath: '/isapi',
        status: 'running',
        percent: null,
        bytesWritten: 0,
        speedBytesSec: 0,
      },
      ...prev.slice(0, 19),
    ]));

    setLoading(true);
    setRadarStatus('ISAPI DOWNLOAD...');
    try {
      const job = await invoke('start_archive_export_job', {
        playbackUri: normalizedUri,
        login: nvr.isapiSearchAuth.login || 'admin',
        pass: nvr.isapiSearchAuth.pass || '',
        sourceHost: terminal.host || '',
        filenameHint,
        taskId,
      });

      if (!job?.report) {
        const stageSummary = (job?.stages || []).map((s) => `${s.stage}:${s.success ? 'ok' : 'fail'}`).join(' | ');
        const reason = job?.finalReason || (job?.stages || []).filter((s) => !s.success).map((s) => `${s.stage}: ${s.reason || 'failed'}`).join(' || ');
        const stageDetails = (job?.stages || []).map((s) => ({
          stage: s.stage,
          success: !!s.success,
          reason: s.reason || '',
        }));
        setDownloadTasks(prev => prev.map(t =>
          t.id === taskId ? { ...t, status: 'error', percent: 0, error: reason || 'Archive export failed', stageSummary, stageDetails, finalStatus: job?.finalStatus || 'failed', retryCount: Number(job?.retryCount || 0), stageCount: Number(job?.stageCount || stageDetails.length), fallbackDurationSeconds: Number(job?.fallbackDurationSeconds || 0) } : t,
        ));
        toast(`Ошибка ISAPI download: ${reason || 'Archive export failed'}`);
        return;
      }

      const report = job.report;
      const durationSec = Math.max((report.durationMs || 0) / 1000, 0.001);
      const speedBytesSec = Math.round((report.bytesWritten || 0) / durationSec);
      setDownloadTasks(prev => prev.map(t =>
        t.id === taskId
          ? {
              ...t,
              status: 'done',
              percent: 100,
              bytesWritten: report.totalBytes || report.bytesWritten || 0,
              speedBytesSec,
              savePath: report.savePath,
              protocol: job.selectedStage,
              stageSummary: (job.stages || []).map((s) => `${s.stage}:${s.success ? 'ok' : 'fail'}`).join(' | '),
              stageDetails: (job.stages || []).map((s) => ({
                stage: s.stage,
                success: !!s.success,
                reason: s.reason || '',
              })),
              finalStatus: job.finalStatus || 'done',
              retryCount: Number(job.retryCount || 0),
              stageCount: Number(job.stageCount || (job.stages || []).length),
              fallbackDurationSeconds: Number(job.fallbackDurationSeconds || 0),
            }
          : t,
      ));
      if (job.selectedStage !== 'direct') {
        const note = job.finalReason ? `\nПричина direct: ${job.finalReason}` : '';
        toast(`Прямой export отказал, задача завершена через ${job.selectedStage}.${note}`);
      }
    } catch (err) {
      setDownloadTasks(prev => prev.map(t =>
        t.id === taskId ? { ...t, status: 'error', percent: 0, error: String(err) } : t,
      ));
      toast(`Ошибка ISAPI download: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSearchOnvifRecordings = async (terminal) => {
    setLoading(true);
    setRadarStatus(`ONVIF RECORDINGS SEARCH: ${terminal.host}`);
    try {
      const login = terminal.login || 'admin';
      const pass = terminal.password || '';
      const result = await invoke('search_onvif_recordings', {
        host: terminal.host,
        login,
        pass,
      });
      nvr.setOnvifRecordingTokens(result);
      nvr.setOnvifSearchAuth({ login, pass });
      toast(`ONVIF recordings (${terminal.host})
Найдено токенов: ${result.length}`);
    } catch (err) {
      toast(`Ошибка ONVIF recordings search: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleDownloadOnvifToken = async (item) => {
    if (!item?.token || !item?.endpoint) return;

    const taskId = `onvif_${Date.now()}`;
    setDownloadTasks(prev => ([
      {
        id: taskId,
        filename: `onvif_${item.token}.mp4`,
        serverAlias: 'onvif',
        folderPath: '/onvif',
        status: 'running',
        percent: null,
        bytesWritten: 0,
        speedBytesSec: 0,
      },
      ...prev.slice(0, 19),
    ]));

    setLoading(true);
    setRadarStatus('ONVIF DOWNLOAD...');
    try {
      const report = await invoke('download_onvif_recording_token', {
        endpoint: item.endpoint,
        recordingToken: item.token,
        login: nvr.onvifSearchAuth.login || 'admin',
        pass: nvr.onvifSearchAuth.pass || '',
        filenameHint: `onvif_${item.token}.mp4`,
        taskId,
      });

      const durationSec = Math.max((report.durationMs || 0) / 1000, 0.001);
      const speedBytesSec = Math.round((report.bytesWritten || 0) / durationSec);
      setDownloadTasks(prev => prev.map(t =>
        t.id === taskId
          ? { ...t, status: 'done', percent: 100, bytesWritten: report.totalBytes || report.bytesWritten || 0, speedBytesSec, savePath: report.savePath }
          : t,
      ));
    } catch (err) {
      setDownloadTasks(prev => prev.map(t =>
        t.id === taskId ? { ...t, status: 'error', percent: 0, error: String(err) } : t,
      ));
      toast(`Ошибка ONVIF download: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleProbeArchiveExport = async (terminal) => {
    setLoading(true);
    setRadarStatus(`ARCHIVE EXPORT PROBE: ${terminal.host}`);
    try {
      const result = await invoke('probe_archive_export_endpoints', {
        host: terminal.host,
        login: terminal.login || 'admin',
        pass: terminal.password || '',
      });
      nvr.setArchiveProbeResults(result);
      const detected = result.filter((x) => x.status === 'detected').length;
      toast(`ПРОВЕРКА EXPORT-ENDPOINT (${terminal.host})

Найдено потенциальных endpoint: ${detected} из ${result.length}.`);
    } catch (err) {
      toast(`Ошибка проверки export-endpoint: ${err}`);
    } finally {
      setLoading(false);
    }
  };


  const buildRtspUrlFromPath = (activePath, terminalData, channelIndex, cleanHost) => {
    if (!activePath.toLowerCase().startsWith('rtsp://')) {
      const safePath = activePath.replace(/channel=1|ch1|Channels\/1/g, (match) => match.replace('1', channelIndex));
      return `rtsp://${terminalData.login}:${terminalData.password}@${cleanHost}/${safePath.replace(/^\//, '')}`;
    }

    const encodedLogin = encodeURIComponent(terminalData.login || 'admin');
    const encodedPass = encodeURIComponent(terminalData.password || '');
    return activePath
      .replace(/channel=1\b/g, `channel=${channelIndex}`)
      .replace(/ch1\b/g, `ch${channelIndex}`)
      .replace(/Channels\/101\b/g, `Channels/${channelIndex}01`)
      .replace(/\/11(\b|$)/g, `/${channelIndex}1$1`)
      .replace('{login}', encodedLogin)
      .replace('{password}', encodedPass);
  };

  const startSingleStream = async (camera) => {
    if (!camera?.terminal || !camera?.channel) return;
    try {
      if (singleStreamSession?.targetId) {
        await invoke('stop_stream', { targetId: singleStreamSession.targetId });
      }

      const terminalData = camera.terminal;
      const channelData = camera.channel;
      const targetId = `single_${terminalData.id}_${channelData.id}`;
      let wsUrl = '';
      let resolvedRtsp = '';

      if (terminalData.type === 'hub') {
        wsUrl = await invoke('start_hub_stream', {
          targetId,
          userId: terminalData.hub_id.toString(),
          channelId: channelData.index.toString(),
          cookie: hubConfig.cookie,
        });
        resolvedRtsp = 'hub';
      } else {
        const cleanHost = terminalData.host.replace(/^(http:\/\/|https:\/\/|rtsp:\/\/)/i, '').split('/')[0];
        const activePath = await invoke('probe_rtsp_path', {
          host: cleanHost,
          login: terminalData.login,
          pass: terminalData.password,
        });

        resolvedRtsp = buildRtspUrlFromPath(activePath, terminalData, channelData.index, cleanHost);
        wsUrl = await invoke('start_stream', { targetId, rtspUrl: resolvedRtsp });
      }

      setSingleStreamSession({
        targetId,
        wsUrl,
        terminal: terminalData,
        channel: channelData,
        cameraName: `${terminalData.name} :: ${channelData.name}`,
      });
      setSingleStreamCamera(camera);
      setStreamViewMode('single');
    } catch (err) {
      toast(`Ошибка запуска одиночного потока: ${err}`);
      setStreamViewMode('none');
    }
  };

  const filteredTargets = targets.filter((t) => {
    const q = targetSearch.trim().toLowerCase();
    const byQuery = !q || `${t.name || ''} ${t.host || ''}`.toLowerCase().includes(q);
    const byType = targetTypeFilter === 'all' || (targetTypeFilter === 'hub' ? t.type === 'hub' : t.type !== 'hub');
    const byArchive = !archiveOnly || t.type === 'hub';
    return byQuery && byType && byArchive;
  });

  const groupedMapTargets = filteredTargets.reduce((acc, target) => {
    const lat = Number(target.lat);
    const lng = Number(target.lng);
    if (!Number.isFinite(lat) || !Number.isFinite(lng)) return acc;

    const siteKey = target.siteId || `${lat.toFixed(6)}:${lng.toFixed(6)}`;
    const siteName = target.siteName || `Позиция ${lat.toFixed(4)}, ${lng.toFixed(4)}`;

    if (!acc.has(siteKey)) {
      acc.set(siteKey, { id: siteKey, siteName, lat, lng, terminals: [] });
    }

    acc.get(siteKey).terminals.push(target);
    return acc;
  }, new Map());

  const handlePortScan = async () => {
    const host = capture.portScanHost.trim();
    if (!host) return toast('Укажите host/IP для сканирования');

    setLoading(true);
    setRadarStatus(`АНАЛИЗ УЗЛА ${host}...`);
    try {
      const result = await scanHostPorts(host);
      capture.setPortScanResult(result);
    } catch (err) {
      toast(`Ошибка сканирования: ${err}`);
    } finally {
      setLoading(false);
    }
  };

const handleSecurityAudit = async () => {
    const host = capture.portScanHost.trim();
    if (!host) return toast('Укажите host/IP для аудита');

    setLoading(true);
    setRadarStatus(`ГЛУБОКИЙ АУДИТ ЗАГОЛОВКОВ ${host}...`);
    try {
      const targetUrl = host.startsWith('http') ? host : `http://${host}`;
      const results = await invoke('analyze_security_headers', { targetUrl });
      setAuditResults(results);
    } catch (err) {
      toast(`Ошибка аудита: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ display: 'flex', height: '100vh', width: '100vw', backgroundColor: '#0a0a0c', color: '#fff', fontFamily: 'monospace' }}>
      <ToastHost />

      {loading && (
        <div style={{ position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, backgroundColor: 'rgba(0,0,0,0.95)', zIndex: 9999, display: 'flex', flexDirection: 'column', justifyContent: 'center', alignItems: 'center', color: '#00f0ff' }}>
          <div style={{ fontSize: '24px', letterSpacing: '5px', marginBottom: '20px' }}>[ ОБРАБОТКА ДАННЫХ ]</div>
          <div style={{ fontSize: '14px', color: '#ff003c' }}>{radarStatus}</div>
        </div>
      )}

      <ArchiveViewer fetchFtpRoot={fetchFtpRoot} goBackFtp={goBackFtp} handleDownloadFtp={handleDownloadFtp} />


      <main style={{ flex: 1, position: 'relative', overflow: 'hidden', display: 'flex', flexDirection: 'row' }}>
        <div style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
        <div style={{ position: 'absolute', inset: 0, zIndex: 0 }}>
          <StreamGrid
            mapCenter={mapCenter}
            groupedMapTargets={groupedMapTargets}
            fetchFtpRoot={fetchFtpRoot}
            setNemesisTarget={setNemesisTarget}
            handleLocalArchive={handleLocalArchive}
            handleFetchNvrDeviceInfo={handleFetchNvrDeviceInfo}
            handleFetchOnvifDeviceInfo={handleFetchOnvifDeviceInfo}
            onCameraPlayClick={(cam) => setPendingCameraStream(cam)}
            labels={labels}
            onLabelClick={(label) => {
              if (label?.lat && label?.lng) setMapCenter([label.lat, label.lng]);
            }}
          />
        </div>

        <div style={{ position: 'absolute', inset: 0, zIndex: 10, pointerEvents: 'none' }}>
          {streamViewMode === 'single' && singleStreamSession && (
            <div style={{ position: 'absolute', bottom: '20px', left: '20px', width: '420px', height: '280px', pointerEvents: 'auto', boxShadow: '0 4px 20px rgba(0,0,0,0.8)', border: '1px solid #333', background: '#000' }}>
              <StreamPlayer
                streamUrl={singleStreamSession.wsUrl}
                cameraName={singleStreamSession.cameraName}
                terminal={singleStreamSession.terminal}
                channel={singleStreamSession.channel}
                hubCookie={hubConfig.cookie}
                onClose={async () => {
                  if (singleStreamSession?.targetId) {
                    try {
                      await invoke('stop_stream', { targetId: singleStreamSession.targetId });
                    } catch {
                      // noop
                    }
                  }
                  setSingleStreamSession(null);
                  setSingleStreamCamera(null);
                  setStreamViewMode('none');
                }}
              />
            </div>
          )}

          {streamViewMode === 'multi' && (
            <div style={{ position: 'absolute', top: 0, left: 0, bottom: 0, width: '450px', backgroundColor: '#0a0a0c', pointerEvents: 'auto', borderRight: '1px solid #222', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
              <MultiStreamGrid
                terminalId={selectedTerminal}
                targets={filteredTargets}
                hubCookie={hubConfig.cookie}
                onClose={() => {
                  setStreamViewMode('none');
                  setSelectedTerminal(null);
                }}
              />
            </div>
          )}

        </div>
        </div>

        <Sidebar
          targets={targets}
          filteredTargets={filteredTargets}
          targetSearch={targetSearch}
          setTargetSearch={setTargetSearch}
          targetTypeFilter={targetTypeFilter}
          setTargetTypeFilter={setTargetTypeFilter}
          archiveOnly={archiveOnly}
          setArchiveOnly={setArchiveOnly}
          form={form}
          setForm={setForm}
          hubRecon={hubRecon}
          handleSmartSave={handleSmartSave}
          handleDeleteTarget={handleDeleteTarget}
          handleGeocode={handleGeocode}
          onNemesis={(t) => setNemesisTarget(t)}
          onMemoryRequest={(t) => handleLocalArchive(t)}
          onIsapiInfo={(t) => handleFetchNvrDeviceInfo(t)}
          onIsapiSearch={(t) => handleSearchIsapiRecordings(t)}
          onOnvifInfo={(t) => handleFetchOnvifDeviceInfo(t)}
          onOnvifRecordings={(t) => handleSearchOnvifRecordings(t)}
          onArchiveEndpoints={(t) => handleProbeArchiveExport(t)}
          agentScope={agentScope}
          setAgentScope={setAgentScope}
          handleRunReconAgent={handleRunReconAgent}
          agentStatus={agentStatus}
          agentPacket={agentPacket}
          handleAgentHandoff={handleAgentHandoff}
          isSniffing={isSniffing}
          handleStartSniffer={handleStartSniffer}
          interceptLogs={interceptLogs}
          implementationStatus={implementationStatus}
          onPlayCamera={(cam) => setPendingCameraStream(cam)}
          handleStartNemesis={handleStartNemesis}
          runtimeLogs={runtimeLogs}
          setRuntimeLogs={setRuntimeLogs}
          downloadTasks={downloadTasks}
          resumeDownloads={resumeDownloads}
          setResumeDownloads={setResumeDownloads}
          handleCancelDownloadTask={handleCancelDownloadTask}
          handleRetryDownloadTask={handleRetryDownloadTask}
          handleClearDownloads={handleClearDownloads}
          labels={labels}
          setLabels={setLabels}
          onLabelClick={(label) => {
            if (label?.lat && label?.lng) setMapCenter([label.lat, label.lng]);
          }}
          nvr={nvr}
          capture={capture}
          auditResults={auditResults}
          handlePortScan={handlePortScan}
          handleSecurityAudit={handleSecurityAudit}
          handleDownloadIsapiPlayback={handleDownloadIsapiPlayback}
          handleCaptureIsapiPlayback={handleCaptureIsapiPlayback}
          handleDownloadOnvifToken={handleDownloadOnvifToken}
          isPlayableRecord={isPlayableRecord}
          isDownloadableRecord={isDownloadableRecord}
          handleCaptureArchive={handleCaptureArchive}
          handleDownloadHttp={handleDownloadHttp}
          activeTargetId={activeTargetId}
          streamRtspUrl={streamRtspUrl}
          activeCameraName={activeCameraName}
          hubConfig={hubConfig}
          fuzzPath={fuzzPath}
          formatBytes={formatBytes}
        />
      </main>

      {pendingCameraStream && (
        <div style={{
          position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
          backgroundColor: 'rgba(0,0,0,0.7)', zIndex: 9999, display: 'flex',
          alignItems: 'center', justifyContent: 'center', pointerEvents: 'auto'
        }}>
          <div style={{ background: '#111', padding: '20px', border: '1px solid #333', borderRadius: '8px' }}>
            <h3 style={{ marginTop: 0 }}>Как открыть камеру {pendingCameraStream.ip}?</h3>
            <div style={{ display: 'flex', gap: '10px', marginTop: '20px' }}>
              <button onClick={async () => {
                await startSingleStream(pendingCameraStream);
                setPendingCameraStream(null);
              }}>Одиночный вид (Слева снизу)</button>
              <button onClick={() => {
                setStreamViewMode('multi');
                setSelectedTerminal(pendingCameraStream.terminalId || pendingCameraStream.ip);
                setPendingCameraStream(null);
              }}>Мульти-стрим (Боковая панель)</button>
              <button onClick={() => setPendingCameraStream(null)}>Отмена</button>
            </div>
          </div>
        </div>
      )}

      {/* ☢ NEMESIS ARCHIVE TERMINAL OVERLAY */}
      {nemesisTarget && (
        <NemesisArchiveTerminal
          target={nemesisTarget}
          onClose={() => setNemesisTarget(null)}
        />
      )}
    </div>
  );
}
