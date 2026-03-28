import {
  PORT_SCAN_RESULT_CONTRACT_VERSION,
  normalizePortScanResultV1,
} from './portScanResultContract';

export const PORT_AUDIT_RESULT_CONTRACT_VERSION = 'port_audit_result_v1';

const DEFAULT_PORT_AUDIT_RESULT = {
  contractVersion: PORT_AUDIT_RESULT_CONTRACT_VERSION,
  target_id: null,
  audited_ports: [],
  unexpected_open_ports: [],
  sensitive_ports_exposed: [],
  auth_boundary_findings: [],
  plaintext_service_detected: false,
  legacy_service_detected: false,
  risk_level: 'unknown',
  issues: [],
  issuesCount: 0,
  recommendations: [],
  evidenceRefs: [],
  confidence: 0,
  resultClass: 'inconclusive',
};

const RISK_LEVELS = ['low', 'medium', 'high', 'critical', 'unknown'];

const DEFAULT_EXPECTED_OPEN_PORTS = [80, 443, 554];
const SENSITIVE_PORTS = [21, 22, 23, 445, 3389, 5900, 2323];
const PLAINTEXT_SERVICE_KEYWORDS = ['telnet', 'ftp', 'http', 'rtsp'];
const LEGACY_SERVICE_KEYWORDS = ['telnet', 'netbios', 'smb1', 'snmpv1'];

function unique(items = []) {
  return [...new Set(items)];
}

function toStringArray(value) {
  if (!Array.isArray(value)) return [];
  return value.map((x) => String(x ?? '').trim()).filter(Boolean);
}

function normalizePorts(value) {
  return unique((Array.isArray(value) ? value : [])
    .map((x) => Number(x))
    .filter((n) => Number.isFinite(n) && n > 0 && n <= 65535)
    .map((n) => Math.trunc(n))).sort((a, b) => a - b);
}

function clampConfidence(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) return 0;
  return Math.max(0, Math.min(1, Number(n.toFixed(4))));
}

function safeClass(value) {
  return ['passed', 'failed', 'inconclusive'].includes(value) ? value : 'inconclusive';
}

function safeRisk(value) {
  return RISK_LEVELS.includes(value) ? value : 'unknown';
}

function inferRiskLevel({
  unexpectedOpenPorts,
  sensitivePortsExposed,
  plaintextServiceDetected,
  legacyServiceDetected,
  reachable,
}) {
  if (!reachable) return 'unknown';

  if ((sensitivePortsExposed.length > 0 && plaintextServiceDetected) || sensitivePortsExposed.includes(23)) {
    return 'critical';
  }
  if (sensitivePortsExposed.length > 0 || legacyServiceDetected || unexpectedOpenPorts.length >= 3) {
    return 'high';
  }
  if (unexpectedOpenPorts.length > 0 || plaintextServiceDetected) {
    return 'medium';
  }
  return 'low';
}

function buildRecommendations({ issues = [], riskLevel = 'unknown' }) {
  const rec = [];
  if (issues.includes('sensitive_ports_exposed')) {
    rec.push('Restrict sensitive ports to trusted management networks.');
  }
  if (issues.includes('plaintext_service_detected')) {
    rec.push('Replace plaintext services with encrypted alternatives where possible.');
  }
  if (issues.includes('legacy_service_detected')) {
    rec.push('Disable legacy protocol versions and enforce modern protocol stacks.');
  }
  if (issues.includes('unexpected_open_ports_detected')) {
    rec.push('Review open-port allowlist and close non-required services.');
  }
  if (riskLevel === 'unknown') {
    rec.push('Re-run controlled scan to improve signal quality before final judgment.');
  }
  return unique(rec).slice(0, 6);
}

export function normalizePortAuditResultV1(input = {}) {
  const issues = toStringArray(input?.issues);
  const recommendations = toStringArray(input?.recommendations);

  return {
    ...DEFAULT_PORT_AUDIT_RESULT,
    ...input,
    contractVersion: PORT_AUDIT_RESULT_CONTRACT_VERSION,
    target_id: input?.target_id ? String(input.target_id) : null,
    audited_ports: normalizePorts(input?.audited_ports),
    unexpected_open_ports: normalizePorts(input?.unexpected_open_ports),
    sensitive_ports_exposed: normalizePorts(input?.sensitive_ports_exposed),
    auth_boundary_findings: toStringArray(input?.auth_boundary_findings),
    plaintext_service_detected: Boolean(input?.plaintext_service_detected),
    legacy_service_detected: Boolean(input?.legacy_service_detected),
    risk_level: safeRisk(input?.risk_level),
    issues,
    issuesCount: issues.length,
    recommendations,
    evidenceRefs: toStringArray(input?.evidenceRefs),
    confidence: clampConfidence(input?.confidence),
    resultClass: safeClass(input?.resultClass),
  };
}

