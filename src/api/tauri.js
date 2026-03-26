import { invoke } from '@tauri-apps/api/core';
import {
  formatSurfaceScanResultV1Marker,
  normalizeSpiderFullScanResultV1,
} from './surfaceScanResultContract';
import {
  deriveSpiderFingerprintEnrichmentV1,
  formatSpiderFingerprintEnrichmentV1Marker,
} from './spiderFingerprintEnrichment';
import {
  deriveSpiderAuthBoundaryHintsV1,
  formatSpiderAuthBoundaryHintsV1Marker,
} from './spiderAuthBoundaryHints';
import {
  buildSpiderEvidenceReportV1,
  formatSpiderEvidenceCompactSummaryV1,
} from './spiderEvidenceReport';
import {
  formatPortScanResultV1Marker,
  normalizeScanHostPortsResultV1,
} from './portScanResultContract';
import {
  formatPortAuditResultV1Marker,
  normalizePortAuditFromScanResultV1,
} from './portAuditResultContract';

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
export const listActiveStreams = () => invoke('list_active_streams');
export const stopAllStreams = () => invoke('stop_all_streams');
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
export const scanHostPortsNormalized = async (host) => {
  const raw = await invoke('scan_host_ports', { host });
  const portScanResult = normalizeScanHostPortsResultV1(raw, { host });
  return {
    raw,
    portScanResult,
    marker: formatPortScanResultV1Marker(portScanResult),
  };
};
export const auditHostPortsNormalized = async (host, options = {}) => {
  const { portScanResult, raw } = await scanHostPortsNormalized(host);
  const portAuditResult = normalizePortAuditFromScanResultV1(portScanResult, options);
  return {
    raw,
    portScanResult,
    portAuditResult,
    marker: formatPortAuditResultV1Marker(portAuditResult),
  };
};
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
export const spiderFullScanNormalized = async (params = {}) => {
  const raw = await invoke('spider_full_scan', params);
  const normalized = normalizeSpiderFullScanResultV1(raw, {
    targetId: params?.targetUrl || params?.targetId || null,
  });
  const fingerprintEnrichment = deriveSpiderFingerprintEnrichmentV1(normalized, raw);
  const authBoundaryHints = deriveSpiderAuthBoundaryHintsV1(normalized, raw);
  const evidenceReport = buildSpiderEvidenceReportV1({
    surfaceScanResult: normalized,
    raw,
  });
  return {
    raw,
    surfaceScanResult: normalized,
    marker: formatSurfaceScanResultV1Marker(normalized),
    fingerprintMarker: formatSpiderFingerprintEnrichmentV1Marker(
      fingerprintEnrichment,
      normalized?.target_id || normalized?.host || params?.targetUrl || 'n/a',
    ),
    authBoundaryMarker: formatSpiderAuthBoundaryHintsV1Marker(
      authBoundaryHints,
      normalized?.target_id || normalized?.host || params?.targetUrl || 'n/a',
    ),
    evidenceReport,
    evidenceMarker: formatSpiderEvidenceCompactSummaryV1(evidenceReport),
  };
};
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
export const pushRuntimeLogEntry = (message) => invoke('push_runtime_log_entry', { message });

// ═══ Minimal agent (stable consumer contract) ═══
const AGENT_MINIMAL_FINAL_STATUS = {
  REVIEWER_REJECTED: 'reviewer_rejected',
  CAPABILITY_SUCCEEDED: 'capability_succeeded',
  CAPABILITY_FAILED: 'capability_failed',
};

const AGENT_MINIMAL_STATUS_ALIASES = {
  reviewerrejected: AGENT_MINIMAL_FINAL_STATUS.REVIEWER_REJECTED,
  reviewer_rejected: AGENT_MINIMAL_FINAL_STATUS.REVIEWER_REJECTED,
  capabilitysucceeded: AGENT_MINIMAL_FINAL_STATUS.CAPABILITY_SUCCEEDED,
  capability_succeeded: AGENT_MINIMAL_FINAL_STATUS.CAPABILITY_SUCCEEDED,
  capabilityfailed: AGENT_MINIMAL_FINAL_STATUS.CAPABILITY_FAILED,
  capability_failed: AGENT_MINIMAL_FINAL_STATUS.CAPABILITY_FAILED,
};

