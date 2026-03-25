export const AUTH_RESULT_CONTRACT_VERSION = 'auth_result_v1';

const DEFAULT_AUTH_RESULT = {
  contractVersion: AUTH_RESULT_CONTRACT_VERSION,
  target_id: null,
  auth_path_type: 'unknown',
  auth_required: true,
  weak_password_detected: false,
  default_credential_detected: false,
  auth_boundary_strength: 'unknown',
  partial_access_detected: false,
  issues: [],
  issuesCount: 0,
  evidenceRefs: [],
  confidence: 0,
  resultClass: 'inconclusive',
};

function toStringArray(value) {
  if (!Array.isArray(value)) return [];
  return value.map((item) => String(item ?? '').trim()).filter(Boolean);
}

function clampConfidence(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) return 0;
  return Math.max(0, Math.min(1, Number(n.toFixed(4))));
}

function safeClass(value) {
  return ['passed', 'failed', 'inconclusive'].includes(value) ? value : 'inconclusive';
}

export function normalizeAuthResultV1(input = {}) {
  const issues = toStringArray(input.issues);
  const evidenceRefs = toStringArray(input.evidenceRefs);

  return {
    ...DEFAULT_AUTH_RESULT,
    ...input,
    contractVersion: AUTH_RESULT_CONTRACT_VERSION,
    target_id: input.target_id ? String(input.target_id) : null,
    auth_path_type: input.auth_path_type ? String(input.auth_path_type) : 'unknown',
    auth_required: typeof input.auth_required === 'boolean' ? input.auth_required : true,
    weak_password_detected: Boolean(input.weak_password_detected),
    default_credential_detected: Boolean(input.default_credential_detected),
    auth_boundary_strength: input.auth_boundary_strength ? String(input.auth_boundary_strength) : 'unknown',
    partial_access_detected: Boolean(input.partial_access_detected),
    issues,
    issuesCount: issues.length,
    evidenceRefs,
    confidence: clampConfidence(input.confidence),
    resultClass: safeClass(input.resultClass),
  };
}

export function normalizeSessionCookieAuthResultV1({
  targetId,
  cookieResult,
} = {}) {
  const secure = typeof cookieResult?.secure === 'boolean' ? cookieResult.secure : null;
  const issues = toStringArray(cookieResult?.issues);
  const inconclusive = Boolean(cookieResult?.inconclusive);

  const resultClass = inconclusive
    ? 'inconclusive'
    : secure === true
      ? 'passed'
      : secure === false
        ? 'failed'
        : 'inconclusive';

  const authBoundaryStrength = resultClass === 'passed'
    ? 'strong'
    : resultClass === 'failed'
      ? 'weak'
      : 'unknown';

  return normalizeAuthResultV1({
    target_id: targetId || null,
    auth_path_type: 'session_cookie_flags',
    auth_required: true,
    weak_password_detected: false,
    default_credential_detected: false,
    auth_boundary_strength: authBoundaryStrength,
    partial_access_detected: inconclusive,
    issues,
    evidenceRefs: [
      ...(Array.isArray(cookieResult?.evidenceRefs) ? cookieResult.evidenceRefs : []),
      `source:${cookieResult?.source || 'unknown'}`,
    ],
    confidence: inconclusive ? 0.4 : resultClass === 'passed' ? 0.78 : 0.7,
    resultClass,
  });
}

export function validateAuthResultV1Shape(input = {}) {
  const issuesValid = Array.isArray(input.issues) && input.issues.every((x) => typeof x === 'string');
  const evidenceRefsValid =
    Array.isArray(input.evidenceRefs) && input.evidenceRefs.every((x) => typeof x === 'string');
  const confidenceValid =
    typeof input.confidence === 'number' && Number.isFinite(input.confidence) && input.confidence >= 0 && input.confidence <= 1;

  const requiredKeys = [
    'contractVersion',
    'target_id',
    'auth_path_type',
    'auth_required',
    'weak_password_detected',
    'default_credential_detected',
    'auth_boundary_strength',
    'partial_access_detected',
    'issues',
    'issuesCount',
    'evidenceRefs',
    'confidence',
    'resultClass',
  ];

  const missingKeys = requiredKeys.filter((k) => !Object.prototype.hasOwnProperty.call(input, k));
  const issuesCountMatches = Number(input.issuesCount) === (Array.isArray(input.issues) ? input.issues.length : 0);
  const resultClassValid = ['passed', 'failed', 'inconclusive'].includes(input.resultClass);

  return {
    ok:
      missingKeys.length === 0 &&
      input.contractVersion === AUTH_RESULT_CONTRACT_VERSION &&
      issuesValid &&
      evidenceRefsValid &&
      confidenceValid &&
      issuesCountMatches &&
      resultClassValid,
    missingKeys,
  };
}
