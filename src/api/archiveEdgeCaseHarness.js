import { ARCHIVE_RESULT_CONTRACT_VERSION, normalizeArchiveResultV1 } from './archiveResultContract';
import { runArchiveBaselinePackV1 } from './archiveBaselinePack';

export const DEFAULT_ARCHIVE_EDGE_CASES_V1 = [
  {
    caseId: 'edge_malformed_success_integrity_mismatch',
    category: 'malformed_success',
    expectedClass: 'failed',
    input: {
      target_id: 'edge_target_a',
      archive_path_type: 'isapi_export_direct',
      search_supported: true,
      search_requires_auth: true,
      export_supported: true,
      export_requires_auth: true,
      partial_access_detected: false,
      timeout_detected: false,
      integrity_status: 'failed',
      issues: ['malformed_success_integrity_mismatch'],
      evidenceRefs: ['edge:malformed_success'],
      confidence: 0.84,
      resultClass: 'passed',
    },
  },
  {
    caseId: 'edge_partial_access_before_auth',
    category: 'partial_access',
    expectedClass: 'failed',
    input: {
      target_id: 'edge_target_b',
      archive_path_type: 'archive_export_probe',
      search_supported: true,
      search_requires_auth: false,
      export_supported: true,
      export_requires_auth: false,
      partial_access_detected: true,
      timeout_detected: false,
      integrity_status: 'degraded',
      issues: ['partial_access_before_auth', 'weak_boundary'],
      evidenceRefs: ['edge:partial_access'],
      confidence: 0.8,
      resultClass: 'failed',
    },
  },
  {
    caseId: 'edge_timeout_unstable_probe',
    category: 'timeout_unstable',
    expectedClass: 'inconclusive',
    input: {
      target_id: 'edge_target_c',
      archive_path_type: 'onvif_search',
      search_supported: true,
      search_requires_auth: true,
      export_supported: false,
      export_requires_auth: true,
      partial_access_detected: false,
      timeout_detected: true,
      integrity_status: 'unknown',
      issues: ['timeout_detected', 'unstable_endpoint'],
      evidenceRefs: ['edge:timeout'],
      confidence: 0.33,
      resultClass: 'inconclusive',
    },
  },
  {
    caseId: 'edge_empty_parameters',
    category: 'parameter_edge',
    expectedClass: 'failed',
    input: {
      target_id: '',
      archive_path_type: 'isapi_search',
      search_supported: true,
      search_requires_auth: true,
      export_supported: false,
      export_requires_auth: true,
      partial_access_detected: false,
      timeout_detected: false,
      integrity_status: 'failed',
      issues: ['empty_target_or_params'],
      evidenceRefs: ['edge:param_empty'],
      confidence: 0.95,
      resultClass: 'failed',
    },
  },
  {
    caseId: 'edge_invalid_time_range',
    category: 'parameter_edge',
    expectedClass: 'failed',
    input: {
      target_id: 'edge_target_d',
      archive_path_type: 'isapi_search',
      search_supported: true,
      search_requires_auth: true,
      export_supported: false,
      export_requires_auth: true,
      partial_access_detected: false,
      timeout_detected: false,
      integrity_status: 'failed',
      issues: ['invalid_time_range', 'from_after_to'],
      evidenceRefs: ['edge:param_time_range'],
      confidence: 0.9,
      resultClass: 'failed',
    },
  },
  {
    caseId: 'edge_weak_channel_identifier',
    category: 'parameter_edge',
    expectedClass: 'inconclusive',
    input: {
      target_id: 'edge_target_e',
      archive_path_type: 'onvif_search',
      search_supported: true,
      search_requires_auth: true,
      export_supported: false,
      export_requires_auth: true,
      partial_access_detected: false,
      timeout_detected: false,
      integrity_status: 'unknown',
      issues: ['weak_channel_identifier'],
      evidenceRefs: ['edge:param_channel'],
      confidence: 0.42,
      resultClass: 'inconclusive',
    },
  },
  {
    caseId: 'edge_auth_boundary_ambiguous',
    category: 'auth_boundary_ambiguity',
    expectedClass: 'inconclusive',
    input: {
      target_id: 'edge_target_f',
      archive_path_type: 'archive_export_probe',
      search_supported: true,
      search_requires_auth: true,
      export_supported: true,
      export_requires_auth: true,
      partial_access_detected: true,
      timeout_detected: false,
      integrity_status: 'degraded',
      issues: ['auth_boundary_inconsistent'],
      evidenceRefs: ['edge:auth_boundary'],
      confidence: 0.47,
      resultClass: 'inconclusive',
    },
  },
];