export function normalizePortAuditFromScanResultV1(scanResult, options = {}) {
  const scan = normalizePortScanResultV1(scanResult || {});
  const expectedOpenPorts = normalizePorts(options?.expectedOpenPorts || DEFAULT_EXPECTED_OPEN_PORTS);
  const auditedPorts = normalizePorts(scan.open_ports);
  const services = toStringArray(scan.services).map((s) => s.toLowerCase());

  const unexpectedOpenPorts = auditedPorts.filter((p) => !expectedOpenPorts.includes(p));
  const sensitivePortsExposed = auditedPorts.filter((p) => SENSITIVE_PORTS.includes(p));

  const plaintextServiceDetected = services.some((s) => PLAINTEXT_SERVICE_KEYWORDS.some((k) => s.includes(k)));
  const legacyServiceDetected = services.some((s) => LEGACY_SERVICE_KEYWORDS.some((k) => s.includes(k)));

  const authBoundaryFindings = unique([
    sensitivePortsExposed.length > 0 ? 'sensitive_surface_exposed' : null,
    plaintextServiceDetected ? 'plaintext_service_detected' : null,
    legacyServiceDetected ? 'legacy_service_detected' : null,
    unexpectedOpenPorts.length > 0 ? 'unexpected_open_ports_detected' : null,
  ].filter(Boolean));

  const issues = unique([
    sensitivePortsExposed.length > 0 ? 'sensitive_ports_exposed' : null,
    plaintextServiceDetected ? 'plaintext_service_detected' : null,
    legacyServiceDetected ? 'legacy_service_detected' : null,
    unexpectedOpenPorts.length > 0 ? 'unexpected_open_ports_detected' : null,
  ].filter(Boolean));

  const riskLevel = inferRiskLevel({
    unexpectedOpenPorts,
    sensitivePortsExposed,
    plaintextServiceDetected,
    legacyServiceDetected,
    reachable: scan.reachable,
  });

  const recommendations = buildRecommendations({ issues, riskLevel });

  const resultClass = riskLevel === 'critical' || riskLevel === 'high'
    ? 'failed'
    : riskLevel === 'medium' || riskLevel === 'unknown'
      ? 'inconclusive'
      : 'passed';

  const confidence = !scan.reachable
    ? 0.25
    : Math.max(0.3, Math.min(0.92, Number(scan.confidence || 0) * 0.9 + (auditedPorts.length > 0 ? 0.1 : 0)));

  return normalizePortAuditResultV1({
    target_id: scan.target_id || scan.host || null,
    audited_ports: auditedPorts,
    unexpected_open_ports: unexpectedOpenPorts,
    sensitive_ports_exposed: sensitivePortsExposed,
    auth_boundary_findings: authBoundaryFindings,
    plaintext_service_detected: plaintextServiceDetected,
    legacy_service_detected: legacyServiceDetected,
    risk_level: riskLevel,
    issues,
    recommendations,
    evidenceRefs: [
      `source_contract:${PORT_SCAN_RESULT_CONTRACT_VERSION}`,
      ...toStringArray(scan.evidenceRefs),
      `audited_ports:${auditedPorts.length}`,
      `expected_ports:${expectedOpenPorts.join(',') || 'none'}`,
    ],
    confidence,
    resultClass,
  });
}

export function validatePortAuditResultV1Shape(input = {}) {
  const requiredKeys = [
    'contractVersion',
    'target_id',
    'audited_ports',
    'unexpected_open_ports',
    'sensitive_ports_exposed',
    'auth_boundary_findings',
    'plaintext_service_detected',
    'legacy_service_detected',
    'risk_level',
    'issues',
    'issuesCount',
    'recommendations',
    'evidenceRefs',
    'confidence',
    'resultClass',
  ];

  const missingKeys = requiredKeys.filter((k) => !Object.prototype.hasOwnProperty.call(input, k));
  const portsOk = [input?.audited_ports, input?.unexpected_open_ports, input?.sensitive_ports_exposed]
    .every((arr) => Array.isArray(arr) && arr.every((p) => Number.isFinite(Number(p))));
  const arraysOk = [input?.auth_boundary_findings, input?.issues, input?.recommendations, input?.evidenceRefs]
    .every((arr) => Array.isArray(arr) && arr.every((x) => typeof x === 'string'));
  const issuesCountMatch = Number(input?.issuesCount) === (Array.isArray(input?.issues) ? input.issues.length : 0);
  const riskOk = RISK_LEVELS.includes(input?.risk_level);
  const classOk = ['passed', 'failed', 'inconclusive'].includes(input?.resultClass);
  const confidenceOk = typeof input?.confidence === 'number' && Number.isFinite(input.confidence) && input.confidence >= 0 && input.confidence <= 1;

  return {
    ok:
      missingKeys.length === 0 &&
      input?.contractVersion === PORT_AUDIT_RESULT_CONTRACT_VERSION &&
      portsOk &&
      arraysOk &&
      issuesCountMatch &&
      riskOk &&
      classOk &&
      confidenceOk,
    missingKeys,
  };
}

export function formatPortAuditResultV1Marker(result = {}) {
  return [
    'PORT_AUDIT_RESULT_V1',
    `target=${result?.target_id || 'n/a'}`,
    `risk=${result?.risk_level || 'unknown'}`,
    `class=${result?.resultClass || 'inconclusive'}`,
    `audited_ports=${Array.isArray(result?.audited_ports) ? result.audited_ports.length : 0}`,
    `issues=${Array.isArray(result?.issues) ? result.issues.length : 0}`,
    `confidence=${Number(result?.confidence || 0).toFixed(4)}`,
  ].join('|');
}
