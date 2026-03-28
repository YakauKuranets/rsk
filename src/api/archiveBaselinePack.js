import { ARCHIVE_RESULT_CONTRACT_VERSION, normalizeArchiveResultV1 } from './archiveResultContract';
import { AUTH_RESULT_CONTRACT_VERSION, validateAuthResultV1Shape } from './authResultContract';

export const DEFAULT_ARCHIVE_BASELINE_PACK_V1_CASES = [
  {
    caseId: 'archive_known_good_direct',
    label: 'known-good',
    expectedClass: 'passed',
    input: {
      target_id: 'lab_known_good_target',
      archive_path_type: 'isapi_export_direct',
      search_supported: true,
      search_requires_auth: true,
      export_supported: true,
      export_requires_auth: true,
      partial_access_detected: false,
      timeout_detected: false,
      integrity_status: 'ok',
      issues: [],
      evidenceRefs: ['path:isapi_export_direct', 'auth:required', 'records:3'],
      confidence: 0.88,
      resultClass: 'passed',
    },
  },
  {
    caseId: 'archive_known_bad_partial_integrity',
    label: 'known-bad',
    expectedClass: 'failed',
    input: {
      target_id: 'lab_known_bad_target',
      archive_path_type: 'archive_export_probe',
      search_supported: true,
      search_requires_auth: false,
      export_supported: true,
      export_requires_auth: false,
      partial_access_detected: true,
      timeout_detected: false,
      integrity_status: 'failed',
      issues: ['weak_archive_auth_boundary', 'partial_access_detected', 'integrity_malformed_success'],
      evidenceRefs: ['path:archive_export_probe', 'auth:weak', 'integrity:failed'],
      confidence: 0.91,
      resultClass: 'failed',
    },
  },
  {
    caseId: 'archive_ambiguous_timeout',
    label: 'ambiguous',
    expectedClass: 'inconclusive',
    input: {
      target_id: 'lab_ambiguous_target',
      archive_path_type: 'onvif_search',
      search_supported: true,
      search_requires_auth: true,
      export_supported: false,
      export_requires_auth: true,
      partial_access_detected: false,
      timeout_detected: true,
      integrity_status: 'unknown',
      issues: ['timeout_detected', 'insufficient_signal'],
      evidenceRefs: ['path:onvif_search', 'endpoint:unstable'],
      confidence: 0.35,
      resultClass: 'inconclusive',
    },
  },
];

function safeClass(value) {
  return ['passed', 'failed', 'inconclusive'].includes(value) ? value : 'inconclusive';
}

function buildCaseReport(item, normalized) {
  const expectedClass = safeClass(item?.expectedClass);
  const classMatch = normalized.resultClass === expectedClass;
  const issuesCountMatch = normalized.issuesCount === normalized.issues.length;
  const hasContractVersion = normalized.contractVersion === ARCHIVE_RESULT_CONTRACT_VERSION;
  const authShapeOk = validateAuthResultV1Shape(normalized?.authResult).ok;
  const authTargetMatch = normalized?.authResult?.target_id === normalized?.target_id;
  const authPathTypeMatch =
    normalized?.authResult?.auth_path_type === `archive:${String(normalized?.archive_path_type || 'unknown')}`;

  const status = classMatch && issuesCountMatch && hasContractVersion && authShapeOk && authTargetMatch && authPathTypeMatch
    ? 'passed'
    : classMatch || issuesCountMatch || hasContractVersion || authShapeOk
      ? 'inconclusive'
      : 'failed';

  return {
    caseId: item.caseId,
    label: item.label,
    expectedClass,
    actualClass: normalized.resultClass,
    status,
    checks: {
      classMatch,
      issuesCountMatch,
      hasContractVersion,
      authShapeOk,
      authTargetMatch,
      authPathTypeMatch,
    },
    result: normalized,
  };
}

export function archiveBaselineMetrics(caseReports = []) {
  const total = caseReports.length;
  const byStatus = { passed: 0, failed: 0, inconclusive: 0 };
  const byActualClass = { passed: 0, failed: 0, inconclusive: 0 };

  for (const report of caseReports) {
    byStatus[report.status] = Number(byStatus[report.status] || 0) + 1;
    byActualClass[report.actualClass] = Number(byActualClass[report.actualClass] || 0) + 1;
  }

  const score = total ? Number((byStatus.passed / total).toFixed(4)) : 0;
  return {
    total,
    byStatus,
    byActualClass,
    passRate: score,
    failedCount: byStatus.failed,
    inconclusiveCount: byStatus.inconclusive,
  };
}

export async function runArchiveBaselinePackV1({
  cases = DEFAULT_ARCHIVE_BASELINE_PACK_V1_CASES,
} = {}) {
  const normalizedCases = (Array.isArray(cases) ? cases : []).map((item, idx) => ({
    caseId: item?.caseId || `archive_case_${idx + 1}`,
    label: item?.label || `case_${idx + 1}`,
    expectedClass: safeClass(item?.expectedClass || 'inconclusive'),
    input: item?.input || {},
  }));

  const caseReports = normalizedCases.map((item) => {
    const normalized = normalizeArchiveResultV1(item.input);
    return buildCaseReport(item, normalized);
  });

  const metrics = archiveBaselineMetrics(caseReports);

  return {
    packId: 'archive_baseline_known_bad_pack_v1',
    contractVersion: ARCHIVE_RESULT_CONTRACT_VERSION,
    createdAt: new Date().toISOString(),
    caseReports,
    metrics,
  };
}

export function formatArchiveBaselineCompactSummary(report) {
  const metrics = report?.metrics || {};
  return [
    `packId=${report?.packId || 'n/a'}`,
    `contractVersion=${report?.contractVersion || ARCHIVE_RESULT_CONTRACT_VERSION}`,
    `total=${Number(metrics.total || 0)}`,
    `passed=${Number(metrics.byStatus?.passed || 0)}`,
    `failed=${Number(metrics.byStatus?.failed || 0)}`,
    `inconclusive=${Number(metrics.byStatus?.inconclusive || 0)}`,
    `passRate=${Number(metrics.passRate || 0).toFixed(4)}`,
    `classDist=${JSON.stringify(metrics.byActualClass || {})}`,
    `authContractVersion=${AUTH_RESULT_CONTRACT_VERSION}`,
    `AUTH_RESULT_V1|present=${(report?.caseReports || []).filter((x) => x?.checks?.authShapeOk).length}/${(report?.caseReports || []).length || 0}`,
  ].join(' | ');
}
