import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getRuntimeLogs, scanHostPorts } from './api/tauri';
import { listen } from '@tauri-apps/api/event';
import 'leaflet/dist/leaflet.css';
import './App.css';
import NemesisArchiveTerminal from './NemesisArchiveTerminal';
import ArchiveViewer from './features/archive/ArchiveViewer';
import StreamGrid from './features/streams/StreamGrid';
import MultiStreamGrid from './features/streams/MultiStreamGrid';
import StreamPlayer from './StreamPlayer';
import SpiderControl from './features/spider/SpiderControl';
import MassAudit from './features/mass-audit/MassAudit';
import AssetDiscovery from './features/discovery/AssetDiscovery';
import AttackGraph from './features/attack-graph/AttackGraph';
import PlaybookRunner from './features/playbook/PlaybookRunner';
import CampaignList from './features/campaign/CampaignList';
import CampaignDashboard from './features/campaign/CampaignDashboard';
import PassiveScanner from './features/passive-scan/PassiveScanner';
import CameraScanPanel from './features/scan/CameraScanPanel';
import { useAppStore } from './store/appStore';
import ToastHost from './components/ToastHost';
import { toast } from './utils/toast';

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
  const [showPlaybooks, setShowPlaybooks] = useState(false);
  const [showCampaigns, setShowCampaigns] = useState(false);
  const [showIotAudit, setShowIotAudit] = useState(false);
  const [showCameraRadar, setShowCameraRadar] = useState(false);
  const [activeCampaignId, setActiveCampaignId] = useState(null);

  const [addressQuery, setAddressQuery] = useState('');
  const [mapCenter, setMapCenter] = useState([53.9, 27.56]);
  const [form, setForm] = useState({ name: '', host: '', login: 'admin', password: '', lat: 53.9, lng: 27.56, channelCount: 4 });

  const [hubSearch, setHubSearch] = useState('');
  const [hubResults, setHubResults] = useState([]);

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

  const [portScanHost, setPortScanHost] = useState('');
  const [portScanResult, setPortScanResult] = useState([]);
  const [runtimeLogs, setRuntimeLogs] = useState([]);
  const [targetSearch, setTargetSearch] = useState('');
  const [targetTypeFilter, setTargetTypeFilter] = useState('all');
  const [archiveOnly, setArchiveOnly] = useState(false);
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
  const [nvrProbeResults, setNvrProbeResults] = useState([]);
  const [nvrDeviceInfo, setNvrDeviceInfo] = useState(null);
  const [isapiSearchResults, setIsapiSearchResults] = useState([]);
  const [isapiFrom, setIsapiFrom] = useState('2026-01-01T00:00:00Z');
  const [isapiTo, setIsapiTo] = useState('2026-12-31T23:59:59Z');
  const [isapiSearchAuth, setIsapiSearchAuth] = useState({ login: 'admin', pass: '' });
  const [onvifDeviceInfo, setOnvifDeviceInfo] = useState(null);
  const [onvifRecordingTokens, setOnvifRecordingTokens] = useState([]);
  const [onvifSearchAuth, setOnvifSearchAuth] = useState({ login: 'admin', pass: '' });
  const [archiveProbeResults, setArchiveProbeResults] = useState([]);
  const [implementationStatus, setImplementationStatus] = useState(null);
  const [auditResults, setAuditResults] = useState([]);
  const [interceptLogs, setInterceptLogs] = useState([]);
  const [isSniffing, setIsSniffing] = useState(false);


  // --- CAPTURE ARCHIVE STATE ---
  const [captureUrl, setCaptureUrl] = useState('');
  const [captureDuration, setCaptureDuration] = useState(120);
  const [captureFilename, setCaptureFilename] = useState('');

  // --- RECON ARCHIVE ROUTES ---
  const [reconUserId, setReconUserId] = useState('');
  const [reconChannelId, setReconChannelId] = useState('0');
  const [reconDate, setReconDate] = useState('2026-02-19');
  const [reconResults, setReconResults] = useState([]);
  const [reconRunning, setReconRunning] = useState(false);

  // --- RELAY ---
  const [relayUrl, setRelayUrl] = useState(() => {
    try { return localStorage.getItem('hyperion_relay_url') || ''; } catch { return ''; }
  });
  const [relayToken, setRelayToken] = useState(() => {
    try { return localStorage.getItem('hyperion_relay_token') || ''; } catch { return ''; }
  });
  const [relayStatus, setRelayStatus] = useState(null);

  const hubConfig = { cookie: '' };

  const pollIntervalRef = useRef(null);
  const healthCheckRef = useRef(null);
  const activeTargetIdRef = useRef(null);

  useEffect(() => { loadTargets(); }, []);

  useEffect(() => {
    setHubCookie(hubConfig.cookie || '');
  }, [setHubCookie]);

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
    let disposed = false;

    const fetchLogs = async () => {
      try {
        const logs = await getRuntimeLogs(200);
        if (!disposed) {
          setRuntimeLogs(logs);

          const progressLines = logs.filter((line) => line.includes('DOWNLOAD_PROGRESS|'));
          const cancelledLines = logs.filter((line) => line.includes('DOWNLOAD_CANCELLED|'));

          if (progressLines.length > 0 || cancelledLines.length > 0) {
            setDownloadTasks((prev) => {
              let next = [...prev];
              for (const line of progressLines) {
                const raw = line.split('DOWNLOAD_PROGRESS|')[1] || '';
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
                const raw = line.split('DOWNLOAD_CANCELLED|')[1] || '';
                const [taskId] = raw.split('|');
                if (!taskId) continue;
                next = next.map((t) =>
                  t.id === taskId ? { ...t, status: 'cancelled', error: 'Отменено пользователем' } : t,
                );
              }

              return next;
            });
          }
        }
      } catch (e) {}
    };

    fetchLogs();
    const timer = setInterval(fetchLogs, 2000);
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  }, []);


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
      for (let key of keys) {
        const jsonStr = await invoke('read_target', { targetId: key });
        loaded.push(JSON.parse(jsonStr));
      }
      setTargets(normalizeTargetRecords(loaded));
    } catch (err) { console.error(err); }
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
      const [lat, lng] = await invoke('geocode_address', { address: addressQuery });
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
        const res = await invoke('search_global_hub', { query: hubSearch, cookie: hubConfig.cookie });
        if (res.length === 0) toast("Поиск не дал результатов. Проверьте адрес или обновите PHPSESSID в коде!");
        setHubResults(res);
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
    const currentLogin = isapiSearchAuth.login || streamTerminal?.login || 'admin';
    const currentPass = isapiSearchAuth.pass || streamTerminal?.password || '';

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

      setNvrProbeResults(probes);
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
      setNvrDeviceInfo(info);
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
        fromTime: isapiFrom,
        toTime: isapiTo,
      });
      setIsapiSearchAuth({ login, pass });
      setIsapiSearchResults(result);
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
      setOnvifDeviceInfo(info);
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
        login: isapiSearchAuth.login || 'admin',
        pass: isapiSearchAuth.pass || '',
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
      setOnvifRecordingTokens(result);
      setOnvifSearchAuth({ login, pass });
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
        login: onvifSearchAuth.login || 'admin',
        pass: onvifSearchAuth.pass || '',
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
      setArchiveProbeResults(result);
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
    const host = portScanHost.trim();
    if (!host) return toast('Укажите host/IP для сканирования');

    setLoading(true);
    setRadarStatus(`АНАЛИЗ УЗЛА ${host}...`);
    try {
      const result = await scanHostPorts(host);
      setPortScanResult(result);
    } catch (err) {
      toast(`Ошибка сканирования: ${err}`);
    } finally {
      setLoading(false);
    }
  };

