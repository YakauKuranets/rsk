import {
  AUTH_RESULT_CONTRACT_VERSION,
  validateAuthResultV1Shape,
} from './authResultContract';
import {
  DEFAULT_CREDENTIAL_HYGIENE_EXPANDED_PROFILES_V1,
  CREDENTIAL_HYGIENE_AUDITOR_VERSION,
  runCredentialHygieneAuditorV1,
} from './credentialHygieneAuditor';

export const CREDENTIAL_HYGIENE_BASELINE_PACK_VERSION = 'credential_hygiene_baseline_pack_v1';

export const DEFAULT_CREDENTIAL_HYGIENE_BASELINE_CASES_V1 = [
  {
    caseId: 'cred_known_good_strong_boundary',
    category: 'known-good',
    expectedClass: 'passed',
    profileId: 'direct_known_good_strong_boundary',
  },
  {
    caseId: 'cred_known_bad_default_credential',
    category: 'known-bad',
    expectedClass: 'failed',
    profileId: 'archive_known_bad_default_cred_fixture',
  },
  {
    caseId: 'cred_known_bad_weak_password',
    category: 'known-bad',
    expectedClass: 'failed',
    profileId: 'direct_weak_password_fixture',
  },
  {
    caseId: 'cred_ambiguous_insufficient_signal',
    category: 'ambiguous',
    expectedClass: 'inconclusive',
    profileId: 'direct_ambiguous_insufficient_signal',
  },
  {
    caseId: 'cred_ambiguous_cookie_probe',
    category: 'ambiguous',
    expectedClass: 'inconclusive',
    profileId: 'cookie_local_tls_profile',
  },
];

function safeClass(value) {
  return ['passed', 'failed', 'inconclusive'].includes(value) ? value : 'inconclusive';
}

function boundaryInterpretationConsistent(report) {
  const auth = report?.authResult || {};
  const status = safeClass(report?.status);

  if (auth.auth_boundary_strength === 'strong') {
    return status === 'passed' || status === 'inconclusive';
  }
  if (auth.auth_boundary_strength === 'weak') {
    return status !== 'passed';
  }
  return true;
}

function buildCaseReport(caseDef, report) {
  const expectedClass = safeClass(caseDef?.expectedClass);
  const actualClass = safeClass(report?.status);
  const classMatch = expectedClass === actualClass;
  const issues = Array.isArray(report?.authResult?.issues) ? report.authResult.issues : [];
  const issuesCount = Number(report?.authResult?.issuesCount || 0);
  const issuesCountMatch = issues.length === issuesCount;
  const hasAuthContract =
    report?.authResult?.contractVersion === AUTH_RESULT_CONTRACT_VERSION &&
    validateAuthResultV1Shape(report?.authResult).ok;
  const boundaryConsistent = boundaryInterpretationConsistent(report);

  const status = classMatch && issuesCountMatch && hasAuthContract && boundaryConsistent
    ? 'passed'
    : !issuesCountMatch || !hasAuthContract
      ? 'failed'
      : 'inconclusive';

  return {
    caseId: caseDef.caseId,
    category: caseDef.category,
    profileId: caseDef.profileId,
    expectedClass,
    actualClass,
    status,
    checks: {
      classMatch,
      issuesCountMatch,
      hasAuthContract,
      boundaryInterpretationConsistent: boundaryConsistent,
    },
    report,
  };
}

export function credentialHygieneBaselineMetrics(caseReports = []) {
  const total = caseReports.length;
  const byStatus = { passed: 0, failed: 0, inconclusive: 0 };
  const byCategory = { 'known-good': 0, 'known-bad': 0, ambiguous: 0, other: 0 };
  const byExpectedClass = { passed: 0, failed: 0, inconclusive: 0 };
  const byActualClass = { passed: 0, failed: 0, inconclusive: 0 };

  for (const item of caseReports) {
    byStatus[item.status] = Number(byStatus[item.status] || 0) + 1;
    byExpectedClass[item.expectedClass] = Number(byExpectedClass[item.expectedClass] || 0) + 1;
    byActualClass[item.actualClass] = Number(byActualClass[item.actualClass] || 0) + 1;
    const category = ['known-good', 'known-bad', 'ambiguous'].includes(item.category)
      ? item.category
      : 'other';
    byCategory[category] = Number(byCategory[category] || 0) + 1;
  }

  const classMatchCount = caseReports.filter((x) => x?.checks?.classMatch).length;
  const issuesCountMatchCount = caseReports.filter((x) => x?.checks?.issuesCountMatch).length;
  const authContractCount = caseReports.filter((x) => x?.checks?.hasAuthContract).length;
  const boundaryConsistentCount = caseReports.filter((x) => x?.checks?.boundaryInterpretationConsistent).length;

  return {
    total,
    byStatus,
    byCategory,
    byExpectedClass,
    byActualClass,
    classMatchRate: total ? Number((classMatchCount / total).toFixed(4)) : 0,
    issuesCountMatchRate: total ? Number((issuesCountMatchCount / total).toFixed(4)) : 0,
    authContractCoverageRate: total ? Number((authContractCount / total).toFixed(4)) : 0,
    boundaryConsistencyRate: total ? Number((boundaryConsistentCount / total).toFixed(4)) : 0,
  };
}

