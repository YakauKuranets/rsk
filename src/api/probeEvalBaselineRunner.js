import {
  compareVerifySessionCookieEvalSnapshots,
  compareProbeEvalSnapshots,
  DEFAULT_SESSION_LIFECYCLE_KNOWN_BAD_PACK_V1,
  runSessionLifecycleKnownBadPackV1,
  runVerifySessionCookieEvalSnapshot,
  runProbeStreamEvalSnapshot,
} from './probeEvalHarness';

const EPS = 0.0001;

export const DEFAULT_PROBE_EVAL_BASELINE_INPUTS = [
  { caseId: 'baseline_case_1', aliveTargetId: 'baseline_alive_1', deadTargetId: 'baseline_dead_1' },
  { caseId: 'baseline_case_2', aliveTargetId: 'baseline_alive_2', deadTargetId: 'baseline_dead_2' },
];

export const DEFAULT_COOKIE_EVAL_BASELINE_INPUTS = [
  {
    caseId: 'cookie_baseline_case_1',
    secureTarget: 'https://localhost',
    issuesTarget: 'http://localhost',
    unreachableTarget: 'http://127.0.0.1:1',
  },
];

export const COOKIE_BASELINE_PROFILES = {
  local_tls: {
    profileId: 'local_tls',
    description:
      'Assumes local HTTPS endpoint with secure cookie flags and a local HTTP endpoint for contrast.',
    assumptions: [
      'https://localhost is reachable and uses cookies in session flow',
      'http://localhost is reachable for non-secure contrast',
      '127.0.0.1:1 stays unreachable for failure path',
    ],
    inputs: [
      {
        caseId: 'cookie_local_tls_case',
        secureTarget: 'https://localhost',
        issuesTarget: 'http://localhost',
        unreachableTarget: 'http://127.0.0.1:1',
      },
    ],
  },
  local_plain_http: {
    profileId: 'local_plain_http',
    description:
      'Assumes local plain HTTP is primary target; useful to observe issuesDetected and failure behavior.',
    assumptions: [
      'http://localhost is reachable',
      'https://localhost may be unavailable (allowed)',
      '127.0.0.1:1 stays unreachable for failure path',
    ],
    inputs: [
      {
        caseId: 'cookie_local_plain_http_case',
        secureTarget: 'http://localhost',
        issuesTarget: 'http://localhost',
        unreachableTarget: 'http://127.0.0.1:1',
      },
    ],
  },
};

export function resolveCookieBaselineProfile(profileName = 'local_tls') {
  const profile = COOKIE_BASELINE_PROFILES[profileName];
  if (profile) return profile;
  return COOKIE_BASELINE_PROFILES.local_tls;
}

function makeBaselineId() {
  return `probe_baseline_${Date.now()}`;
}

export function buildProbeEvalBaseline(snapshot, { baselineId } = {}) {
  return {
    baselineId: baselineId || makeBaselineId(),
    capabilityMode: snapshot?.capabilityMode || 'probe_stream',
    sourceSnapshot: {
      snapshotId: snapshot?.snapshotId || null,
      createdAt: snapshot?.createdAt || null,
      inputs: Array.isArray(snapshot?.inputs) ? snapshot.inputs : [],
      profileId: snapshot?.profileId || null,
    },
    metrics: snapshot?.metrics || null,
    contractHealth: snapshot?.contractHealth || null,
    createdAt: new Date().toISOString(),
  };
}

function classifyDelta(delta) {
  if (!delta) return 'inconclusive';

  const fallback = Number(delta.fallbackRateDelta || 0);
  const semantic = Number(delta.semanticAliveKnownRateDelta || 0);
  const mismatch = Number(delta.mismatchDelta || 0);
  const statusDelta = delta.statusDistributionDelta || {};
  const succeededDelta = Number(statusDelta.capability_succeeded || 0);
  const failedDelta = Number(statusDelta.capability_failed || 0);

  const improvedSignals = [fallback < -EPS, semantic > EPS, mismatch < 0, succeededDelta > 0, failedDelta < 0].filter(Boolean).length;
  const regressedSignals = [fallback > EPS, semantic < -EPS, mismatch > 0, succeededDelta < 0, failedDelta > 0].filter(Boolean).length;

  if (improvedSignals === 0 && regressedSignals === 0) return 'unchanged';
  if (improvedSignals > 0 && regressedSignals === 0) return 'improved';
  if (regressedSignals > 0 && improvedSignals === 0) return 'regressed';
  return 'inconclusive';
}

