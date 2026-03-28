import {
  SURFACE_SCAN_RESULT_CONTRACT_VERSION,
  normalizeSurfaceScanResultV1,
  validateSurfaceScanResultV1Shape,
} from './surfaceScanResultContract';
import {
  SPIDER_EVIDENCE_REPORT_VERSION,
  buildSpiderEvidenceReportV1,
  validateSpiderEvidenceReportV1Shape,
} from './spiderEvidenceReport';

export const SPIDER_BASELINE_PACK_VERSION = 'spider_baseline_pack_v1';

export const DEFAULT_SPIDER_BASELINE_CASES_V1 = [
  {
    caseId: 'spider_known_good_surface_reachable',
    category: 'known-good',
    expectedClass: 'passed',
    expectedSignal: 'moderate_or_strong',
    expectEnrichment: true,
    expectAuthBoundaryLowNoise: true,
    surfaceInput: {
      target_id: 'spider-good-1',
      host: 'baseline-good.local',
      reachable: true,
      resultClass: 'passed',
      services: [{ port: 80, service: 'http', protocol: 'tcp' }],
      web_endpoints: ['/index.html', '/api/status'],
      stream_hints: ['rtsp_status:open'],
      archive_hints: ['archive_path:/archive'],
      vendor_hints: ['vendor:hikvision', 'fp_vendor:hikvision'],
      auth_boundary_hints: ['auth_boundary:segmented'],
      evidenceRefs: ['target:baseline-good.local', 'pages_crawled:3'],
      confidence: 0.82,
    },
  },
  {
    caseId: 'spider_known_bad_overconfident_noise',
    category: 'known-bad',
    expectedClass: 'failed',
    expectedSignal: 'weak',
    expectEnrichment: false,
    expectAuthBoundaryLowNoise: false,
    surfaceInput: {
      target_id: 'spider-bad-1',
      host: 'baseline-bad.local',
      reachable: false,
      resultClass: 'failed',
      services: [],
      web_endpoints: [],
      stream_hints: [],
      archive_hints: [],
      vendor_hints: ['fp_vendor:unknown', 'fp_os:linux', 'vendor:generic'],
      auth_boundary_hints: ['auth_boundary:strict', 'insufficient_signal'],
      evidenceRefs: ['target:baseline-bad.local', 'pages_crawled:0'],
      confidence: 0.18,
    },
  },
  {
    caseId: 'spider_ambiguous_partial_signal',
    category: 'ambiguous',
    expectedClass: 'inconclusive',
    expectedSignal: 'weak_or_moderate',
    expectEnrichment: false,
    expectAuthBoundaryLowNoise: true,
    surfaceInput: {
      target_id: 'spider-ambiguous-1',
      host: 'baseline-ambiguous.local',
      reachable: true,
      resultClass: 'inconclusive',
      services: [],
      web_endpoints: ['/login'],
      stream_hints: [],
      archive_hints: [],
      vendor_hints: [],
      auth_boundary_hints: ['insufficient_signal'],
      evidenceRefs: ['target:baseline-ambiguous.local', 'pages_crawled:1'],
      confidence: 0.37,
    },
  },
];

function safeClass(value) {
  return ['passed', 'failed', 'inconclusive'].includes(value) ? value : 'inconclusive';
}

function hasExplainableEnrichment(surface = {}) {
  return (
    (Array.isArray(surface?.vendor_hints) && surface.vendor_hints.length > 0) ||
    (Array.isArray(surface?.stream_hints) && surface.stream_hints.length > 0) ||
    (Array.isArray(surface?.archive_hints) && surface.archive_hints.length > 0)
  );
}

function authBoundaryHintConsistency(surface = {}) {
  const hints = Array.isArray(surface?.auth_boundary_hints) ? surface.auth_boundary_hints : [];
  const hasInsufficientSignal = hints.includes('insufficient_signal');
  const cls = safeClass(surface?.resultClass);
  if (!surface?.reachable && cls === 'passed') return false;
  if (hasInsufficientSignal && cls === 'passed') return false;
  return true;
}

