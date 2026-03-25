import { normalizeArchiveResultV1 } from './archiveResultContract';
import {
  AUTH_RESULT_CONTRACT_VERSION,
  normalizeAuthResultV1,
  validateAuthResultV1Shape,
} from './authResultContract';
import { verifySessionCookieFlagsCapability } from './capabilities';

export const CREDENTIAL_HYGIENE_AUDITOR_VERSION = 'credential_hygiene_auditor_v1';

export const DEFAULT_CREDENTIAL_HYGIENE_PROFILES_V1 = [
  {
    profileId: 'cookie_local_tls_profile',
    label: 'Cookie TLS controlled',
    sourceType: 'cookie_probe',
    targetId: 'https://localhost',
  },
  {
    profileId: 'archive_known_bad_default_cred_fixture',
    label: 'Archive fixture default cred',
    sourceType: 'archive_fixture',
    archiveInput: {
      target_id: 'fixture_default_cred_target',
      archive_path_type: 'archive_export_probe',
      search_supported: true,
      search_requires_auth: false,
      export_supported: true,
      export_requires_auth: false,
      partial_access_detected: true,
      timeout_detected: false,
      integrity_status: 'degraded',
      issues: ['default_credentials_exposed', 'auth_boundary_inconsistent'],
      evidenceRefs: ['fixture:default_cred'],
      confidence: 0.9,
      resultClass: 'failed',
    },
  },
  {
    profileId: 'direct_weak_password_fixture',
    label: 'Direct weak-password fixture',
    sourceType: 'direct_auth_fixture',
    authInput: {
      target_id: 'fixture_weak_password_target',
      auth_path_type: 'controlled_fixture:credential_hygiene',
      auth_required: true,
      weak_password_detected: true,
      default_credential_detected: false,
      auth_boundary_strength: 'weak',
      partial_access_detected: false,
      issues: ['weak_password_policy_detected'],
      evidenceRefs: ['fixture:weak_password'],
      confidence: 0.92,
      resultClass: 'failed',
    },
  },
];

function clampClass(value) {
  return ['passed', 'failed', 'inconclusive'].includes(value) ? value : 'inconclusive';
}

function deriveWeakAndDefaultFlags(authResult = {}) {
  const issues = Array.isArray(authResult.issues) ? authResult.issues : [];
  const weakFromIssues = issues.some((x) => /weak_password|weak_credential|weak_boundary/i.test(String(x)));
  const defaultFromIssues = issues.some((x) => /default_credential|default_credentials/i.test(String(x)));

  return {
    weak_password_detected: Boolean(authResult.weak_password_detected || weakFromIssues),
    default_credential_detected: Boolean(authResult.default_credential_detected || defaultFromIssues),
  };
}

async function resolveAuthResultFromProfile(profile, mode = 'discovery_mode') {
  const sourceType = String(profile?.sourceType || 'direct_auth_fixture');

  if (sourceType === 'cookie_probe') {
    const targetId = String(profile?.targetId || '').trim();
    const cookie = await verifySessionCookieFlagsCapability(targetId, mode);
    return normalizeAuthResultV1({
      ...(cookie?.authResult || {}),
      target_id: targetId || cookie?.authResult?.target_id || null,
      auth_path_type: cookie?.authResult?.auth_path_type || 'session_cookie_flags',
    });
  }

  if (sourceType === 'archive_fixture') {
    const archive = normalizeArchiveResultV1(profile?.archiveInput || {});
    return normalizeAuthResultV1(archive?.authResult || {});
  }

  return normalizeAuthResultV1(profile?.authInput || {});
}

function evaluateCredentialHygiene(authResultInput = {}) {
  const base = normalizeAuthResultV1(authResultInput);
  const derived = deriveWeakAndDefaultFlags(base);
  const authResult = normalizeAuthResultV1({
    ...base,
    ...derived,
    resultClass: clampClass(base.resultClass),
  });

  const violations = [];
  if (authResult.default_credential_detected) violations.push('default_credential_detected');
  if (authResult.weak_password_detected) violations.push('weak_password_detected');
  if (!authResult.auth_required) violations.push('auth_not_required');
  if (String(authResult.auth_boundary_strength) === 'weak') violations.push('auth_boundary_weak');
  if (authResult.partial_access_detected) violations.push('partial_access_detected');

  const status = violations.length > 0
    ? 'failed'
    : authResult.resultClass === 'passed'
      ? 'passed'
      : 'inconclusive';

  return {
    status,
    authResult,
    violations,
    shapeOk: validateAuthResultV1Shape(authResult).ok,
  };
}

