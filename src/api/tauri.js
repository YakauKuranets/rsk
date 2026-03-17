import { invoke } from '@tauri-apps/api/core';

// ═══ Targets ═══
export const saveTarget = (data) => invoke('save_target', { data: JSON.stringify(data) });
export const readTarget = (id) => invoke('read_target', { id });
export const getAllTargets = () => invoke('get_all_targets');
export const deleteTarget = (id) => invoke('delete_target', { id });

// ═══ Streams ═══
export const startStream = (targetId, rtspUrl) => invoke('start_stream', { targetId, rtspUrl });
export const stopStream = (targetId) => invoke('stop_stream', { targetId });
export const checkStreamAlive = (targetId) => invoke('check_stream_alive', { targetId });
export const restartStream = (targetId, rtspUrl) => invoke('restart_stream', { targetId, rtspUrl });
export const startHubStream = (targetId, userId, channelId, cookie) =>
  invoke('start_hub_stream', { targetId, userId, channelId, cookie });

// ═══ FTP ═══
export const getFtpFolders = (serverAlias, path) => invoke('get_ftp_folders', { serverAlias, path });
export const downloadFtpFile = (serverAlias, filePath, login, password) =>
  invoke('download_ftp_file', { serverAlias, filePath, login, password });

// ═══ Search & Recon ═══
export const externalSearch = (country, city) => invoke('external_search', { country, city });
export const searchGlobalHub = (query, cookie) => invoke('search_global_hub', { query, cookie });
export const geocodeAddress = (address) => invoke('geocode_address', { address });
export const scanHostPorts = (host) => invoke('scan_host_ports', { host });
export const analyzeSecurityHeaders = (targetUrl) => invoke('analyze_security_headers', { targetUrl });

// ═══ NVR/Archive ═══
export const generateNvrChannels = (host, login, password, channelCount) =>
  invoke('generate_nvr_channels', { host, login, password, channelCount });
export const probeNvrProtocols = (host, login, pass) =>
  invoke('probe_nvr_protocols', { host, login, pass });
export const fetchNvrDeviceInfo = (host, login, pass) =>
  invoke('fetch_nvr_device_info', { host, login, pass });
export const searchIsapiRecordings = (host, login, pass, from, to) =>
  invoke('search_isapi_recordings', { host, login, pass, from, to });
export const searchOnvifRecordings = (host, login, pass) =>
  invoke('search_onvif_recordings', { host, login, pass });

// ═══ Spider ═══
export const spiderFullScan = (params) => invoke('spider_full_scan', params);
export const fuzzCctvApi = (params) => invoke('fuzz_cctv_api', params);

// ═══ Audit ═══
export const adaptiveCredentialAudit = (ip, vendor, osintContext) =>
  invoke('adaptive_credential_audit', { ip, vendor, osintContext });
export const runMassAudit = (targetIps, knownLogin, knownPass) =>
  invoke('run_mass_audit', { targetIps, knownLogin, knownPass });
export const verifyVulnerabilities = (ip, vendor) =>
  invoke('verify_vulnerabilities', { ip, vendor });
export const searchPublicExploits = (vendor) => invoke('search_public_exploits', { vendor });
export const scanNeighborhood = (ip) => invoke('scan_neighborhood', { ip });
export const collectMetadata = (ip) => invoke('collect_metadata', { ip });

// ═══ Job Runner ═══
export const startAuditJob = (target) => invoke('start_audit_job', { target });
export const startSessionJob = (target) => invoke('start_session_job', { target });
export const startFuzzerJob = (target) => invoke('start_fuzzer_job', { target });

// ═══ Logs ═══
export const getRuntimeLogs = (limit = 200) => invoke('get_runtime_logs', { limit });

// ═══ Nexus ═══
export const runNexusProtocol = (ip, login, pass) =>
  invoke('run_nexus_protocol', { ip, login, pass });

// ═══ New modules (после реализации) ═══
export const discoverExternalAssets = (params) => invoke('discover_external_assets', params);
export const advancedCredentialAudit = (params) => invoke('advanced_credential_audit', params);
export const analyzeTraffic = (params) => invoke('analyze_traffic', params);
export const verifyVulnerability = (params) => invoke('verify_vulnerability', params);
export const generateAttackGraph = (targetsJson) => invoke('generate_attack_graph', { targetsJson });
export const checkCompliance = (findingsJson, standards) =>
  invoke('check_compliance', { findingsJson, standards });
export const exportReportJson = (campaignData) => invoke('export_report_json', { campaignData });
export const updateVulnDatabase = () => invoke('update_vuln_database');