function evidenceReportConsistency(surface = {}, evidenceReport = {}) {
  const shape = validateSpiderEvidenceReportV1Shape(evidenceReport);
  if (!shape.ok) return false;

  const summary = evidenceReport?.surfaceSummary || {};
  const serviceCount = Array.isArray(surface?.services) ? surface.services.length : 0;
  const webCount = Array.isArray(surface?.web_endpoints) ? surface.web_endpoints.length : 0;
  const evidenceCount = Array.isArray(surface?.evidenceRefs) ? surface.evidenceRefs.length : 0;

  return (
    summary?.resultClass === safeClass(surface?.resultClass) &&
    Boolean(summary?.reachable) === Boolean(surface?.reachable) &&
    Number(summary?.totalServices || 0) === serviceCount &&
    Number(summary?.totalWebEndpoints || 0) === webCount &&
    Number(summary?.totalEvidenceRefs || 0) === evidenceCount
  );
}

function signalExpectationMet(expectedSignal, evidenceReport = {}) {
  const signal = String(evidenceReport?.surfaceSummary?.signalStrength || 'weak');
  if (expectedSignal === 'weak') return signal === 'weak';
  if (expectedSignal === 'moderate_or_strong') return signal === 'moderate' || signal === 'strong';
  if (expectedSignal === 'weak_or_moderate') return signal === 'weak' || signal === 'moderate';
  return true;
}

function buildCaseReport(caseDef = {}) {
  const surfaceResult = normalizeSurfaceScanResultV1(caseDef?.surfaceInput || {});
  const evidenceReport = buildSpiderEvidenceReportV1({ surfaceScanResult: surfaceResult });

  const expectedClass = safeClass(caseDef?.expectedClass);
  const classMatch = safeClass(surfaceResult?.resultClass) === expectedClass;
  const hasSurfaceContract =
    surfaceResult?.contractVersion === SURFACE_SCAN_RESULT_CONTRACT_VERSION &&
    validateSurfaceScanResultV1Shape(surfaceResult).ok;
  const enrichmentPresentWhenExpected = caseDef?.expectEnrichment
    ? hasExplainableEnrichment(surfaceResult)
    : true;
  const authConsistency = authBoundaryHintConsistency(surfaceResult);
  const evidenceConsistency = evidenceReportConsistency(surfaceResult, evidenceReport);
  const signalExpectation = signalExpectationMet(caseDef?.expectedSignal, evidenceReport);

  const status =
    classMatch &&
    hasSurfaceContract &&
    enrichmentPresentWhenExpected &&
    authConsistency &&
    evidenceConsistency &&
    signalExpectation
      ? 'passed'
      : !hasSurfaceContract || !evidenceConsistency
        ? 'failed'
        : 'inconclusive';

  return {
    caseId: caseDef?.caseId || 'spider_case_unknown',
    category: caseDef?.category || 'ambiguous',
    status,
    expected: {
      class: expectedClass,
      signal: caseDef?.expectedSignal || 'any',
      expectEnrichment: Boolean(caseDef?.expectEnrichment),
    },
    actual: {
      class: safeClass(surfaceResult?.resultClass),
      signal: String(evidenceReport?.surfaceSummary?.signalStrength || 'weak'),
    },
    checks: {
      classMatch,
      hasSurfaceContract,
      enrichmentPresentWhenExpected,
      authBoundaryHintConsistency: authConsistency,
      evidenceReportConsistency: evidenceConsistency,
      signalExpectation,
      evidenceReportVersionMatch:
        evidenceReport?.reportVersion === SPIDER_EVIDENCE_REPORT_VERSION,
    },
    surfaceResult,
    evidenceReport,
  };
}