function classifyCookieDelta(delta) {
  if (!delta) return 'inconclusive';
  const secure = Number(delta.secureRateDelta || 0);
  const issues = Number(delta.issuesDetectedRateDelta || 0);
  const reviewerReject = Number(delta.reviewerRejectRateDelta || 0);
  const inconclusiveFail = Number(delta.inconclusiveFailureRateDelta || 0);

  const improvedSignals = [secure > EPS, issues > EPS, reviewerReject < -EPS, inconclusiveFail < -EPS].filter(Boolean).length;
  const regressedSignals = [secure < -EPS, issues < -EPS, reviewerReject > EPS, inconclusiveFail > EPS].filter(Boolean).length;

  if (improvedSignals === 0 && regressedSignals === 0) return 'unchanged';
  if (improvedSignals > 0 && regressedSignals === 0) return 'improved';
  if (regressedSignals > 0 && improvedSignals === 0) return 'regressed';
  return 'inconclusive';
}

export function summarizeProbeEvalDelta(delta, classification) {
  if (!delta) return 'No delta available.';

  return [
    `classification=${classification}`,
    `fallbackRateDelta=${Number(delta.fallbackRateDelta || 0).toFixed(4)}`,
    `semanticAliveKnownRateDelta=${Number(delta.semanticAliveKnownRateDelta || 0).toFixed(4)}`,
    `mismatchDelta=${Number(delta.mismatchDelta || 0)}`,
    `statusDelta=${JSON.stringify(delta.statusDistributionDelta || {})}`,
  ].join(' | ');
}

function summarizeCookieEvalDelta(delta, classification) {
  if (!delta) return 'No delta available.';
  return [
    `classification=${classification}`,
    `secureRateDelta=${Number(delta.secureRateDelta || 0).toFixed(4)}`,
    `issuesDetectedRateDelta=${Number(delta.issuesDetectedRateDelta || 0).toFixed(4)}`,
    `reviewerRejectRateDelta=${Number(delta.reviewerRejectRateDelta || 0).toFixed(4)}`,
    `inconclusiveFailureRateDelta=${Number(delta.inconclusiveFailureRateDelta || 0).toFixed(4)}`,
    `invariantsPassedBaseline=${delta.invariantsPassedBaseline === true ? 'yes' : 'no'}`,
    `invariantsPassedSnapshot=${delta.invariantsPassedSnapshot === true ? 'yes' : 'no'}`,
    `statusDelta=${JSON.stringify(delta.statusDistributionDelta || {})}`,
  ].join(' | ');
}

function getCookieInvariantHealth(snapshot, baseline) {
  const snapshotPassed = snapshot?.contractHealth?.invariantsPassed === true;
  const baselinePassed = baseline?.contractHealth?.invariantsPassed === true;
  return {
    snapshotPassed,
    baselinePassed,
    safeToCompare: snapshotPassed && baselinePassed,
  };
}

export function compareSnapshotAgainstBaseline(snapshot, baseline) {
  if (!snapshot?.metrics || !baseline?.metrics) {
    return {
      classification: 'inconclusive',
      delta: null,
      summary: 'comparison is inconclusive: missing snapshot/baseline metrics',
    };
  }

  const baselineSnapshotLike = {
    snapshotId: baseline?.sourceSnapshot?.snapshotId || baseline?.baselineId || null,
    metrics: baseline.metrics,
    contractHealth: baseline?.contractHealth || null,
  };

  const isCookieMode = (baseline?.capabilityMode || snapshot?.capabilityMode) === 'verify_session_cookie_flags';
  const delta = isCookieMode
    ? compareVerifySessionCookieEvalSnapshots(baselineSnapshotLike, snapshot)
    : compareProbeEvalSnapshots(baselineSnapshotLike, snapshot);
  const metricClassification = isCookieMode ? classifyCookieDelta(delta) : classifyDelta(delta);
  const invariantHealth = isCookieMode ? getCookieInvariantHealth(snapshot, baseline) : null;
  const classification =
    isCookieMode && invariantHealth && !invariantHealth.safeToCompare
      ? 'inconclusive'
      : metricClassification;
  const summaryCore = isCookieMode
    ? summarizeCookieEvalDelta(delta, classification)
    : summarizeProbeEvalDelta(delta, classification);
  const summary =
    isCookieMode && invariantHealth && !invariantHealth.safeToCompare
      ? `${summaryCore} | warning=invariant_contract_failed_compare_unsafe`
      : summaryCore;

  return {
    classification,
    delta,
    summary,
    warning:
      isCookieMode && invariantHealth && !invariantHealth.safeToCompare
        ? 'invariant_contract_failed_compare_unsafe'
        : null,
    invariantHealth,
    baselineId: baseline.baselineId,
    snapshotId: snapshot.snapshotId,
  };
}

