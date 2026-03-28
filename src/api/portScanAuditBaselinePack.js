import {
  PORT_SCAN_RESULT_CONTRACT_VERSION,
  normalizePortScanResultV1,
  validatePortScanResultV1Shape,
} from './portScanResultContract';
import {
  PORT_AUDIT_RESULT_CONTRACT_VERSION,
  normalizePortAuditFromScanResultV1,
  validatePortAuditResultV1Shape,
} from './portAuditResultContract';

export const PORT_SCAN_AUDIT_BASELINE_PACK_VERSION = 'port_scan_audit_baseline_pack_v1';

export const DEFAULT_PORT_SCAN_AUDIT_BASELINE_CASES_V1 = [
  {
    caseId: 'port_scan_known_good_https_only',
    category: 'known-good',
    expectedScanClass: 'passed',
    expectedAuditClass: 'passed',
    expectedRiskLevel: 'low',
    expectedOpenPorts: [443],
    scanInput: {
      target_id: 'baseline_good_target',
      host: 'baseline-good.local',
      reachable: true,
      open_ports: [443],
      services: ['https'],
      protocol: 'tcp',
      banner: 'nginx',
      vendor_hints: ['hint:nginx'],
      evidenceRefs: ['baseline:good'],
      confidence: 0.86,
      resultClass: 'passed',
    },
  },
  {
    caseId: 'port_scan_known_bad_telnet_sensitive',
    category: 'known-bad',
    expectedScanClass: 'passed',
    expectedAuditClass: 'failed',
    expectedRiskLevel: 'critical',
    expectedOpenPorts: [443],
    scanInput: {
      target_id: 'baseline_bad_target',
      host: 'baseline-bad.local',
      reachable: true,
      open_ports: [23, 445],
      services: ['telnet', 'smb1'],
      protocol: 'tcp',
      banner: 'legacy_stack',
      vendor_hints: ['hint:legacy'],
      evidenceRefs: ['baseline:bad'],
      confidence: 0.9,
      resultClass: 'passed',
    },
  },
  {
    caseId: 'port_scan_ambiguous_unreachable',
    category: 'ambiguous',
    expectedScanClass: 'failed',
    expectedAuditClass: 'inconclusive',
    expectedRiskLevel: 'unknown',
    expectedOpenPorts: [80, 443, 554],
    scanInput: {
      target_id: 'baseline_ambiguous_target',
      host: 'baseline-ambiguous.local',
      reachable: false,
      open_ports: [],
      services: [],
      protocol: null,
      banner: null,
      vendor_hints: [],
      evidenceRefs: ['baseline:ambiguous'],
      confidence: 0.2,
      resultClass: 'failed',
    },
  },
];

function safeClass(value) {
  return ['passed', 'failed', 'inconclusive'].includes(value) ? value : 'inconclusive';
}

function riskInterpretationConsistent(audit) {
  const risk = String(audit?.risk_level || 'unknown');
  const cls = safeClass(audit?.resultClass);
  if (risk === 'critical' || risk === 'high') return cls === 'failed';
  if (risk === 'medium' || risk === 'unknown') return cls === 'inconclusive';
  if (risk === 'low') return cls === 'passed';
  return false;
}

function buildCaseReport(caseDef, scanResult, auditResult) {
  const expectedScanClass = safeClass(caseDef?.expectedScanClass);
  const expectedAuditClass = safeClass(caseDef?.expectedAuditClass);
  const expectedRiskLevel = String(caseDef?.expectedRiskLevel || 'unknown');

  const scanClassMatch = safeClass(scanResult?.resultClass) === expectedScanClass;
  const auditClassMatch = safeClass(auditResult?.resultClass) === expectedAuditClass;
  const riskMatch = String(auditResult?.risk_level || 'unknown') === expectedRiskLevel;
  const issuesCountMatch = Number(auditResult?.issuesCount || 0) === (Array.isArray(auditResult?.issues) ? auditResult.issues.length : 0);
  const hasScanContract =
    scanResult?.contractVersion === PORT_SCAN_RESULT_CONTRACT_VERSION &&
    validatePortScanResultV1Shape(scanResult).ok;
  const hasAuditContract =
    auditResult?.contractVersion === PORT_AUDIT_RESULT_CONTRACT_VERSION &&
    validatePortAuditResultV1Shape(auditResult).ok;
  const riskConsistent = riskInterpretationConsistent(auditResult);

  const status =
    scanClassMatch &&
    auditClassMatch &&
    riskMatch &&
    issuesCountMatch &&
    hasScanContract &&
    hasAuditContract &&
    riskConsistent
      ? 'passed'
      : !hasScanContract || !hasAuditContract || !issuesCountMatch
        ? 'failed'
        : 'inconclusive';

  return {
    caseId: caseDef.caseId,
    category: caseDef.category,
    status,
    expected: {
      scanClass: expectedScanClass,
      auditClass: expectedAuditClass,
      riskLevel: expectedRiskLevel,
    },
    actual: {
      scanClass: scanResult?.resultClass || 'inconclusive',
      auditClass: auditResult?.resultClass || 'inconclusive',
      riskLevel: auditResult?.risk_level || 'unknown',
    },
    checks: {
      classMatch: scanClassMatch && auditClassMatch,
      scanClassMatch,
      auditClassMatch,
      riskMatch,
      issuesCountMatch,
      hasScanContract,
      hasAuditContract,
      riskInterpretationConsistent: riskConsistent,
    },
    scanResult,
    auditResult,
  };
}