export function credentialHygieneMetrics(reports = []) {
  const total = reports.length;
  const byStatus = { passed: 0, failed: 0, inconclusive: 0 };
  const shapeValidCount = reports.filter((r) => r?.shapeOk).length;
  const defaultCredentialDetectedCount = reports.filter((r) => r?.authResult?.default_credential_detected).length;
  const weakPasswordDetectedCount = reports.filter((r) => r?.authResult?.weak_password_detected).length;
  const authNotRequiredCount = reports.filter((r) => r?.authResult?.auth_required === false).length;
  const weakBoundaryCount = reports.filter((r) => r?.authResult?.auth_boundary_strength === 'weak').length;

  for (const report of reports) {
    byStatus[report.status] = Number(byStatus[report.status] || 0) + 1;
  }

  return {
    total,
    byStatus,
    shapeValidRate: total ? Number((shapeValidCount / total).toFixed(4)) : 0,
    defaultCredentialDetectedRate: total ? Number((defaultCredentialDetectedCount / total).toFixed(4)) : 0,
    weakPasswordDetectedRate: total ? Number((weakPasswordDetectedCount / total).toFixed(4)) : 0,
    authNotRequiredRate: total ? Number((authNotRequiredCount / total).toFixed(4)) : 0,
    weakBoundaryRate: total ? Number((weakBoundaryCount / total).toFixed(4)) : 0,
  };
}

export async function runCredentialHygieneAuditorV1({
  profiles = DEFAULT_CREDENTIAL_HYGIENE_PROFILES_V1,
  mode = 'discovery_mode',
} = {}) {
  const normalizedProfiles = (Array.isArray(profiles) ? profiles : []).map((item, idx) => ({
    profileId: item?.profileId || `credential_hygiene_profile_${idx + 1}`,
    label: item?.label || `Credential hygiene profile ${idx + 1}`,
    sourceType: item?.sourceType || 'direct_auth_fixture',
    targetId: item?.targetId || null,
    archiveInput: item?.archiveInput || null,
    authInput: item?.authInput || null,
  }));

  const reports = [];
  for (const profile of normalizedProfiles) {
    const authResult = await resolveAuthResultFromProfile(profile, mode);
    const evalResult = evaluateCredentialHygiene(authResult);
    reports.push({
      profileId: profile.profileId,
      label: profile.label,
      sourceType: profile.sourceType,
      targetId: evalResult.authResult?.target_id || profile.targetId || null,
      status: evalResult.status,
      shapeOk: evalResult.shapeOk,
      violations: evalResult.violations,
      authResult: evalResult.authResult,
    });
  }

  const metrics = credentialHygieneMetrics(reports);

  return {
    auditorId: CREDENTIAL_HYGIENE_AUDITOR_VERSION,
    authContractVersion: AUTH_RESULT_CONTRACT_VERSION,
    createdAt: new Date().toISOString(),
    mode,
    totalProfiles: reports.length,
    reports,
    metrics,
  };
}

export function formatCredentialHygieneCompactSummary(report = {}) {
  const metrics = report?.metrics || {};
  return [
    `auditorId=${report?.auditorId || CREDENTIAL_HYGIENE_AUDITOR_VERSION}`,
    `authContractVersion=${report?.authContractVersion || AUTH_RESULT_CONTRACT_VERSION}`,
    `total=${Number(metrics.total || 0)}`,
    `passed=${Number(metrics.byStatus?.passed || 0)}`,
    `failed=${Number(metrics.byStatus?.failed || 0)}`,
    `inconclusive=${Number(metrics.byStatus?.inconclusive || 0)}`,
    `shapeValidRate=${Number(metrics.shapeValidRate || 0).toFixed(4)}`,
    `defaultCredentialDetectedRate=${Number(metrics.defaultCredentialDetectedRate || 0).toFixed(4)}`,
    `weakPasswordDetectedRate=${Number(metrics.weakPasswordDetectedRate || 0).toFixed(4)}`,
    `CREDENTIAL_HYGIENE_V1|profiles=${Number(report?.totalProfiles || 0)}|mode=${report?.mode || 'n/a'}`,
  ].join(' | ');
}
