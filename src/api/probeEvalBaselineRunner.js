import {
  compareVerifySessionCookieEvalSnapshots,
  compareProbeEvalSnapshots,
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
    },
    metrics: snapshot?.metrics || null,
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
    `statusDelta=${JSON.stringify(delta.statusDistributionDelta || {})}`,
  ].join(' | ');
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
  };

  const isCookieMode = (baseline?.capabilityMode || snapshot?.capabilityMode) === 'verify_session_cookie_flags';
  const delta = isCookieMode
    ? compareVerifySessionCookieEvalSnapshots(baselineSnapshotLike, snapshot)
    : compareProbeEvalSnapshots(baselineSnapshotLike, snapshot);
  const classification = isCookieMode ? classifyCookieDelta(delta) : classifyDelta(delta);
  const summary = isCookieMode
    ? summarizeCookieEvalDelta(delta, classification)
    : summarizeProbeEvalDelta(delta, classification);

  return {
    classification,
    delta,
    summary,
    baselineId: baseline.baselineId,
    snapshotId: snapshot.snapshotId,
  };
}

export async function runProbeEvalBaselineRunner({
  capabilityMode = 'probe_stream',
  inputs = DEFAULT_PROBE_EVAL_BASELINE_INPUTS,
  mode = 'discovery_mode',
  baseline = null,
} = {}) {
  const snapshot =
    capabilityMode === 'verify_session_cookie_flags'
      ? await runVerifySessionCookieEvalSnapshot({
          inputs: inputs.length ? inputs : DEFAULT_COOKIE_EVAL_BASELINE_INPUTS,
          mode,
        })
      : await runProbeStreamEvalSnapshot({ inputs, mode });
  snapshot.capabilityMode = capabilityMode;
  const baselineRecord = baseline || buildProbeEvalBaseline(snapshot);
  const comparison = compareSnapshotAgainstBaseline(snapshot, baselineRecord);

  return {
    snapshot,
    baseline: baselineRecord,
    comparison,
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
        )},inconclusiveFailureRate:${Number(metrics.inconclusiveFailureRate || 0).toFixed(4)}`
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
    `summary=${comparison.summary || 'n/a'}`,
    `keyMetrics=${keyMetrics}`,
  ];

  return lines.join('\n');
}
