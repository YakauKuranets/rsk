import React from 'react';
import { invoke } from '@tauri-apps/api/core';
import { SPIDER_MODULES_CONFIG, useAppStore } from '../../store/appStore';
import { toast } from '../../utils/toast';

export default function SpiderControl({ handleStartNemesis, handleAnalyzeSources, handlePlayFuzzedLink }) {
  const spiderTarget = useAppStore((s) => s.spiderTarget);
  const setSpiderTarget = useAppStore((s) => s.setSpiderTarget);
  const spiderMaxDepth = useAppStore((s) => s.spiderMaxDepth);
  const setSpiderMaxDepth = useAppStore((s) => s.setSpiderMaxDepth);
  const spiderMaxPages = useAppStore((s) => s.spiderMaxPages);
  const setSpiderMaxPages = useAppStore((s) => s.setSpiderMaxPages);
  const spiderDirBrute = useAppStore((s) => s.spiderDirBrute);
  const setSpiderDirBrute = useAppStore((s) => s.setSpiderDirBrute);
  const spiderEnableVulnVerification = useAppStore((s) => s.spiderEnableVulnVerification);
  const setSpiderEnableVulnVerification = useAppStore((s) => s.setSpiderEnableVulnVerification);
  const spiderEnableVideoStreamAnalyzer = useAppStore((s) => s.spiderEnableVideoStreamAnalyzer);
  const setSpiderEnableVideoStreamAnalyzer = useAppStore((s) => s.setSpiderEnableVideoStreamAnalyzer);
  const spiderEnableCredentialDepthAudit = useAppStore((s) => s.spiderEnableCredentialDepthAudit);
  const setSpiderEnableCredentialDepthAudit = useAppStore((s) => s.setSpiderEnableCredentialDepthAudit);
  const spiderEnablePassiveArpDiscovery = useAppStore((s) => s.spiderEnablePassiveArpDiscovery);
  const setSpiderEnablePassiveArpDiscovery = useAppStore((s) => s.setSpiderEnablePassiveArpDiscovery);
  const spiderEnableOsintImport = useAppStore((s) => s.spiderEnableOsintImport);
  const setSpiderEnableOsintImport = useAppStore((s) => s.setSpiderEnableOsintImport);
  const spiderEnableTopologyDiscovery = useAppStore((s) => s.spiderEnableTopologyDiscovery);
  const setSpiderEnableTopologyDiscovery = useAppStore((s) => s.setSpiderEnableTopologyDiscovery);
  const spiderEnableThreatIntel = useAppStore((s) => s.spiderEnableThreatIntel);
  const setSpiderEnableThreatIntel = useAppStore((s) => s.setSpiderEnableThreatIntel);
  const spiderEnableSnapshotRefresh = useAppStore((s) => s.spiderEnableSnapshotRefresh);
  const setSpiderEnableSnapshotRefresh = useAppStore((s) => s.setSpiderEnableSnapshotRefresh);
  const spiderEnableUptimeMonitoring = useAppStore((s) => s.spiderEnableUptimeMonitoring);
  const setSpiderEnableUptimeMonitoring = useAppStore((s) => s.setSpiderEnableUptimeMonitoring);
  const spiderEnableNeighborDiscovery = useAppStore((s) => s.spiderEnableNeighborDiscovery);
  const setSpiderEnableNeighborDiscovery = useAppStore((s) => s.setSpiderEnableNeighborDiscovery);
  const spiderEnableScheduledAudits = useAppStore((s) => s.spiderEnableScheduledAudits);
  const setSpiderEnableScheduledAudits = useAppStore((s) => s.setSpiderEnableScheduledAudits);
  const spiderRunning = useAppStore((s) => s.spiderRunning);
  const setSpiderRunning = useAppStore((s) => s.setSpiderRunning);
  const spiderReport = useAppStore((s) => s.spiderReport);
  const setSpiderReport = useAppStore((s) => s.setSpiderReport);
  const spiderTab = useAppStore((s) => s.spiderTab);
  const setSpiderTab = useAppStore((s) => s.setSpiderTab);
  const targetInput = useAppStore((s) => s.targetInput);
  const setTargetInput = useAppStore((s) => s.setTargetInput);
  const attackType = useAppStore((s) => s.attackType);
  const setAttackType = useAppStore((s) => s.setAttackType);
  const fuzzResults = useAppStore((s) => s.fuzzResults);
  const fuzzLogin = useAppStore((s) => s.fuzzLogin);
  const setFuzzLogin = useAppStore((s) => s.setFuzzLogin);
  const fuzzPassword = useAppStore((s) => s.fuzzPassword);
  const setFuzzPassword = useAppStore((s) => s.setFuzzPassword);
  const fuzzPath = useAppStore((s) => s.fuzzPath);
  const setFuzzPath = useAppStore((s) => s.setFuzzPath);
  const sourceAnalysis = useAppStore((s) => s.sourceAnalysis);
  const hubCookie = useAppStore((s) => s.hubCookie);

  const spiderModuleStateMap = {
    dir_bruteforce: [spiderDirBrute, setSpiderDirBrute],
    enable_vuln_verification: [spiderEnableVulnVerification, setSpiderEnableVulnVerification],
    enable_video_stream_analyzer: [spiderEnableVideoStreamAnalyzer, setSpiderEnableVideoStreamAnalyzer],
    enable_passive_arp_discovery: [spiderEnablePassiveArpDiscovery, setSpiderEnablePassiveArpDiscovery],
    enable_credential_depth_audit: [spiderEnableCredentialDepthAudit, setSpiderEnableCredentialDepthAudit],
    enable_open_share_scanner: [spiderDirBrute, setSpiderDirBrute],
    enable_osint_import: [spiderEnableOsintImport, setSpiderEnableOsintImport],
    enable_topology_discovery: [spiderEnableTopologyDiscovery, setSpiderEnableTopologyDiscovery],
    enable_threat_intel: [spiderEnableThreatIntel, setSpiderEnableThreatIntel],
  };

  const formatBytes = (bytes) => {
    if (!bytes) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB'];
    let value = bytes;
    let idx = 0;
    while (value >= 1024 && idx < units.length - 1) {
      value /= 1024;
      idx += 1;
    }
    return `${value.toFixed(idx === 0 ? 0 : 1)} ${units[idx]}`;
  };
  return (
    <>
        {/* =============== 🕷️ SPIDER — УЛЬТИМАТИВНЫЙ ПАУК =============== */}
        <div style={{ border: '1px solid #b366ff', padding: '10px', backgroundColor: '#150030', marginBottom: '20px', boxShadow: '0 0 15px rgba(179,102,255,0.2)' }}>
          <h3 style={{ color: '#b366ff', marginTop: '0', fontSize: '0.9rem' }}>🕷️ HYPERION SPIDER</h3>
          <div style={{ fontSize: '10px', color: '#8855cc', marginBottom: '8px' }}>
            Глубокий обход сайта: crawler + JS parser + dir bruteforce + tech fingerprint
          </div>

          <input
            style={{ width: '100%', backgroundColor: '#000', border: '1px solid #b366ff', color: '#b366ff', padding: '6px', marginBottom: '6px', boxSizing: 'border-box', fontSize: '11px' }}
            placeholder="https://target/ или 10.0.0.0/24"
            value={spiderTarget}
            onChange={e => setSpiderTarget(e.target.value)}
          />
          <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
            <input type="number" style={{ flex: 1, backgroundColor: '#000', border: '1px solid #b366ff', color: '#b366ff', padding: '6px', fontSize: '11px' }}
              placeholder="Глубина" value={spiderMaxDepth} onChange={e => setSpiderMaxDepth(parseInt(e.target.value) || 3)} />
            <input type="number" style={{ flex: 1, backgroundColor: '#000', border: '1px solid #b366ff', color: '#b366ff', padding: '6px', fontSize: '11px' }}
              placeholder="Макс страниц" value={spiderMaxPages} onChange={e => setSpiderMaxPages(parseInt(e.target.value) || 50)} />
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr', gap: '4px', marginBottom: '6px', fontSize: '9px', color: '#b694df' }}>
            {SPIDER_MODULES_CONFIG.map((module) => {
              const [enabled, setEnabled] = spiderModuleStateMap[module.id] || [false, () => {}];
              return (
                <label key={module.id} style={{ border: '1px solid #3a1d58', backgroundColor: '#120024', padding: '5px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '5px', color: '#d2b7ff', fontSize: '10px', fontWeight: 'bold' }}>
                    <input type="checkbox" checked={enabled} onChange={e => setEnabled(e.target.checked)} />
                    {module.title}
                  </div>
                  <div style={{ color: '#9b7bc6', marginTop: '3px', lineHeight: 1.3 }}>{module.desc}</div>
                </label>
              );
            })}
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '4px', marginBottom: '6px', fontSize: '9px', color: '#b694df' }}>
            <label><input type="checkbox" checked={spiderEnableSnapshotRefresh} onChange={e => setSpiderEnableSnapshotRefresh(e.target.checked)} /> Snapshot Refresh</label>
            <label><input type="checkbox" checked={spiderEnableUptimeMonitoring} onChange={e => setSpiderEnableUptimeMonitoring(e.target.checked)} /> Uptime</label>
            <label><input type="checkbox" checked={spiderEnableNeighborDiscovery} onChange={e => setSpiderEnableNeighborDiscovery(e.target.checked)} /> Neighbors</label>
            <label><input type="checkbox" checked={spiderEnableScheduledAudits} onChange={e => setSpiderEnableScheduledAudits(e.target.checked)} /> Scheduled Audits</label>
          </div>

          <button
            disabled={spiderRunning}
            onClick={async () => {
              if (!spiderTarget.trim()) return toast('Введите URL цели');
              setSpiderRunning(true);
              setSpiderReport(null);
              try {
                const report = await invoke('spider_full_scan', {
                  targetUrl: spiderTarget.trim(),
                  cookie: hubCookie || null,
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
              } catch (err) {
                toast(`Spider error: ${err}`);
              } finally {
                setSpiderRunning(false);
              }
            }}
            style={{ width: '100%', backgroundColor: spiderRunning ? '#333' : '#b366ff', color: '#000', border: 'none', padding: '8px', cursor: spiderRunning ? 'wait' : 'pointer', fontWeight: 'bold', fontSize: '11px', letterSpacing: '1px' }}
          >
            {spiderRunning ? '⏳ ПАУК РАБОТАЕТ...' : '🕷️ ЗАПУСТИТЬ ПОЛНОЕ СКАНИРОВАНИЕ'}
          </button>

          {spiderReport && (
            <div style={{ marginTop: '10px' }}>
              <div style={{ color: '#b366ff', fontSize: '10px', marginBottom: '6px' }}>
                ✅ {spiderReport.pagesCrawled} страниц | {spiderReport.jsEndpoints?.length || 0} JS endpoints | {spiderReport.dirResults?.filter(d => d.statusCode !== 404).length || 0} dirs | {spiderReport.techStack?.length || 0} tech | {spiderReport.durationSec}s
              </div>
              <div style={{ fontSize: '9px', color: '#666', marginBottom: '6px' }}>HTML сохранён: {spiderReport.savedHtmlDir}</div>

              {spiderReport.targetCard && (
                <div style={{ border: '1px solid #663399', background: '#10001f', padding: '6px', marginBottom: '6px', fontSize: '10px' }}>
                  <div style={{ color: '#d8b7ff', fontWeight: 'bold', marginBottom: '4px' }}>🎯 TARGET CARD</div>
                  <div style={{ color: '#b8a0d8' }}>[ IP/HOST: {spiderReport.targetCard.host} ]</div>
                  <div style={{ color: '#b8a0d8' }}>[ ВЕНДОР: {spiderReport.targetCard.vendorGuess} ]</div>
                  <div style={{ color: '#b8a0d8' }}>[ API: {spiderReport.targetCard.apiGuess} ]</div>
                  <div style={{ color: '#b8a0d8' }}>[ RTSP: {spiderReport.targetCard.rtspStatus} ]</div>
                  <div style={{ color: '#9f82c5', marginTop: '4px' }}>
                    [ ПОРТЫ: {(spiderReport.targetCard.openPorts || []).map(p => `${p.port} (${p.service})`).join(', ') || 'не обнаружены'} ]
                  </div>
                </div>
              )}

              {spiderReport.discoveredTargets?.length > 0 && (
                <div style={{ border: '1px solid #4a2f6c', background: '#0d0217', padding: '6px', marginBottom: '6px', fontSize: '9px', maxHeight: '120px', overflowY: 'auto' }}>
                  <div style={{ color: '#b694df', fontWeight: 'bold', marginBottom: '4px' }}>📡 SWEEP RESULTS ({spiderReport.discoveredTargets.length})</div>
                  {spiderReport.discoveredTargets.map((t, i) => (
                    <div key={`${t.host}_${i}`} style={{ color: '#a989d1', marginBottom: '3px' }}>
                      {t.host} → {(t.openPorts || []).map(p => p.port).join(', ')}
                    </div>
                  ))}
                </div>
              )}

              {spiderReport.moduleStatuses?.length > 0 && (
                <div style={{ border: '1px solid #3a2755', background: '#0b0314', padding: '6px', marginBottom: '6px', fontSize: '9px', maxHeight: '120px', overflowY: 'auto' }}>
                  <div style={{ color: '#c19cff', fontWeight: 'bold', marginBottom: '4px' }}>🧪 AUDIT MODULES</div>
                  {spiderReport.moduleStatuses.map((m, i) => (
                    <div key={`${m.module}_${i}`} style={{ color: '#ac90d5', marginBottom: '3px' }}>
                      {m.module}: {m.status} — {m.details}
                    </div>
                  ))}
                </div>
              )}

              {spiderReport.videoStreamInfo?.length > 0 && (
                <div style={{ border: '1px solid #294a52', background: '#041014', padding: '6px', marginBottom: '6px', fontSize: '9px' }}>
                  <div style={{ color: '#7fd7e8', fontWeight: 'bold', marginBottom: '4px' }}>🎥 VIDEO STREAM INFO</div>
                  {spiderReport.videoStreamInfo.map((v, i) => (
                    <div key={`${v.host}_${i}`} style={{ color: '#8ecad6', marginBottom: '3px' }}>
                      {v.host}: {v.status} | {v.codec} | {v.resolution} | fps={v.fps} | br={v.bitrate}
                    </div>
                  ))}
                </div>
              )}

              {spiderReport.passiveDevices?.length > 0 && (
                <div style={{ border: '1px solid #355126', background: '#0b1406', padding: '6px', marginBottom: '6px', fontSize: '9px', maxHeight: '100px', overflowY: 'auto' }}>
                  <div style={{ color: '#a3d58a', fontWeight: 'bold', marginBottom: '4px' }}>📡 PASSIVE DEVICES</div>
                  {spiderReport.passiveDevices.map((d, i) => (
                    <div key={`${d.ip}_${i}`} style={{ color: '#95c27e', marginBottom: '2px' }}>{d.ip} — {d.mac}</div>
                  ))}
                </div>
              )}

              {spiderReport.threatLinks?.length > 0 && (
                <div style={{ border: '1px solid #5c3b1e', background: '#1a0f05', padding: '6px', marginBottom: '6px', fontSize: '9px' }}>
                  <div style={{ color: '#ffc27a', fontWeight: 'bold', marginBottom: '4px' }}>⚠️ THREAT INTEL</div>
                  {spiderReport.threatLinks.map((t, i) => (
                    <div key={`${t.cve}_${i}`} style={{ color: '#e6b17a', marginBottom: '2px' }}>{t.cve}: {t.title} ({t.url})</div>
                  ))}
                </div>
              )}

              {/* Вкладки */}
              <div style={{ display: 'flex', gap: '2px', marginBottom: '6px' }}>
                {[['pages', '📄'], ['js', '📜 JS'], ['dirs', '📁 DIRS'], ['tech', '🔧 TECH'], ['sitemap', '🗺️']].map(([key, label]) => (
                  <button key={key} onClick={() => setSpiderTab(key)}
                    style={{ flex: 1, padding: '4px', fontSize: '9px', fontWeight: 'bold', cursor: 'pointer',
                      backgroundColor: spiderTab === key ? '#b366ff' : '#1a0030',
                      color: spiderTab === key ? '#000' : '#b366ff',
                      border: '1px solid #b366ff' }}>
                    {label}
                  </button>
                ))}
              </div>

              <div style={{ border: '1px solid #b366ff', background: '#0a0015', maxHeight: '300px', overflowY: 'auto', padding: '6px' }}>
                {/* PAGES */}
                {spiderTab === 'pages' && spiderReport.pages?.map((p, i) => (
                  <div key={i} style={{ borderBottom: '1px solid #1a0030', padding: '4px 0', fontSize: '9px' }}>
                    <div style={{ color: p.statusCode === 200 ? '#b366ff' : '#ff5555' }}>
                      [{p.statusCode}] {p.title || '(no title)'}
                    </div>
                    <div style={{ color: '#555', wordBreak: 'break-all' }}>{p.url}</div>
                    <div style={{ color: '#444' }}>{p.contentType} | {formatBytes(p.contentLength)} | {p.linksFound} links | depth {p.depth}</div>
                  </div>
                ))}

                {/* JS ENDPOINTS */}
                {spiderTab === 'js' && spiderReport.jsEndpoints?.map((e, i) => (
                  <div key={i} style={{ borderBottom: '1px solid #1a0030', padding: '4px 0', fontSize: '9px' }}>
                    <div style={{ color: '#ff9900', fontWeight: 'bold' }}>[{e.method}] {e.endpoint}</div>
                    <div style={{ color: '#555' }}>📜 {e.sourceScript?.split('/').pop()}</div>
                    <div style={{ color: '#333', fontSize: '8px' }}>{e.context}</div>
                  </div>
                ))}

                {/* DIR RESULTS */}
                {spiderTab === 'dirs' && spiderReport.dirResults?.filter(d => d.statusCode !== 404).map((d, i) => (
                  <div key={i} style={{ borderBottom: '1px solid #1a0030', padding: '4px 0', fontSize: '9px' }}>
                    <div style={{ color: d.statusCode === 200 ? '#00ff9c' : d.statusCode === 403 ? '#ff9900' : '#888' }}>
                      {d.verdict}
                    </div>
                    <div style={{ color: '#b366ff' }}>{d.path}</div>
                    <div style={{ color: '#444' }}>{d.contentType} | {formatBytes(d.contentLength)}</div>
                  </div>
                ))}

                {/* TECH STACK */}
                {spiderTab === 'tech' && spiderReport.techStack?.map((t, i) => (
                  <div key={i} style={{ borderBottom: '1px solid #1a0030', padding: '4px 0', fontSize: '10px' }}>
                    <span style={{ color: '#b366ff', fontWeight: 'bold' }}>{t.key}: </span>
                    <span style={{ color: '#ddd' }}>{t.value}</span>
                    <span style={{ color: '#555', fontSize: '9px' }}> ({t.source})</span>
                  </div>
                ))}

                {/* SITEMAP */}
                {spiderTab === 'sitemap' && spiderReport.sitemap?.map((url, i) => (
                  <div key={i} style={{ fontSize: '9px', color: '#b366ff', padding: '2px 0', wordBreak: 'break-all' }}>
                    {url}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* 🔥 НОВЫЙ БЛОК: NEMESIS FUZZER 🔥 */}
        <div style={{ border: '1px solid #ffaa00', padding: '10px', backgroundColor: '#1a1100', marginBottom: '20px', boxShadow: '0 0 10px rgba(255, 170, 0, 0.2)' }}>
          <h3 style={{ color: '#ffaa00', marginTop: '0', fontSize: '0.9rem' }}>🔥 NEMESIS: ВЗЛОМ АРХИВА (FUZZER)</h3>

          <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
            <input
              id="nemesis-target-input"
              style={{ flex: 1, backgroundColor: '#000', border: '1px solid #aa3333', color: '#ff6666', padding: '6px', boxSizing: 'border-box' }}
              placeholder="TARGET URL (e.g. 93.125.2.167:2019/Streaming/Channels/101)"
              value={targetInput}
              onChange={e => setTargetInput(e.target.value)}
            />
            <select
              style={{ width: '240px', backgroundColor: '#000', border: '1px solid #aa3333', color: '#ff6666', padding: '6px', boxSizing: 'border-box' }}
              value={attackType}
              onChange={e => setAttackType(e.target.value)}
            >
              <option value="RTSP_BRUTE">RTSP_BRUTE</option>
              <option value="CGI_EXPLOIT">CGI_EXPLOIT</option>
              <option value="CUSTOM_INJECT">CUSTOM_INJECT</option>
            </select>
            <button
              onClick={handleStartNemesis}
              style={{ backgroundColor: '#2a0000', border: '1px solid #ff5555', color: '#ff6666', padding: '6px 10px', cursor: 'pointer', fontSize: '11px', letterSpacing: '1px', fontWeight: 'bold' }}
            >
              EXECUTE
            </button>
          </div>

          <div style={{ display: 'flex', gap: '6px', marginBottom: '6px' }}>
            <input
              style={{ flex: 1, backgroundColor: '#000', border: '1px solid #ffaa00', color: '#ffaa00', padding: '6px', boxSizing: 'border-box' }}
              placeholder="Логин (admin)"
              value={fuzzLogin}
              onChange={e => setFuzzLogin(e.target.value)}
            />
            <input
              style={{ flex: 1, backgroundColor: '#000', border: '1px solid #ffaa00', color: '#ffaa00', padding: '6px', boxSizing: 'border-box' }}
              type="password"
              placeholder="Пароль"
              value={fuzzPassword}
              onChange={e => setFuzzPassword(e.target.value)}
            />
          </div>

          <textarea
            style={{ width: '100%', backgroundColor: '#000', border: '1px solid #ffaa00', color: '#ffaa00', padding: '6px', marginBottom: '8px', boxSizing: 'border-box', height: '50px', fontSize: '10px', resize: 'none' }}
            placeholder="Целевой путь: archive/path/file.mkv"
            value={fuzzPath}
            onChange={e => setFuzzPath(e.target.value)}
          />

          <button
            onClick={handleStartNemesis}
            style={{ width: '100%', backgroundColor: '#ffaa00', color: '#000', border: 'none', padding: '8px', cursor: 'pointer', fontWeight: 'bold', letterSpacing: '1px' }}>
            ☢ RUN LEGACY FLOW
          </button>

          {fuzzResults.length > 0 && (
            <div style={{ marginTop: '10px', border: '1px solid #ffaa00', background: '#050505', maxHeight: '150px', overflowY: 'auto', padding: '6px' }}>
              {fuzzResults.map((res, idx) => {
                const isPlayable = res.includes('[200]') || res.includes('[401]') || res.includes('УСПЕХ') || res.includes('НАЙДЕНО');
                const hasUrl = /(http|rtsp):\/\/[^\s]+/.test(res);
                return (
                  <div key={idx} style={{ fontSize: '10px', color: isPlayable ? '#00ff9c' : '#ffcc00', marginBottom: '4px', wordBreak: 'break-all', display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                    <span style={{ flex: 1 }}>{res}</span>
                    {isPlayable && hasUrl && (
                      <button
                        onClick={() => handlePlayFuzzedLink(res)}
                        style={{ marginLeft: '10px', background: '#1a4a4a', color: '#00f0ff', border: '1px solid #00f0ff', padding: '2px 8px', cursor: 'pointer', fontSize: '9px', fontWeight: 'bold', flexShrink: 0 }}
                      >
                        ▶ ПЛЕЙ
                      </button>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
        {/* ============================== */}

        <button
            onClick={handleAnalyzeSources}
            style={{ width: '100%', marginTop: '8px', backgroundColor: '#1a4a4a', color: '#00f0ff', border: '1px solid #00f0ff', padding: '8px', cursor: 'pointer', fontWeight: 'bold' }}>
            🕷️ ПРОЧИТАТЬ ИСХОДНЫЙ КОД (НАЙТИ API)
          </button>

          {sourceAnalysis && (
            <div style={{ marginTop: '10px', border: '1px solid #00f0ff', background: '#001111', maxHeight: '200px', overflowY: 'auto', padding: '6px' }}>
              <div style={{ color: '#ffcc00', fontSize: '10px', fontWeight: 'bold' }}>НАЙДЕННЫЕ ФОРМЫ (ACTION):</div>
              {sourceAnalysis.forms.map((f, i) => <div key={'f'+i} style={{ color: '#00f0ff', fontSize: '10px' }}>➡ {f}</div>)}

              <div style={{ color: '#ffcc00', fontSize: '10px', fontWeight: 'bold', marginTop: '6px' }}>СКРЫТЫЕ AJAX / API:</div>
              {sourceAnalysis.apiEndpoints.map((a, i) => <div key={'a'+i} style={{ color: '#ff003c', fontSize: '10px' }}>⚡ {a}</div>)}

              <div style={{ color: '#ffcc00', fontSize: '10px', fontWeight: 'bold', marginTop: '6px' }}>ПАРАМЕТРЫ ФОРМ (INPUTS):</div>
              <div style={{ color: '#aaa', fontSize: '10px' }}>{sourceAnalysis.inputs.join(', ') || 'нет'}</div>
            </div>
          )}

    </>
  );
}