function safeClass(v) {
  return ['passed', 'failed', 'inconclusive'].includes(v) ? v : 'inconclusive';
}

function buildEdgeCaseReport(item, normalized) {
  const expectedClass = safeClass(item.expectedClass);
  const actualClass = safeClass(normalized.resultClass);
  const classMatch = expectedClass === actualClass;
  const issuesCountMatch = normalized.issuesCount === normalized.issues.length;
  const contractVersionOk = normalized.contractVersion === ARCHIVE_RESULT_CONTRACT_VERSION;

  const status = classMatch && issuesCountMatch && contractVersionOk
    ? 'passed'
    : !issuesCountMatch || !contractVersionOk
      ? 'failed'
      : 'inconclusive';

  return {
    caseId: item.caseId,
    category: item.category,
    expectedClass,
    actualClass,
    status,
    checks: {
      classMatch,
      issuesCountMatch,
      contractVersionOk,
    },
    result: normalized,
  };
}

export function archiveEdgeCaseMetrics(edgeCaseReports = []) {
  const total = edgeCaseReports.length;
  const byStatus = { passed: 0, failed: 0, inconclusive: 0 };
  const byCategory = {};
  const byExpectedClass = { passed: 0, failed: 0, inconclusive: 0 };
  const byActualClass = { passed: 0, failed: 0, inconclusive: 0 };

  for (const report of edgeCaseReports) {
    byStatus[report.status] = Number(byStatus[report.status] || 0) + 1;
    byExpectedClass[report.expectedClass] = Number(byExpectedClass[report.expectedClass] || 0) + 1;
    byActualClass[report.actualClass] = Number(byActualClass[report.actualClass] || 0) + 1;
    byCategory[report.category] = Number(byCategory[report.category] || 0) + 1;
  }

  const classMatchCount = edgeCaseReports.filter((item) => item.checks.classMatch).length;
  return {
    total,
    byStatus,
    byCategory,
    byExpectedClass,
    byActualClass,
    classMatchRate: total ? Number((classMatchCount / total).toFixed(4)) : 0,
    failedCount: byStatus.failed,
    inconclusiveCount: byStatus.inconclusive,
  };
}

export async function runArchiveEdgeCaseHarnessV1({
  cases = DEFAULT_ARCHIVE_EDGE_CASES_V1,
  includeBaseline = true,
} = {}) {
  const baseline = includeBaseline ? await runArchiveBaselinePackV1() : null;
  const normalizedCases = (Array.isArray(cases) ? cases : []).map((item, idx) => ({
    caseId: item?.caseId || `edge_case_${idx + 1}`,
    category: item?.category || 'unspecified',
    expectedClass: safeClass(item?.expectedClass || 'inconclusive'),
    input: item?.input || {},
  }));

  const edgeCaseReports = normalizedCases.map((item) => {
    const normalized = normalizeArchiveResultV1(item.input);
    return buildEdgeCaseReport(item, normalized);
  });

  const metrics = archiveEdgeCaseMetrics(edgeCaseReports);

  return {
    harnessId: 'archive_edge_case_harness_v1',
    contractVersion: ARCHIVE_RESULT_CONTRACT_VERSION,
    createdAt: new Date().toISOString(),
    baseline,
    edgeCaseReports,
    metrics,
  };
}

export function formatArchiveEdgeCaseCompactSummary(report) {
  const metrics = report?.metrics || {};
  return [
    `harnessId=${report?.harnessId || 'n/a'}`,
    `contractVersion=${report?.contractVersion || ARCHIVE_RESULT_CONTRACT_VERSION}`,
    `total=${Number(metrics.total || 0)}`,
    `passed=${Number(metrics.byStatus?.passed || 0)}`,
    `failed=${Number(metrics.byStatus?.failed || 0)}`,
    `inconclusive=${Number(metrics.byStatus?.inconclusive || 0)}`,
    `classMatchRate=${Number(metrics.classMatchRate || 0).toFixed(4)}`,
    `byCategory=${JSON.stringify(metrics.byCategory || {})}`,
  ].join(' | ');
}
