import {
  compareProbeEvalSnapshots,
  runProbeStreamEvalSnapshot,
} from './probeEvalHarness';

const EPS = 0.0001;

export const DEFAULT_PROBE_EVAL_BASELINE_INPUTS = [
  { caseId: 'baseline_case_1', aliveTargetId: 'baseline_alive_1', deadTargetId: 'baseline_dead_1' },
  { caseId: 'baseline_case_2', aliveTargetId: 'baseline_alive_2', deadTargetId: 'baseline_dead_2' },
];

function makeBaselineId() {
  return `probe_baseline_${Date.now()}`;
}

export function buildProbeEvalBaseline(snapshot, { baselineId } = {}) {
  return {
    baselineId: baselineId || makeBaselineId(),
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

  const delta = compareProbeEvalSnapshots(baselineSnapshotLike, snapshot);
  const classification = classifyDelta(delta);
  const summary = summarizeProbeEvalDelta(delta, classification);

  return {
    classification,
    delta,
    summary,
    baselineId: baseline.baselineId,
    snapshotId: snapshot.snapshotId,
  };
}

export async function runProbeEvalBaselineRunner({
  inputs = DEFAULT_PROBE_EVAL_BASELINE_INPUTS,
  mode = 'discovery_mode',
  baseline = null,
} = {}) {
  const snapshot = await runProbeStreamEvalSnapshot({ inputs, mode });
  const baselineRecord = baseline || buildProbeEvalBaseline(snapshot);
  const comparison = compareSnapshotAgainstBaseline(snapshot, baselineRecord);

  return {
    snapshot,
    baseline: baselineRecord,
    comparison,
  };
}