const handleSecurityAudit = async () => {
    const host = portScanHost.trim();
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


      <main style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
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

          <div style={{ position: 'absolute', top: 16, right: 16, width: '400px', backgroundColor: '#111115', borderLeft: '2px solid #ff003c', padding: '20px', maxHeight: 'calc(100% - 32px)', overflowY: 'auto', pointerEvents: 'auto' }}>

        <h2 style={{ color: '#ff003c', fontSize: '1.2rem', marginBottom: '20px' }}>HYPERION NODE</h2>

        <div style={{ border: '1px solid #222', padding: '8px', marginBottom: '12px' }}>
          <button style={{ width: '100%', marginBottom: '6px' }} onClick={() => setShowPlaybooks(v => !v)}>PLAYBOOKS</button>
          {showPlaybooks && <PlaybookRunner />}
          <button style={{ width: '100%', marginBottom: '6px' }} onClick={() => setShowCampaigns(v => !v)}>КАМПАНИИ</button>
          {showCampaigns && (
            <>
              <CampaignList onOpen={setActiveCampaignId} />
              <CampaignDashboard campaignId={activeCampaignId} />
            </>
          )}
          <button style={{ width: '100%' }} onClick={() => setShowIotAudit(v => !v)}>IoT АУДИТ</button>
          {showIotAudit && <PassiveScanner />}

          <button style={{ width: '100%', marginTop: '6px', marginBottom: '6px', backgroundColor: showCameraRadar ? '#00f0ff' : '#111', color: showCameraRadar ? '#000' : '#fff' }} onClick={() => setShowCameraRadar(v => !v)}>
            УЛЬТИМАТИВНЫЙ РАДАР КАМЕР
          </button>
          {showCameraRadar && <CameraScanPanel onPlayCamera={(cam) => setPendingCameraStream(cam)} />}

          <button
            onClick={handleStartSniffer}
            disabled={isSniffing}
            style={{ width: '100%', padding: '8px', background: isSniffing ? '#003300' : '#00ffcc', color: '#000', fontWeight: 'bold', marginTop: '6px', marginBottom: '6px', cursor: isSniffing ? 'default' : 'pointer' }}
          >
            {isSniffing ? '🎧 СЛУШАЮ ЭФИР (СНИФФЕР)...' : '🎧 ЗАПУСТИТЬ ПАССИВНЫЙ ПЕРЕХВАТ'}
          </button>

          {interceptLogs.length > 0 && (
            <div style={{ background: '#111', padding: '10px', border: '1px solid #00ffcc', marginBottom: '10px', fontSize: '11px', color: '#00ffcc' }}>
              <div>УЯЗВИМЫЙ ТРАФИК В СЕТИ:</div>
              {interceptLogs.map((log, i) => (
                <div key={`intercept_${i}`}>[{log.protocol}] {log.details}</div>
              ))}
            </div>
          )}
        </div>

        <div style={{ border: '1px solid #2f9a4f', padding: '10px', backgroundColor: '#07130b', marginBottom: '20px' }}>
          <h3 style={{ color: '#7dff9c', marginTop: '0', fontSize: '0.9rem' }}>📌 СТАТУС РЕАЛИЗАЦИИ</h3>
          {implementationStatus ? (
            <>
              <div style={{ color: '#c9ffd6', fontSize: '12px', marginBottom: '8px' }}>
                Выполнено: <b>{implementationStatus.completed}/{implementationStatus.total}</b> · В работе: <b>{implementationStatus.in_progress}</b> · Осталось: <b>{implementationStatus.left}</b>
              </div>
              <div style={{ maxHeight: '120px', overflowY: 'auto', border: '1px solid #15361f', padding: '6px', background: '#020a05' }}>
                {(implementationStatus.items || []).map((item, idx) => (
                  <div key={`${item.name}_${idx}`} style={{ color: '#8ed9a2', fontSize: '11px', marginBottom: '4px' }}>
                    {item.status === 'completed' ? '✅' : item.status === 'in_progress' ? '🛠️' : '⏳'} {item.name}
                  </div>
                ))}
              </div>
            </>
          ) : (
            <div style={{ color: '#7aa887', fontSize: '11px' }}>Загрузка статуса...</div>
          )}
        </div>

        <MassAudit />


        <SpiderControl handleStartNemesis={handleStartNemesis} handleAnalyzeSources={handleAnalyzeSources} handlePlayFuzzedLink={handlePlayFuzzedLink} />


        {/* =============== РАЗВЕДКА АРХИВНЫХ МАРШРУТОВ =============== */}
        <div style={{ border: '1px solid #00ff9c', padding: '10px', backgroundColor: '#001a0a', marginBottom: '20px', boxShadow: '0 0 10px rgba(0,255,156,0.15)' }}>
          <h3 style={{ color: '#00ff9c', marginTop: '0', fontSize: '0.9rem' }}>🔍 РАЗВЕДКА АРХИВА (HUB)</h3>
          <div style={{ fontSize: '10px', color: '#6b9', marginBottom: '8px' }}>
            Прощупывает все PHP-эндпоинты stream.example.local на наличие архивного доступа для конкретной камеры.
          </div>

          <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
            <input
              style={{ flex: 1, backgroundColor: '#000', border: '1px solid #00ff9c', color: '#00ff9c', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
              placeholder="User ID (напр. 1234)"
              value={reconUserId}
              onChange={e => setReconUserId(e.target.value)}
            />
            <input
              style={{ flex: 1, backgroundColor: '#000', border: '1px solid #00ff9c', color: '#00ff9c', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
              placeholder="Channel (0,1,2...)"
              value={reconChannelId}
              onChange={e => setReconChannelId(e.target.value)}
            />
            <input
              type="date"
              style={{ flex: 1, backgroundColor: '#000', border: '1px solid #00ff9c', color: '#00ff9c', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
              value={reconDate}
              onChange={e => setReconDate(e.target.value)}
            />
          </div>

          <button
            disabled={reconRunning}
            onClick={async () => {
              if (!reconUserId.trim()) return toast('Введите User ID камеры');
              setReconRunning(true);
              setReconResults([]);
              try {
                const results = await invoke('recon_hub_archive_routes', {
                  userId: reconUserId,
                  channelId: reconChannelId,
                  adminHash: hubConfig.cookie.split('admin=')[1]?.split(';')[0]?.trim() || '',
                  targetDate: reconDate || null,
                  targetFtpPath: fuzzPath || null,
                });
                setReconResults(results);
              } catch (err) {
                toast(`Ошибка разведки: ${err}`);
              } finally {
                setReconRunning(false);
              }
            }}
            style={{ width: '100%', backgroundColor: reconRunning ? '#333' : '#00ff9c', color: '#000', border: 'none', padding: '8px', cursor: reconRunning ? 'wait' : 'pointer', fontWeight: 'bold', fontSize: '11px', letterSpacing: '1px' }}
          >
            {reconRunning ? '⏳ РАЗВЕДКА...' : '🔍 ЗАПУСТИТЬ РАЗВЕДКУ МАРШРУТОВ'}
          </button>

          {reconResults.length > 0 && (
            <div style={{ marginTop: '10px', border: '1px solid #00ff9c', background: '#000', maxHeight: '300px', overflowY: 'auto', padding: '6px' }}>
              <div style={{ color: '#00ff9c', fontSize: '10px', fontWeight: 'bold', marginBottom: '6px' }}>
                РЕЗУЛЬТАТЫ: {reconResults.length} маршрутов | {reconResults.filter(r => r.isVideo).length} видео | {reconResults.filter(r => r.isRedirect).length} редиректов
              </div>
              {reconResults.map((r, idx) => (
                <div key={idx} style={{
                  borderBottom: '1px solid #112',
                  padding: '6px 0',
                  opacity: r.verdict.includes('НЕ НАЙДЕНО') || r.verdict.includes('ПУСТО') ? 0.4 : 1
                }}>
                  <div style={{ fontSize: '10px', color: r.isVideo ? '#00ff9c' : r.isRedirect ? '#ffcc00' : '#888', fontWeight: r.isVideo ? 'bold' : 'normal' }}>
                    {r.verdict}
                  </div>
                  <div style={{ fontSize: '9px', color: '#666', wordBreak: 'break-all' }}>
                    {r.method} {r.url}
                  </div>
                  <div style={{ fontSize: '9px', color: '#555' }}>
                    HTTP {r.statusCode} | {r.contentType || 'n/a'} | {r.contentLength > 0 ? formatBytes(r.contentLength) : '0'}
                  </div>
                  {r.bodyPreview && r.bodyPreview.length > 10 && !r.bodyPreview.startsWith('[') && (
                    <div style={{ fontSize: '9px', color: '#444', marginTop: '2px', maxHeight: '30px', overflow: 'hidden' }}>
                      {r.bodyPreview.substring(0, 150)}
                    </div>
                  )}
                  {/* Кнопка: захватить найденное видео */}
                  {r.isVideo && (
                    <button
                      onClick={() => {
                        setCaptureUrl(r.url);
                        handleCaptureArchive(r.url, `recon_${reconUserId}_ch${reconChannelId}_${reconDate}.mp4`, captureDuration, `Cookie: ${hubConfig.cookie}\r\nReferer: https://stream.example.local/stream/admin.php\r\n`);
                      }}
                      style={{ marginTop: '4px', background: '#1a4a1a', color: '#00ff9c', border: '1px solid #00ff9c', padding: '3px 8px', cursor: 'pointer', fontSize: '9px', fontWeight: 'bold' }}
                    >
                      🎬 ЗАХВАТИТЬ ЭТОТ ПОТОК
                    </button>
                  )}
                  {r.isRedirect && r.redirectTo && (
                    <button
                      onClick={() => {
                        const fullUrl = r.redirectTo.startsWith('http') ? r.redirectTo : `https://stream.example.local${r.redirectTo}`;
                        setCaptureUrl(fullUrl);
                      }}
                      style={{ marginTop: '4px', background: '#4a4a1a', color: '#ffcc00', border: '1px solid #ffcc00', padding: '3px 8px', cursor: 'pointer', fontSize: '9px' }}
                    >
                      ↗️ СЛЕДОВАТЬ ЗА РЕДИРЕКТОМ
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        <AssetDiscovery />
        <AttackGraph targets={filteredTargets} />

        <div style={{ border: '1px solid #ff003c', padding: '10px', backgroundColor: '#1a0505', marginBottom: '20px' }}>
          <h3 style={{ color: '#ff003c', marginTop: '0', fontSize: '0.9rem' }}>GLOBAL HUB: MVD LINK</h3>
          <div style={{ display: 'flex', marginBottom: '10px' }}>
              <input style={{ flex: 1, backgroundColor: '#000', border: '1px solid #ff003c', color: '#ff003c', padding: '8px' }} placeholder="Улица, дом..." value={hubSearch} onChange={e => setHubSearch(e.target.value)} />
              <button style={{ backgroundColor: '#ff003c', color: '#fff', border: 'none', padding: '8px', cursor: 'pointer', fontWeight: 'bold' }} onClick={handleHubSearch}>СКАН</button>
          </div>

          {hubResults.map(cam => (
              <div key={cam.id} style={{ border: '1px solid #444', padding: '10px', marginBottom: '8px', backgroundColor: '#050505' }}>
                  <div style={{ color: '#fff', fontWeight: 'bold', fontSize: '12px', marginBottom: '5px' }}>{cam.ip}</div>
                  <div style={{ fontSize: '10px', color: '#888', marginBottom: '8px' }}>USER ID: {cam.id} | Камер: {cam.channels.length}</div>

                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '5px', marginBottom: '8px' }}>
                      {cam.channels.map(ch => (
                          <button key={ch} onClick={() => handleHubStream(cam.id, ch, cam.ip)} style={{ backgroundColor: '#00f0ff', color: '#000', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '10px', fontWeight: 'bold' }}>
                            LIVE: К-{parseInt(ch) + 1}
                          </button>
                      ))}
                  </div>

                  <div style={{ display: 'flex', gap: '5px' }}>
                      <button onClick={() => fetchFtpRoot('video1')} style={{ flex: 1, backgroundColor: '#1a4a4a', color: '#00f0ff', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                        📁 АРХИВ (FTP)
                      </button>

                      <button onClick={() => handleSaveHubToLocal(cam)} style={{ flex: 1, backgroundColor: '#4a1a4a', color: '#ff00ff', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                        📌 В БАЗУ
                      </button>
                  </div>
              </div>
          ))}
        </div>

        <hr style={{ borderColor: '#222' }} />

        {/* =============== ЗАХВАТ АРХИВА (УНИВЕРСАЛЬНЫЙ) =============== */}
        <div style={{ marginTop: '20px', border: '1px solid #ff9900', padding: '10px', backgroundColor: '#1a1100', marginBottom: '20px' }}>
          <h3 style={{ color: '#ff9900', marginTop: '0', fontSize: '0.9rem' }}>📦 ЗАХВАТ АРХИВА (FFmpeg / HTTP)</h3>
          <div style={{ fontSize: '10px', color: '#aa8833', marginBottom: '8px' }}>
            Введите RTSP, HTTP или MJPEG URL источника. FFmpeg захватит видео в MP4.
          </div>

          <input
            style={{ width: '100%', backgroundColor: '#000', border: '1px solid #ff9900', color: '#ff9900', padding: '8px', marginBottom: '6px', boxSizing: 'border-box', fontSize: '11px' }}
            placeholder="rtsp://admin:pass@192.168.1.100/Streaming/tracks/101"
            value={captureUrl}
            onChange={e => setCaptureUrl(e.target.value)}
          />

          <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
            <input
              style={{ flex: 2, backgroundColor: '#000', border: '1px solid #ff9900', color: '#ff9900', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
              placeholder="Имя файла (авто)"
              value={captureFilename}
              onChange={e => setCaptureFilename(e.target.value)}
            />
            <input
              type="number"
              style={{ flex: 1, backgroundColor: '#000', border: '1px solid #ff9900', color: '#ff9900', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
              placeholder="Сек"
              value={captureDuration}
              onChange={e => setCaptureDuration(parseInt(e.target.value) || 60)}
            />
          </div>

          <div style={{ display: 'flex', gap: '6px' }}>
            <button
              onClick={() => {
                if (!captureUrl.trim()) return toast('Введите URL источника');
                handleCaptureArchive(captureUrl, captureFilename || null, captureDuration);
              }}
              style={{ flex: 1, backgroundColor: '#ff9900', color: '#000', border: 'none', padding: '8px', cursor: 'pointer', fontWeight: 'bold', fontSize: '11px' }}
            >
              🎬 ЗАХВАТ (FFmpeg)
            </button>
            <button
              onClick={() => {
                if (!captureUrl.trim()) return toast('Введите URL для скачивания');
                handleDownloadHttp(captureUrl, { filenameHint: captureFilename || null });
              }}
              style={{ flex: 1, backgroundColor: '#1a4a1a', color: '#9f9', border: '1px solid #4a4', padding: '8px', cursor: 'pointer', fontWeight: 'bold', fontSize: '11px' }}
            >
              ⬇ HTTP ПРЯМАЯ
            </button>
          </div>

          {/* Быстрые кнопки для текущего стрима */}
          {activeTargetId && streamRtspUrl && streamRtspUrl !== 'hub' && (
            <button
              onClick={() => handleCaptureArchive(streamRtspUrl, `${activeCameraName.replace(/[^a-zA-Zа-яА-Я0-9]/g, '_')}_${Date.now()}.mp4`, captureDuration)}
              style={{ width: '100%', marginTop: '6px', backgroundColor: '#4a3a1a', color: '#ffd27a', border: '1px solid #ff9900', padding: '8px', cursor: 'pointer', fontWeight: 'bold', fontSize: '11px' }}
            >
              📹 ЗАПИСАТЬ ТЕКУЩИЙ СТРИМ ({captureDuration}с)
            </button>
          )}
        </div>

        <div style={{ marginTop: '20px' }}>
          <h3 style={{ color: '#00f0ff', fontSize: '0.9rem', marginBottom: '10px' }}>АНАЛИЗАТОР УЗЛА (ПОРТЫ И ЗАЩИТА)</h3>
          <div style={{ display: 'flex', gap: '6px', marginBottom: '10px' }}>
            <input
              style={{ flex: 1, backgroundColor: '#000', border: '1px solid #333', color: '#00f0ff', padding: '10px', boxSizing: 'border-box' }}
              placeholder='IP/Host (пример: 192.168.1.100)'
              value={portScanHost}
              onChange={e => setPortScanHost(e.target.value)}
            />
            <button onClick={handlePortScan} style={{ backgroundColor: '#1a4a4a', color: '#00f0ff', border: '1px solid #00f0ff', cursor: 'pointer', padding: '0 12px', fontWeight: 'bold' }}>ПОРТЫ</button>
            <button onClick={handleSecurityAudit} style={{ backgroundColor: '#4a1a4a', color: '#ff00ff', border: '1px solid #ff00ff', cursor: 'pointer', padding: '0 12px', fontWeight: 'bold' }}>АУДИТ</button>
          </div>

          {portScanResult.length > 0 && (
            <div style={{ border: '1px solid #222', background: '#050505', marginBottom: '10px' }}>
              {portScanResult.map((item) => (
                <div key={item.port} style={{ display: 'flex', justifyContent: 'space-between', padding: '8px 10px', borderBottom: '1px solid #111', fontSize: '11px' }}>
                  <span style={{ color: '#aaa' }}>{item.port} / {item.service}</span>
                  <span style={{ color: item.open ? '#00ff9c' : '#ff5555', fontWeight: 'bold' }}>{item.open ? 'OPEN' : 'CLOSED'}</span>
                </div>
              ))}
            </div>
          )}

          {auditResults.length > 0 && (
            <div style={{ border: '1px solid #ff00ff', background: '#1a001a', padding: '8px' }}>
              <div style={{ color: '#ff00ff', fontSize: '10px', marginBottom: '6px', fontWeight: 'bold' }}>РЕЗУЛЬТАТЫ ГЛУБОКОГО АУДИТА:</div>
              {auditResults.map((line, idx) => (
                <div key={idx} style={{ fontSize: '11px', color: line.includes('🔴') ? '#ff5555' : line.includes('🟢') ? '#00ff9c' : '#aaa', marginBottom: '4px' }}>
                  {line}
                </div>
              ))}
            </div>
          )}
        </div>

        <hr style={{ borderColor: '#222' }} />

        <div style={{ marginTop: '20px' }}>
          <h3 style={{ color: '#00f0ff', fontSize: '0.9rem', marginBottom: '10px' }}>NVR PROBE (ISAPI/ONVIF)</h3>
          <div style={{ border: '1px solid #222', background: '#050505', maxHeight: '180px', overflowY: 'auto', padding: '8px' }}>
            {nvrProbeResults.length === 0 && (
              <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «⏳ ЗАПРОС ПАМЯТИ» у локальной цели.</div>
            )}
            {nvrProbeResults.map((r, idx) => (
              <div key={`${r.protocol}_${r.endpoint}_${idx}`} style={{ borderBottom: '1px solid #111', padding: '6px 0' }}>
                <div style={{ fontSize: '10px', color: '#bbb' }}>{r.protocol}</div>
                <div style={{ fontSize: '10px', color: '#777', wordBreak: 'break-all' }}>{r.endpoint}</div>
                <div style={{ fontSize: '10px', color: r.status === 'detected' ? '#00ff9c' : r.status === 'not_detected' ? '#ffcc66' : '#ff5555' }}>{r.status}</div>
              </div>
            ))}
          </div>
        </div>

        <div style={{ marginTop: '14px' }}>
          <h3 style={{ color: '#00f0ff', fontSize: '0.85rem', marginBottom: '8px' }}>ISAPI DEVICE INFO</h3>
          <div style={{ border: '1px solid #222', background: '#050505', maxHeight: '160px', overflowY: 'auto', padding: '8px' }}>
            {!nvrDeviceInfo && <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «ℹ ISAPI DEVICE INFO» у локальной цели.</div>}
            {nvrDeviceInfo && (
              <>
                <div style={{ color: '#aaa', fontSize: '10px', marginBottom: '6px', wordBreak: 'break-all' }}>{nvrDeviceInfo.endpoint} [{nvrDeviceInfo.status}]</div>
                <pre style={{ margin: 0, color: '#9fc2ff', fontSize: '10px', whiteSpace: 'pre-wrap' }}>{nvrDeviceInfo.bodyPreview || 'empty'}</pre>
              </>
            )}
          </div>
        </div>

        <div style={{ marginTop: '14px' }}>
          <h3 style={{ color: '#9fd7ff', fontSize: '0.85rem', marginBottom: '8px' }}>ISAPI SEARCH RESULTS</h3>
          <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
            <input value={isapiFrom} onChange={(e) => setIsapiFrom(e.target.value)} style={{ flex: 1, background: '#000', color: '#9fd7ff', border: '1px solid #1f2d4a', padding: '4px', fontSize: '10px' }} placeholder='from' />
            <input value={isapiTo} onChange={(e) => setIsapiTo(e.target.value)} style={{ flex: 1, background: '#000', color: '#9fd7ff', border: '1px solid #1f2d4a', padding: '4px', fontSize: '10px' }} placeholder='to' />
          </div>
          <div style={{ border: '1px solid #1f2d4a', background: '#05070b', maxHeight: '160px', overflowY: 'auto', padding: '8px' }}>
            {isapiSearchResults.length === 0 && <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «🔎 ISAPI SEARCH RECORDS» у локальной цели.</div>}
            {isapiSearchResults.map((item, idx) => (
              <div key={`${item.endpoint}_${idx}`} style={{ borderBottom: '1px solid #111', padding: '6px 0', fontSize: '10px' }}>
                <div style={{ color: '#90b8d8', wordBreak: 'break-all' }}>{item.endpoint}</div>
                <div style={{ color: '#7fa9cb' }}>track: {item.trackId || '-'}</div>
                <div style={{ color: '#7fa9cb' }}>start: {item.startTime || '-'}</div>
                <div style={{ color: '#7fa9cb' }}>end: {item.endTime || '-'}</div>
                <div style={{ color: '#7fa9cb' }}>
                  probe: transport={item.transport || '-'} | playable={String(isPlayableRecord(item))} | downloadable={String(isDownloadableRecord(item))} | conf={item.confidence ?? 0}
                </div>
                <div style={{ color: '#9fd7ff', wordBreak: 'break-all' }}>uri: {item.playbackUri || '-'}</div>
                {item.playbackUri && (
                  <div style={{ display: 'flex', gap: '6px', marginTop: '6px' }}>
                    <button onClick={() => handleDownloadIsapiPlayback(item)} disabled={!isDownloadableRecord(item)} style={{ background: isDownloadableRecord(item) ? '#1f3a2a' : '#1a1a1a', color: isDownloadableRecord(item) ? '#9fffc5' : '#666', border: isDownloadableRecord(item) ? '1px solid #38a169' : '1px solid #333', padding: '3px 6px', cursor: isDownloadableRecord(item) ? 'pointer' : 'not-allowed', fontSize: '10px', opacity: isDownloadableRecord(item) ? 1 : 0.7 }}>
                      {isDownloadableRecord(item) ? '⬇ DOWNLOAD BY URI' : 'NO-DL (probe)'}
                    </button>
                    <button onClick={() => handleCaptureIsapiPlayback(item)} style={{ background: '#12263d', color: '#9fd7ff', border: '1px solid #2f6aa3', padding: '3px 6px', cursor: 'pointer', fontSize: '10px' }}>
                      ◉ CAPTURE FALLBACK
                    </button>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>

        <div style={{ marginTop: '14px' }}>
          <h3 style={{ color: '#00f0ff', fontSize: '0.85rem', marginBottom: '8px' }}>ONVIF DEVICE INFO</h3>
          <div style={{ border: '1px solid #222', background: '#050505', maxHeight: '160px', overflowY: 'auto', padding: '8px' }}>
            {!onvifDeviceInfo && <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «ℹ ONVIF DEVICE INFO» у локальной цели.</div>}
            {onvifDeviceInfo && (
              <>
                <div style={{ color: '#aaa', fontSize: '10px', marginBottom: '6px', wordBreak: 'break-all' }}>{onvifDeviceInfo.endpoint} [{onvifDeviceInfo.status}]</div>
                <pre style={{ margin: 0, color: '#a8ffb0', fontSize: '10px', whiteSpace: 'pre-wrap' }}>{onvifDeviceInfo.bodyPreview || 'empty'}</pre>
              </>
            )}
          </div>
        </div>

        <div style={{ marginTop: '14px' }}>
          <h3 style={{ color: '#b9ffcf', fontSize: '0.85rem', marginBottom: '8px' }}>ONVIF RECORDING TOKENS</h3>
          <div style={{ border: '1px solid #2a5a36', background: '#050b06', maxHeight: '130px', overflowY: 'auto', padding: '8px' }}>
            {onvifRecordingTokens.length === 0 && <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «🔎 ONVIF RECORDINGS» у локальной цели.</div>}
            {onvifRecordingTokens.map((item, idx) => (
              <div key={`${item.endpoint}_${item.token}_${idx}`} style={{ borderBottom: '1px solid #132418', padding: '6px 0' }}>
                <div style={{ color: '#88c89b', fontSize: '10px', wordBreak: 'break-all' }}>{item.endpoint}</div>
                <div style={{ color: '#b9ffcf', fontSize: '10px' }}>token: {item.token}</div>
                <button onClick={() => handleDownloadOnvifToken(item)} style={{ marginTop: '6px', background: '#1f3a2a', color: '#b9ffcf', border: '1px solid #38a169', padding: '3px 6px', cursor: 'pointer', fontSize: '10px' }}>
                  ⬇ DOWNLOAD TOKEN
                </button>
              </div>
            ))}
          </div>
        </div>

        <div style={{ marginTop: '14px' }}>
          <h3 style={{ color: '#ffd27a', fontSize: '0.85rem', marginBottom: '8px' }}>ARCHIVE EXPORT ENDPOINTS</h3>
          <div style={{ border: '1px solid #3a2a1a', background: '#0b0805', maxHeight: '160px', overflowY: 'auto', padding: '8px' }}>
            {archiveProbeResults.length === 0 && <div style={{ color: '#666', fontSize: '11px' }}>Нет данных. Нажми «📦 PROBE EXPORT ENDPOINTS» у локальной цели.</div>}
            {archiveProbeResults.map((item, idx) => (
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

        <hr style={{ borderColor: '#222' }} />

        <div style={{ marginTop: '20px' }}>
          <h3 style={{ color: '#00f0ff', fontSize: '0.9rem', marginBottom: '10px' }}>РЕГИСТРАЦИЯ ЛОКАЛЬНОГО УЗЛА</h3>
          <input style={{ width: '100%', backgroundColor: '#000', border: '1px solid #333', color: '#00f0ff', padding: '10px', marginBottom: '8px', boxSizing: 'border-box' }} placeholder="Имя узла" value={form.name} onChange={e => setForm({ ...form, name: e.target.value })} />
          <input style={{ width: '100%', backgroundColor: '#000', border: '1px solid #333', color: '#00f0ff', padding: '10px', marginBottom: '8px', boxSizing: 'border-box' }} placeholder="IP (напр. 93.125.3.58:554)" value={form.host} onChange={e => setForm({ ...form, host: e.target.value })} />
          <input style={{ width: '100%', backgroundColor: '#000', border: '1px solid #333', color: '#00f0ff', padding: '10px', marginBottom: '8px', boxSizing: 'border-box' }} placeholder="Логин" value={form.login} onChange={e => setForm({ ...form, login: e.target.value })} />
          <input style={{ width: '100%', backgroundColor: '#000', border: '1px solid #333', color: '#00f0ff', padding: '10px', marginBottom: '8px', boxSizing: 'border-box' }} type="password" placeholder="Пароль" value={form.password} onChange={e => setForm({ ...form, password: e.target.value })} />
          <input style={{ width: '100%', backgroundColor: '#000', border: '1px solid #333', color: '#00f0ff', padding: '10px', marginBottom: '15px', boxSizing: 'border-box' }} type="number" placeholder="Каналы" value={form.channelCount} onChange={e => setForm({ ...form, channelCount: e.target.value })} />

          <div style={{ display: 'flex', gap: '5px', marginBottom: '15px' }}>
            <input style={{ flex: 1, backgroundColor: '#000', border: '1px solid #333', color: '#00f0ff', padding: '10px', boxSizing: 'border-box' }} placeholder="Координаты" value={addressQuery} onChange={e => setAddressQuery(e.target.value)} />
            <button style={{ backgroundColor: '#1a4a4a', color: '#00f0ff', border: '1px solid #00f0ff', cursor: 'pointer', padding: '0 15px' }} onClick={handleGeocode}>GEO</button>
          </div>

          <button style={{ width: '100%', padding: '12px', backgroundColor: '#00f0ff', color: '#000', fontWeight: 'bold', cursor: 'pointer', border: 'none', boxSizing: 'border-box' }} onClick={handleSmartSave}>ENCRYPT DATA</button>
        </div>

        <h3 style={{ color: '#00f0ff', marginTop: '40px', fontSize: '0.9rem' }}>БАЗА ЦЕЛЕЙ</h3>
        <div style={{ border: '1px solid #222', background: '#050505', padding: '8px', marginBottom: '10px' }}>
          <input
            style={{ width: '100%', backgroundColor: '#000', border: '1px solid #333', color: '#00f0ff', padding: '8px', marginBottom: '6px', boxSizing: 'border-box' }}
            placeholder='Поиск по имени/IP'
            value={targetSearch}
            onChange={e => setTargetSearch(e.target.value)}
          />
          <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
            <button onClick={() => setTargetTypeFilter('all')} style={{ flex: 1, background: targetTypeFilter === 'all' ? '#1a4a4a' : '#111', color: '#00f0ff', border: '1px solid #00f0ff', padding: '6px', cursor: 'pointer', fontSize: '11px' }}>ВСЕ</button>
            <button onClick={() => setTargetTypeFilter('hub')} style={{ flex: 1, background: targetTypeFilter === 'hub' ? '#4a1a4a' : '#111', color: '#ff00ff', border: '1px solid #ff00ff', padding: '6px', cursor: 'pointer', fontSize: '11px' }}>HUB</button>
            <button onClick={() => setTargetTypeFilter('local')} style={{ flex: 1, background: targetTypeFilter === 'local' ? '#4a3a1a' : '#111', color: '#ffcc66', border: '1px solid #ffcc66', padding: '6px', cursor: 'pointer', fontSize: '11px' }}>LOCAL</button>
          </div>
          <label style={{ fontSize: '11px', color: '#bbb', display: 'flex', alignItems: 'center', gap: '6px' }}>
            <input type='checkbox' checked={archiveOnly} onChange={e => setArchiveOnly(e.target.checked)} />
            Только цели с архивом
          </label>
          <div style={{ color: '#777', fontSize: '10px', marginTop: '6px' }}>Показано: {filteredTargets.length} из {targets.length}</div>
        </div>
        {filteredTargets.map(t => (
          <div key={t.id} style={{ border: '1px solid #222', padding: '10px', marginBottom: '8px', position: 'relative', backgroundColor: '#0a0a0c' }}>
            <div style={{ color: t.type === 'hub' ? '#ff00ff' : '#00f0ff', fontSize: '0.9rem', paddingRight: '20px' }}>{t.name}</div>
            <div style={{ fontSize: '10px', color: '#555', marginBottom: '8px' }}>{t.host}</div>

            {t.type === 'hub' ? (
                // ОБНОВЛЕННАЯ КНОПКА ОТКРЫТИЯ FTP ИЗ НИЖНЕГО СПИСКА
                <button onClick={() => fetchFtpRoot('video1')} style={{ width: '100%', backgroundColor: '#1a4a4a', color: '#00f0ff', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                  📁 АРХИВ (FTP)
                </button>
            ) : (
                <>
                  <button onClick={() => setNemesisTarget({ host: t.host, login: t.login || 'admin', password: t.password || '', name: t.name, channels: t.channels })} style={{ width: '100%', background: 'linear-gradient(90deg, #2a0808, #0a0808)', color: '#ff003c', border: '1px solid #ff003c', padding: '6px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold', letterSpacing: '1px' }}>
                    ☢ NEMESIS ARCHIVE
                  </button>
                  <button onClick={() => handleLocalArchive(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#4a1a4a', color: '#ff9900', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                    ⏳ ЗАПРОС ПАМЯТИ
                  </button>
                  <button onClick={() => handleFetchNvrDeviceInfo(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#1a1a4a', color: '#9fc2ff', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                    ℹ ISAPI DEVICE INFO
                  </button>
                  <button onClick={() => handleSearchIsapiRecordings(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#1f2d4a', color: '#9fd7ff', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                    🔎 ISAPI SEARCH RECORDS
                  </button>
                  <button onClick={() => handleFetchOnvifDeviceInfo(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#1a3a1a', color: '#a8ffb0', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                    ℹ ONVIF DEVICE INFO
                  </button>
                  <button onClick={() => handleSearchOnvifRecordings(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#1a3a1a', color: '#b9ffcf', border: '1px solid #2a5a36', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                    🔎 ONVIF RECORDINGS
                  </button>
                  <button onClick={() => handleProbeArchiveExport(t)} style={{ width: '100%', marginTop: '6px', backgroundColor: '#3a2a1a', color: '#ffd27a', border: 'none', padding: '5px', cursor: 'pointer', fontSize: '11px', fontWeight: 'bold' }}>
                    📦 PROBE EXPORT ENDPOINTS
                  </button>
                </>
            )}

            <button onClick={() => handleDeleteTarget(t.id)} style={{ position: 'absolute', right: 8, top: 8, background: 'none', border: 'none', color: '#ff003c', cursor: 'pointer' }}>✖</button>
          </div>
        ))}

        <h3 style={{ color: '#00f0ff', marginTop: '30px', fontSize: '0.9rem' }}>LIVE-ЛОГИ ЯДРА</h3>
        <div style={{ border: '1px solid #222', background: '#050505', maxHeight: '180px', overflowY: 'auto', padding: '8px' }}>
          {runtimeLogs.length === 0 && (
            <div style={{ color: '#666', fontSize: '11px' }}>Логи пока пусты</div>
          )}
          {runtimeLogs.map((line, idx) => (
            <div key={`${idx}_${line}`} style={{ color: '#9fefff', fontSize: '10px', lineHeight: '1.4', marginBottom: '2px' }}>
              {line}
            </div>
          ))}
        </div>

        {/* =============== RELAY CONFIG =============== */}
        <div style={{ border: '1px solid #6a6aff', padding: '10px', backgroundColor: '#0a0a2a', marginTop: '20px', marginBottom: '10px' }}>
          <h3 style={{ color: '#6a6aff', marginTop: '0', fontSize: '0.9rem' }}>🔗 FTP RELAY (ПК 2)</h3>
          <div style={{ fontSize: '10px', color: '#88a', marginBottom: '6px' }}>
            Если FTP недоступен с этого ПК — запустите hyperion-relay.exe на ПК с доступом и укажите его адрес.
          </div>
          <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
            <input
              style={{ flex: 3, backgroundColor: '#000', border: '1px solid #6a6aff', color: '#6a6aff', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
              placeholder="http://192.168.1.100:8090"
              value={relayUrl}
              onChange={e => {
                setRelayUrl(e.target.value);
                try { localStorage.setItem('hyperion_relay_url', e.target.value); } catch {}
              }}
            />
            <input
              style={{ flex: 2, backgroundColor: '#000', border: '1px solid #6a6aff', color: '#6a6aff', padding: '6px', boxSizing: 'border-box', fontSize: '11px' }}
              placeholder="Token (опц.)"
              value={relayToken}
              onChange={e => {
                setRelayToken(e.target.value);
                try { localStorage.setItem('hyperion_relay_token', e.target.value); } catch {}
              }}
            />
          </div>
          <button
            onClick={async () => {
              if (!relayUrl.trim()) return toast('Введите URL relay');
              try {
                const resp = await invoke('relay_ping', {
                  relayUrl: relayUrl.trim(),
                  relayToken: relayToken.trim() || null,
                });
                setRelayStatus('ok');
                toast(`Relay доступен! Версия: ${resp.version || '?'}, uptime: ${resp.uptime_sec || 0}s`);
              } catch (err) {
                setRelayStatus('error');
                toast(`Relay недоступен: ${err}`);
              }
            }}
            style={{ width: '100%', backgroundColor: relayStatus === 'ok' ? '#1a4a1a' : relayStatus === 'error' ? '#4a1a1a' : '#1a1a4a', color: '#6a6aff', border: '1px solid #6a6aff', padding: '6px', cursor: 'pointer', fontWeight: 'bold', fontSize: '11px' }}
          >
            {relayStatus === 'ok' ? '✅ RELAY ПОДКЛЮЧЁН' : relayStatus === 'error' ? '❌ ПРОВЕРИТЬ СНОВА' : '🔗 ПРОВЕРИТЬ СВЯЗЬ'}
          </button>
          {relayUrl.trim() && (
            <div style={{ fontSize: '10px', color: '#559', marginTop: '4px' }}>
              FTP-браузер и загрузки будут идти через relay автоматически.
            </div>
          )}
        </div>

        <h3 style={{ color: '#00f0ff', marginTop: '30px', fontSize: '0.9rem' }}>МЕНЕДЖЕР ЗАГРУЗОК</h3>
        <label style={{ fontSize: '11px', color: '#bbb', display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '6px' }}>
          <input type='checkbox' checked={resumeDownloads} onChange={e => setResumeDownloads(e.target.checked)} />
          Включить докачку (resume)
        </label>
        <button
          onClick={clearFinishedDownloads}
          style={{ width: '100%', marginBottom: '6px', background: '#111', color: '#aaa', border: '1px solid #333', padding: '6px', cursor: 'pointer', fontSize: '11px' }}
        >
          ОЧИСТИТЬ ЗАВЕРШЕННЫЕ/ОШИБКИ
        </button>
        <div style={{ border: '1px solid #222', background: '#050505', maxHeight: '220px', overflowY: 'auto', padding: '8px' }}>
          {downloadTasks.length === 0 && (
            <div style={{ color: '#666', fontSize: '11px' }}>Загрузок пока нет</div>
          )}
          {downloadTasks.map((task) => (
            <div key={task.id} style={{ border: '1px solid #111', padding: '6px', marginBottom: '6px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px' }}>
                <span style={{ color: '#ddd' }}>{task.filename}</span>
                <span style={{ color: task.status === 'done' ? '#00ff9c' : task.status === 'error' ? '#ff5555' : task.status === 'cancelled' ? '#caa0ff' : '#ffcc66' }}>
                  {task.status === 'running' ? 'В ПРОЦЕССЕ' : task.status === 'done' ? 'ГОТОВО' : task.status === 'cancelled' ? 'ОТМЕНЕНО' : 'ОШИБКА'}
                </span>
              </div>
              <div style={{ fontSize: '10px', color: '#888', marginTop: '4px' }}>
                {task.serverAlias}
                {task.protocol ? `/${task.protocol}` : ''} • {formatBytes(task.bytesWritten)}
                {task.speedBytesSec > 0 ? ` • ${formatBytes(task.speedBytesSec)}/s` : ''}
                {task.resumed ? ' • RESUME' : ''}
                {task.skipped ? ' • SKIPPED' : ''}
              </div>
              {task.stageSummary && (
                <div style={{ fontSize: '10px', color: '#6da8ff', marginTop: '4px' }}>
                  {task.stageSummary}
                </div>
              )}
              {(task.finalStatus || Number.isFinite(task.retryCount) || Number.isFinite(task.stageCount)) && (
                <div style={{ fontSize: '10px', color: '#9ca8bd', marginTop: '3px' }}>
                  status={task.finalStatus || task.status} • retries={task.retryCount ?? 0} • stages={task.stageCount ?? (task.stageDetails?.length || 0)}{task.fallbackDurationSeconds ? ` • ffmpegT=${task.fallbackDurationSeconds}s` : ''}
                </div>
              )}
              {Array.isArray(task.stageDetails) && task.stageDetails.length > 0 && (
                <div style={{ marginTop: '4px', border: '1px solid #1a2238', background: '#070b14', padding: '4px 6px' }}>
                  {task.stageDetails.map((s, idx) => (
                    <div key={`${task.id}_${s.stage}_${idx}`} style={{ fontSize: '10px', color: s.success ? '#73ffb0' : '#ffb6b6', marginBottom: '2px' }}>
                      {s.success ? '✅' : '❌'} {s.stage}
                      {s.reason ? ` — ${s.reason}` : ''}
                    </div>
                  ))}
                </div>
              )}
              {task.error && (
                <div style={{ fontSize: '10px', color: '#ff9b9b', marginTop: '4px', wordBreak: 'break-word' }}>
                  {task.error}
                </div>
              )}
              <div style={{ height: '4px', background: '#111', marginTop: '6px' }}>
                <div
                  style={{
                    width: `${task.percent ?? (task.status === 'running' ? 10 : 0)}%`,
                    height: '100%',
                    background: task.status === 'error' ? '#ff5555' : '#00f0ff',
                    transition: 'width 0.3s ease',
                  }}
                />
              </div>

              {task.status === 'running' && (
                <button
                  onClick={() => handleCancelDownloadTask(task)}
                  style={{ marginTop: '6px', marginRight: '6px', background: '#2d1a4a', color: '#d8b0ff', border: '1px solid #b36bff', padding: '4px 8px', cursor: 'pointer', fontSize: '10px' }}
                >
                  ОТМЕНИТЬ
                </button>
              )}


              {task.status === 'error' && (
                <button
                  onClick={() => handleRetryDownloadTask(task)}
                  style={{ marginTop: '6px', background: '#4a1a1a', color: '#ffaaaa', border: '1px solid #ff5555', padding: '4px 8px', cursor: 'pointer', fontSize: '10px' }}
                >
                  ПОВТОРИТЬ ЗАГРУЗКУ
                </button>
              )}
            </div>
          ))}
        </div>
          </div>
        </div>
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