const normalizeStatus = (value) => {
  const key = String(value ?? '')
    .trim()
    .toLowerCase();
  return AGENT_MINIMAL_STATUS_ALIASES[key] ?? AGENT_MINIMAL_FINAL_STATUS.CAPABILITY_FAILED;
};

const ensureObject = (value) => (value && typeof value === 'object' ? value : {});

export const validateAgentMinimalEnvelope = (raw) => {
  const envelope = ensureObject(raw);
  const errors = [];

  if (!envelope.agentRunId || typeof envelope.agentRunId !== 'string') {
    errors.push('agentRunId is missing');
  }
  if (!envelope.targetId || typeof envelope.targetId !== 'string') {
    errors.push('targetId is missing');
  }
  if (!envelope.mode || typeof envelope.mode !== 'string') {
    errors.push('mode is missing');
  }
  if (typeof envelope.reporterSummary !== 'string') {
    errors.push('reporterSummary is missing');
  }

  return {
    ok: errors.length === 0,
    errors,
    envelope,
  };
};

export const normalizeAgentMinimalResult = (raw) => {
  const { ok, errors, envelope } = validateAgentMinimalEnvelope(raw);
  const plannerDecision = ensureObject(envelope.plannerDecision);
  const reviewerVerdict = ensureObject(envelope.reviewerVerdict);
  const capabilityResultSummary = ensureObject(envelope.capabilityResultSummary);
  const capabilityArgsSummary = ensureObject(envelope.capabilityArgsSummary);
  const evidenceRefs = Array.isArray(envelope.evidenceRefs) ? envelope.evidenceRefs : [];
  const normalizedFinalStatus = normalizeStatus(envelope.finalStatus);

  return {
    ok,
    errors,
    runId: envelope.agentRunId ?? null,
    targetId: envelope.targetId ?? null,
    mode: envelope.mode ?? null,
    finalStatus: normalizedFinalStatus,
    plannerDecision: {
      actionCount: Number(plannerDecision.actionCount ?? 0),
      primaryCapability: plannerDecision.primaryCapability ?? null,
      rationale: plannerDecision.rationale ?? null,
      confidence:
        typeof plannerDecision.confidence === 'number' ? plannerDecision.confidence : null,
    },
    reviewerVerdict: {
      approved: Boolean(reviewerVerdict.approved),
      reasons: Array.isArray(reviewerVerdict.reasons) ? reviewerVerdict.reasons : [],
    },
    capabilityInvoked: envelope.capabilityInvoked ?? null,
    capabilityArgsSummary: {
      targetId: capabilityArgsSummary.targetId ?? null,
      ipOrUrl: capabilityArgsSummary.ipOrUrl ?? null,
    },
    capabilityResultSummary: {
      ok: Boolean(capabilityResultSummary.ok),
      alive:
        typeof capabilityResultSummary.alive === 'boolean' ? capabilityResultSummary.alive : null,
      secure:
        typeof capabilityResultSummary.secure === 'boolean'
          ? capabilityResultSummary.secure
          : null,
      issuesCount:
        typeof capabilityResultSummary.issuesCount === 'number'
          ? capabilityResultSummary.issuesCount
          : null,
      errorCode: capabilityResultSummary.errorCode ?? null,
      errorMessage: capabilityResultSummary.errorMessage ?? null,
    },
    evidenceRefs,
    reporterSummary: envelope.reporterSummary ?? '',
    raw: envelope,
  };
};

export const runAgentMinimal = async ({
  targetId,
  mode,
  permitProbeStream = true,
  permitVerifySessionCookieFlags = false,
  preferredCapability = null,
  verifySessionCookieFlagsIpOrUrl = null,
}) => {
  const response = await invoke('run_agent_minimal', {
    req: {
      planner: {
        targetId,
        mode,
        preferredCapability,
        verifySessionCookieFlags: verifySessionCookieFlagsIpOrUrl
          ? { ipOrUrl: verifySessionCookieFlagsIpOrUrl }
          : undefined,
      },
      permitProbeStream,
      permitVerifySessionCookieFlags,
    },
  });

  return normalizeAgentMinimalResult(response);
};

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