export async function runProbeEvalBaselineRunner({
  capabilityMode = 'probe_stream',
  inputs = DEFAULT_PROBE_EVAL_BASELINE_INPUTS,
  cookieProfile = 'local_tls',
  mode = 'discovery_mode',
  baseline = null,
} = {}) {
  const cookieProfileResolved = resolveCookieBaselineProfile(cookieProfile);
  const snapshot =
    capabilityMode === 'verify_session_cookie_flags'
      ? await runVerifySessionCookieEvalSnapshot({
          inputs: inputs.length ? inputs : cookieProfileResolved?.inputs || DEFAULT_COOKIE_EVAL_BASELINE_INPUTS,
          mode,
        })
      : await runProbeStreamEvalSnapshot({ inputs, mode });
  snapshot.capabilityMode = capabilityMode;
  snapshot.profileId =
    capabilityMode === 'verify_session_cookie_flags' ? cookieProfileResolved?.profileId : null;
  const baselineRecord = baseline || buildProbeEvalBaseline(snapshot);
  const comparison = compareSnapshotAgainstBaseline(snapshot, baselineRecord);

  return {
    snapshot,
    baseline: baselineRecord,
    comparison,
    profile: capabilityMode === 'verify_session_cookie_flags' ? cookieProfileResolved : null,
  };
}

export function formatProbeEvalBaselineCompactReport(runResult) {
  const snapshot = runResult?.snapshot || {};
  const baseline = runResult?.baseline || {};
  const comparison = runResult?.comparison || {};
  const metrics = snapshot?.metrics || {};
  const capabilityMode = snapshot?.capabilityMode || baseline?.capabilityMode || 'probe_stream';

  const keyMetrics =
    capabilityMode === 'verify_session_cookie_flags'
      ? `secureRate:${Number(metrics.secureRate || 0).toFixed(4)},issuesDetectedRate:${Number(
          metrics.issuesDetectedRate || 0,
        ).toFixed(4)},reviewerRejectRate:${Number(metrics.reviewerRejectRate || 0).toFixed(
          4,
        )},inconclusiveFailureRate:${Number(metrics.inconclusiveFailureRate || 0).toFixed(4)},invariantsPassed:${
          snapshot?.contractHealth?.invariantsPassed === true ? 'yes' : 'no'
        }`
      : `fallbackRate:${Number(metrics.fallbackRate || 0).toFixed(4)},semanticAliveKnownRate:${Number(
          metrics.semanticAliveKnownRate || 0,
        ).toFixed(4)},reviewerRejectRate:${Number(metrics.reviewerRejectRate || 0).toFixed(
          4,
        )},mismatchCount:${Number(metrics.mismatchIndications?.length || 0)}`;

  const lines = [
    `capabilityMode=${capabilityMode}`,
    `snapshotId=${snapshot.snapshotId || 'n/a'}`,
    `baselineId=${baseline.baselineId || 'n/a'}`,
    `classification=${comparison.classification || 'inconclusive'}`,
    `warning=${comparison.warning || 'none'}`,
    `summary=${comparison.summary || 'n/a'}`,
    `keyMetrics=${keyMetrics}`,
  ];

  return lines.join('\n');
}

export { DEFAULT_SESSION_LIFECYCLE_KNOWN_BAD_PACK_V1 };

export async function runSessionLifecycleKnownBadPackV1Runner({
  mode = 'discovery_mode',
  cases = DEFAULT_SESSION_LIFECYCLE_KNOWN_BAD_PACK_V1,
} = {}) {
  const pack = await runSessionLifecycleKnownBadPackV1({ mode, cases });
  const compact = [
    `packId=${pack.packId}`,
    `mode=${pack.mode}`,
    `totalCases=${pack.totalCases}`,
    `passed=${pack.passed}`,
    `failed=${pack.failed}`,
    `inconclusive=${pack.inconclusive}`,
    `successRate=${Number(pack.successRate || 0).toFixed(4)}`,
  ].join(' | ');

  return {
    compact,
    ...pack,
  };
}
