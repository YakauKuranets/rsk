import { probeStreamPreferred, shouldStopStreamOnProbe } from './capabilities';
import { runAgentMinimal } from './tauri';

function toRate(count, total) {
  if (!total) return 0;
  return Number((count / total).toFixed(4));
}

export function aggregateProbeEvalMetrics(events) {
  const total = events.length;
  const byStatus = {
    reviewer_rejected: 0,
    capability_succeeded: 0,
    capability_failed: 0,
    unknown: 0,
  };
  let reviewerRejected = 0;
  let fallbackUsed = 0;
  let semanticAliveKnown = 0;

  for (const e of events) {
    const status = String(e?.finalStatus || 'unknown');
    if (Object.prototype.hasOwnProperty.call(byStatus, status)) {
      byStatus[status] += 1;
    } else {
      byStatus.unknown += 1;
    }

    if (status === 'reviewer_rejected') reviewerRejected += 1;
    if (e?.fallbackUsed) fallbackUsed += 1;
    if (e?.semanticAliveKnown) semanticAliveKnown += 1;
  }

  const mismatchIndications = events
    .filter((e) => e.semanticAliveKnown && e.finalStatus === 'capability_succeeded' && !e.alive)
    .map((e) => ({ scenario: e.scenario, reason: 'status_alive_mismatch' }));

  return {
    total,
    finalStatusDistribution: byStatus,
    reviewerRejectRate: toRate(reviewerRejected, total),
    fallbackRate: toRate(fallbackUsed, total),
    semanticAliveKnownRate: toRate(semanticAliveKnown, total),
    mismatchIndications,
  };
}

function buildEvent(scenario, probe) {
  return {
    scenario,
    ok: Boolean(probe?.ok),
    finalStatus: probe?.finalStatus || 'unknown',
    alive: Boolean(probe?.alive),
    source: probe?.source || 'unknown',
    runId: probe?.runId || null,
    fallbackUsed: Boolean(probe?.fallbackUsed),
    semanticAliveKnown: Boolean(probe?.semanticAliveKnown),
    reporterSummary: probe?.reporterSummary || null,
    shouldStopStream: shouldStopStreamOnProbe(probe),
  };
}

export async function runProbeStreamEvalHarness({ aliveTargetId, deadTargetId, mode = 'discovery_mode' }) {
  const events = [];

  const reviewerRejectedRaw = await runAgentMinimal({
    targetId: aliveTargetId || deadTargetId || 'eval_probe_target',
    mode,
    permitProbeStream: false,
  });
  events.push(
    buildEvent('reviewer_rejected_when_permit_false', {
      finalStatus: reviewerRejectedRaw?.finalStatus,
      source: 'minimal-agent',
      runId: reviewerRejectedRaw?.runId || null,
      reporterSummary: reviewerRejectedRaw?.reporterSummary || null,
      semanticAliveKnown: false,
      fallbackUsed: false,
      alive: false,
      ok: false,
    }),
  );

  if (aliveTargetId) {
    const aliveProbe = await probeStreamPreferred(aliveTargetId, mode);
    events.push(buildEvent('capability_succeeded_on_alive_target', aliveProbe));

    const forcedFallback = await probeStreamPreferred(aliveTargetId, mode, {
      forceLegacyFallback: true,
    });
    events.push(buildEvent('fallback_path_behavior', forcedFallback));
  }

  if (deadTargetId) {
    const deadProbe = await probeStreamPreferred(deadTargetId, mode);
    events.push(buildEvent('capability_failed_on_dead_target', deadProbe));
  }

  const semanticUnknown = {
    ok: false,
    finalStatus: 'capability_failed',
    alive: false,
    source: 'synthetic-semantic-unknown',
    runId: null,
    fallbackUsed: false,
    semanticAliveKnown: false,
    reporterSummary: 'synthetic for semanticAliveKnown-sensitive behavior',
  };
  events.push(buildEvent('semanticAliveKnown_sensitive_behavior', semanticUnknown));

  return {
    events,
    metrics: aggregateProbeEvalMetrics(events),
  };
}

function makeSnapshotId() {
  const ts = Date.now();
  const rand = Math.floor(Math.random() * 1e6)
    .toString()
    .padStart(6, '0');
  return `probe_eval_${ts}_${rand}`;
}

function mergeStatusDistribution(reports) {
  const merged = {
    reviewer_rejected: 0,
    capability_succeeded: 0,
    capability_failed: 0,
    unknown: 0,
  };

  for (const report of reports) {
    const dist = report?.metrics?.finalStatusDistribution || {};
    for (const key of Object.keys(merged)) {
      merged[key] += Number(dist[key] || 0);
    }
  }
  return merged;
}

export async function runProbeStreamEvalSnapshot({
  inputs = [],
  mode = 'discovery_mode',
  includeCaseMetrics = true,
} = {}) {
  const normalizedInputs = inputs.map((item, idx) => ({
    caseId: item?.caseId || `case_${idx + 1}`,
    aliveTargetId: item?.aliveTargetId || null,
    deadTargetId: item?.deadTargetId || null,
    mode: item?.mode || mode,
  }));

  const caseReports = [];
  const events = [];
  for (const input of normalizedInputs) {
    const report = await runProbeStreamEvalHarness({
      aliveTargetId: input.aliveTargetId,
      deadTargetId: input.deadTargetId,
      mode: input.mode,
    });

    const caseEvents = report.events.map((event) => ({
      ...event,
      caseId: input.caseId,
    }));
    events.push(...caseEvents);
    caseReports.push({
      input,
      metrics: report.metrics,
      eventCount: caseEvents.length,
    });
  }

  const metrics = aggregateProbeEvalMetrics(events);
  metrics.finalStatusDistribution = mergeStatusDistribution(caseReports);

  return {
    snapshotId: makeSnapshotId(),
    createdAt: new Date().toISOString(),
    inputs: normalizedInputs,
    events,
    metrics,
    caseReports: includeCaseMetrics ? caseReports : undefined,
  };
}

function distributionDelta(baseDist, nextDist) {
  const keys = ['reviewer_rejected', 'capability_succeeded', 'capability_failed', 'unknown'];
  const delta = {};
  for (const key of keys) {
    delta[key] = Number(nextDist?.[key] || 0) - Number(baseDist?.[key] || 0);
  }
  return delta;
}

export function compareProbeEvalSnapshots(baseSnapshot, nextSnapshot) {
  const baseMetrics = baseSnapshot?.metrics || {};
  const nextMetrics = nextSnapshot?.metrics || {};

  return {
    baseSnapshotId: baseSnapshot?.snapshotId || null,
    nextSnapshotId: nextSnapshot?.snapshotId || null,
    createdAt: new Date().toISOString(),
    statusDistributionDelta: distributionDelta(
      baseMetrics.finalStatusDistribution,
      nextMetrics.finalStatusDistribution,
    ),
    fallbackRateDelta: Number(nextMetrics.fallbackRate || 0) - Number(baseMetrics.fallbackRate || 0),
    semanticAliveKnownRateDelta:
      Number(nextMetrics.semanticAliveKnownRate || 0) - Number(baseMetrics.semanticAliveKnownRate || 0),
    mismatchDelta:
      Number(nextMetrics.mismatchIndications?.length || 0) -
      Number(baseMetrics.mismatchIndications?.length || 0),
  };
}