export async function runCredentialHygieneBaselinePackV1({
  cases = DEFAULT_CREDENTIAL_HYGIENE_BASELINE_CASES_V1,
  profiles = DEFAULT_CREDENTIAL_HYGIENE_EXPANDED_PROFILES_V1,
  mode = 'discovery_mode',
  includeContinuity = true,
} = {}) {
  const profileMap = new Map((Array.isArray(profiles) ? profiles : []).map((p) => [p?.profileId, p]));

  const resolvedCases = (Array.isArray(cases) ? cases : []).map((item, idx) => ({
    caseId: item?.caseId || `credential_hygiene_case_${idx + 1}`,
    category: item?.category || 'ambiguous',
    expectedClass: safeClass(item?.expectedClass || 'inconclusive'),
    profileId: item?.profileId || null,
  }));

  const usedProfiles = resolvedCases
    .map((item) => profileMap.get(item.profileId))
    .filter(Boolean);

  const auditorResult = await runCredentialHygieneAuditorV1({
    profiles: usedProfiles,
    mode,
  });

  const reportByProfileId = new Map(
    (Array.isArray(auditorResult?.reports) ? auditorResult.reports : []).map((r) => [r?.profileId, r]),
  );

  const caseReports = resolvedCases.map((caseDef) => {
    const report = reportByProfileId.get(caseDef.profileId) || {
      profileId: caseDef.profileId,
      status: 'inconclusive',
      shapeOk: false,
      violations: ['profile_report_missing'],
      authResult: {
        contractVersion: AUTH_RESULT_CONTRACT_VERSION,
        target_id: null,
        auth_path_type: 'unknown',
        auth_required: true,
        weak_password_detected: false,
        default_credential_detected: false,
        auth_boundary_strength: 'unknown',
        partial_access_detected: false,
        issues: ['profile_report_missing'],
        issuesCount: 1,
        evidenceRefs: ['baseline:missing_profile'],
        confidence: 0,
        resultClass: 'inconclusive',
      },
    };
    return buildCaseReport(caseDef, report);
  });

  const metrics = credentialHygieneBaselineMetrics(caseReports);

  return {
    packId: CREDENTIAL_HYGIENE_BASELINE_PACK_VERSION,
    auditorVersion: CREDENTIAL_HYGIENE_AUDITOR_VERSION,
    authContractVersion: AUTH_RESULT_CONTRACT_VERSION,
    createdAt: new Date().toISOString(),
    mode,
    caseReports,
    metrics,
    continuity: includeContinuity
      ? {
          auditorSummary: {
            totalProfiles: auditorResult?.totalProfiles || 0,
            metrics: auditorResult?.metrics || null,
          },
        }
      : null,
  };
}

export function formatCredentialHygieneBaselineCompactSummary(report = {}) {
  const metrics = report?.metrics || {};
  return [
    `packId=${report?.packId || CREDENTIAL_HYGIENE_BASELINE_PACK_VERSION}`,
    `auditorVersion=${report?.auditorVersion || CREDENTIAL_HYGIENE_AUDITOR_VERSION}`,
    `authContractVersion=${report?.authContractVersion || AUTH_RESULT_CONTRACT_VERSION}`,
    `total=${Number(metrics.total || 0)}`,
    `passed=${Number(metrics.byStatus?.passed || 0)}`,
    `failed=${Number(metrics.byStatus?.failed || 0)}`,
    `inconclusive=${Number(metrics.byStatus?.inconclusive || 0)}`,
    `classMatchRate=${Number(metrics.classMatchRate || 0).toFixed(4)}`,
    `authContractCoverageRate=${Number(metrics.authContractCoverageRate || 0).toFixed(4)}`,
    `boundaryConsistencyRate=${Number(metrics.boundaryConsistencyRate || 0).toFixed(4)}`,
    `CREDENTIAL_HYGIENE_BASELINE_V1|cases=${Number(metrics.total || 0)}|mode=${report?.mode || 'n/a'}`,
  ].join(' | ');
}