export function portScanAuditBaselineMetrics(caseReports = []) {
  const total = caseReports.length;
  const byStatus = { passed: 0, failed: 0, inconclusive: 0 };
  const byCategory = { 'known-good': 0, 'known-bad': 0, ambiguous: 0, other: 0 };
  const byRiskLevel = { low: 0, medium: 0, high: 0, critical: 0, unknown: 0 };

  for (const item of caseReports) {
    byStatus[item.status] = Number(byStatus[item.status] || 0) + 1;
    const cat = ['known-good', 'known-bad', 'ambiguous'].includes(item.category) ? item.category : 'other';
    byCategory[cat] = Number(byCategory[cat] || 0) + 1;
    const risk = String(item?.actual?.riskLevel || 'unknown');
    byRiskLevel[risk] = Number(byRiskLevel[risk] || 0) + 1;
  }

  const classMatchCount = caseReports.filter((x) => x?.checks?.classMatch).length;
  const issuesCountMatchCount = caseReports.filter((x) => x?.checks?.issuesCountMatch).length;
  const scanContractCount = caseReports.filter((x) => x?.checks?.hasScanContract).length;
  const auditContractCount = caseReports.filter((x) => x?.checks?.hasAuditContract).length;
  const riskConsistentCount = caseReports.filter((x) => x?.checks?.riskInterpretationConsistent).length;

  return {
    total,
    byStatus,
    byCategory,
    byRiskLevel,
    classMatchRate: total ? Number((classMatchCount / total).toFixed(4)) : 0,
    issuesCountMatchRate: total ? Number((issuesCountMatchCount / total).toFixed(4)) : 0,
    scanContractCoverageRate: total ? Number((scanContractCount / total).toFixed(4)) : 0,
    auditContractCoverageRate: total ? Number((auditContractCount / total).toFixed(4)) : 0,
    riskInterpretationConsistencyRate: total ? Number((riskConsistentCount / total).toFixed(4)) : 0,
  };
}

export async function runPortScanAuditBaselinePackV1({
  cases = DEFAULT_PORT_SCAN_AUDIT_BASELINE_CASES_V1,
} = {}) {
  const normalizedCases = (Array.isArray(cases) ? cases : []).map((item, idx) => ({
    caseId: item?.caseId || `port_scan_audit_case_${idx + 1}`,
    category: item?.category || 'ambiguous',
    expectedScanClass: safeClass(item?.expectedScanClass || 'inconclusive'),
    expectedAuditClass: safeClass(item?.expectedAuditClass || 'inconclusive'),
    expectedRiskLevel: String(item?.expectedRiskLevel || 'unknown'),
    expectedOpenPorts: Array.isArray(item?.expectedOpenPorts) ? item.expectedOpenPorts : [80, 443, 554],
    scanInput: item?.scanInput || {},
  }));

  const caseReports = normalizedCases.map((item) => {
    const scanResult = normalizePortScanResultV1(item.scanInput);
    const auditResult = normalizePortAuditFromScanResultV1(scanResult, {
      expectedOpenPorts: item.expectedOpenPorts,
    });
    return buildCaseReport(item, scanResult, auditResult);
  });

  const metrics = portScanAuditBaselineMetrics(caseReports);

  return {
    packId: PORT_SCAN_AUDIT_BASELINE_PACK_VERSION,
    scanContractVersion: PORT_SCAN_RESULT_CONTRACT_VERSION,
    auditContractVersion: PORT_AUDIT_RESULT_CONTRACT_VERSION,
    createdAt: new Date().toISOString(),
    caseReports,
    metrics,
    continuity: {
      readyForNextStep: true,
      nextStepHint: 'phase30_7_spider_fingerprint_enrichment_v1',
    },
  };
}

export function formatPortScanAuditBaselineCompactSummary(report = {}) {
  const metrics = report?.metrics || {};
  return [
    `packId=${report?.packId || PORT_SCAN_AUDIT_BASELINE_PACK_VERSION}`,
    `scanContractVersion=${report?.scanContractVersion || PORT_SCAN_RESULT_CONTRACT_VERSION}`,
    `auditContractVersion=${report?.auditContractVersion || PORT_AUDIT_RESULT_CONTRACT_VERSION}`,
    `total=${Number(metrics.total || 0)}`,
    `passed=${Number(metrics.byStatus?.passed || 0)}`,
    `failed=${Number(metrics.byStatus?.failed || 0)}`,
    `inconclusive=${Number(metrics.byStatus?.inconclusive || 0)}`,
    `classMatchRate=${Number(metrics.classMatchRate || 0).toFixed(4)}`,
    `scanContractCoverageRate=${Number(metrics.scanContractCoverageRate || 0).toFixed(4)}`,
    `auditContractCoverageRate=${Number(metrics.auditContractCoverageRate || 0).toFixed(4)}`,
    `riskConsistencyRate=${Number(metrics.riskInterpretationConsistencyRate || 0).toFixed(4)}`,
    `PORT_SCAN_AUDIT_BASELINE_V1|cases=${Number(metrics.total || 0)}`,
  ].join(' | ');
}