export function spiderBaselineMetrics(caseReports = []) {
  const total = caseReports.length;
  const byStatus = { passed: 0, failed: 0, inconclusive: 0 };
  const byCategory = { 'known-good': 0, 'known-bad': 0, ambiguous: 0, other: 0 };

  for (const report of caseReports) {
    byStatus[report.status] = Number(byStatus[report.status] || 0) + 1;
    const cat = ['known-good', 'known-bad', 'ambiguous'].includes(report.category)
      ? report.category
      : 'other';
    byCategory[cat] = Number(byCategory[cat] || 0) + 1;
  }

  const count = (key) => caseReports.filter((x) => x?.checks?.[key]).length;
  const rate = (n) => (total ? Number((n / total).toFixed(4)) : 0);

  return {
    total,
    byStatus,
    byCategory,
    classMatchRate: rate(count('classMatch')),
    surfaceContractCoverageRate: rate(count('hasSurfaceContract')),
    enrichmentExpectationRate: rate(count('enrichmentPresentWhenExpected')),
    authBoundaryConsistencyRate: rate(count('authBoundaryHintConsistency')),
    evidenceReportConsistencyRate: rate(count('evidenceReportConsistency')),
    signalExpectationRate: rate(count('signalExpectation')),
  };
}

export async function runSpiderBaselinePackV1({
  cases = DEFAULT_SPIDER_BASELINE_CASES_V1,
  includeContinuity = true,
} = {}) {
  const normalizedCases = (Array.isArray(cases) ? cases : []).map((item, idx) => ({
    caseId: item?.caseId || `spider_baseline_case_${idx + 1}`,
    category: item?.category || 'ambiguous',
    expectedClass: safeClass(item?.expectedClass),
    expectedSignal: String(item?.expectedSignal || 'any'),
    expectEnrichment: Boolean(item?.expectEnrichment),
    expectAuthBoundaryLowNoise: item?.expectAuthBoundaryLowNoise !== false,
    surfaceInput: item?.surfaceInput || {},
  }));

  const caseReports = normalizedCases.map((item) => buildCaseReport(item));
  const metrics = spiderBaselineMetrics(caseReports);

  return {
    packId: SPIDER_BASELINE_PACK_VERSION,
    surfaceContractVersion: SURFACE_SCAN_RESULT_CONTRACT_VERSION,
    evidenceReportVersion: SPIDER_EVIDENCE_REPORT_VERSION,
    createdAt: new Date().toISOString(),
    caseReports,
    metrics,
    continuity: includeContinuity
      ? {
          readyForNextStep: true,
          nextStepHint: 'phase30_11_phase_aligned_followup',
        }
      : null,
  };
}

export function formatSpiderBaselineCompactSummaryV1(report = {}) {
  const metrics = report?.metrics || {};
  return [
    `packId=${report?.packId || SPIDER_BASELINE_PACK_VERSION}`,
    `surfaceContractVersion=${report?.surfaceContractVersion || SURFACE_SCAN_RESULT_CONTRACT_VERSION}`,
    `evidenceReportVersion=${report?.evidenceReportVersion || SPIDER_EVIDENCE_REPORT_VERSION}`,
    `total=${Number(metrics?.total || 0)}`,
    `passed=${Number(metrics?.byStatus?.passed || 0)}`,
    `failed=${Number(metrics?.byStatus?.failed || 0)}`,
    `inconclusive=${Number(metrics?.byStatus?.inconclusive || 0)}`,
    `classMatchRate=${Number(metrics?.classMatchRate || 0).toFixed(4)}`,
    `surfaceContractCoverageRate=${Number(metrics?.surfaceContractCoverageRate || 0).toFixed(4)}`,
    `authBoundaryConsistencyRate=${Number(metrics?.authBoundaryConsistencyRate || 0).toFixed(4)}`,
    `evidenceConsistencyRate=${Number(metrics?.evidenceReportConsistencyRate || 0).toFixed(4)}`,
    `SPIDER_BASELINE_V1|cases=${Number(metrics?.total || 0)}`,
  ].join(' | ');
}
