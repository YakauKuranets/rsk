import {
  probeStreamPreferred,
  shouldStopStreamOnProbe,
  verifySessionCookieFlagsCapability,
} from './capabilities';
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

function statusDistribution(events) {
  const byStatus = {
    reviewer_rejected: 0,
    capability_succeeded: 0,
    capability_failed: 0,
    unknown: 0,
  };
  for (const event of events) {
    const status = String(event?.finalStatus || 'unknown');
    if (Object.prototype.hasOwnProperty.call(byStatus, status)) {
      byStatus[status] += 1;
    } else {
      byStatus.unknown += 1;
    }
  }
  return byStatus;
}

function buildCookieEvent(scenario, normalized) {
  return {
    scenario,
    capability: 'verify_session_cookie_flags',
    finalStatus: normalized?.finalStatus || 'unknown',
    ok: Boolean(normalized?.ok),
    secure:
      typeof normalized?.capabilityResultSummary?.secure === 'boolean'
        ? normalized.capabilityResultSummary.secure
        : null,
    issuesCount:
      typeof normalized?.capabilityResultSummary?.issuesCount === 'number'
        ? normalized.capabilityResultSummary.issuesCount
        : null,
    reviewerApproved: Boolean(normalized?.reviewerVerdict?.approved),
    runId: normalized?.runId || null,
    reporterSummary: normalized?.reporterSummary || null,
  };
}

export function aggregateCookieEvalMetrics(events) {
  const total = events.length;
  const finalStatusDistribution = statusDistribution(events);
  const reviewerRejected = finalStatusDistribution.reviewer_rejected;
  const failedOrInconclusive = finalStatusDistribution.capability_failed + finalStatusDistribution.unknown;

  const succeeded = events.filter((e) => e.finalStatus === 'capability_succeeded');
  const secureCount = succeeded.filter((e) => e.secure === true).length;
  const issuesDetectedCount = succeeded.filter((e) => Number(e.issuesCount || 0) > 0).length;

  return {
    total,
    finalStatusDistribution,
    reviewerRejectRate: toRate(reviewerRejected, total),
    secureRate: toRate(secureCount, succeeded.length),
    issuesDetectedRate: toRate(issuesDetectedCount, succeeded.length),
    inconclusiveFailureRate: toRate(failedOrInconclusive, total),
  };
}

export function validateCookieResultInvariants(result) {
  const violations = [];
  const issues = result?.issues;
  const issuesCount = result?.issuesCount;
  const source = String(result?.source || '');
  const fallbackUsed = Boolean(result?.fallbackUsed);
  const inconclusive = Boolean(result?.inconclusive);

  if (!Array.isArray(issues) || !issues.every((x) => typeof x === 'string')) {
    violations.push('issues_must_be_string_array');
  }
  if (Number(issuesCount) !== (Array.isArray(issues) ? issues.length : 0)) {
    violations.push('issues_count_must_match_issues_length');
  }
  if (fallbackUsed && source === 'minimal-agent') {
    violations.push('fallback_used_conflicts_with_minimal_agent_source');
  }
  if (!fallbackUsed && source !== 'minimal-agent' && source !== 'client-validation') {
    violations.push('non_fallback_source_must_be_minimal_or_client_validation');
  }
  if (inconclusive && result?.ok) {
    violations.push('inconclusive_conflicts_with_ok_true');
  }
  if (inconclusive && source === 'minimal-agent') {
    violations.push('inconclusive_conflicts_with_confident_minimal_agent_source');
  }

  return {
    ok: violations.length === 0,
    violations,
  };
}

export async function runCookieResultInvariantChecks({
  target = 'https://localhost',
  mode = 'discovery_mode',
} = {}) {
  const preferred = await verifySessionCookieFlagsCapability(target, mode);
  const fallback = await verifySessionCookieFlagsCapability(target, mode, {
    forceLegacyFallback: true,
  });

  const preferredCheck = validateCookieResultInvariants(preferred);
  const fallbackCheck = validateCookieResultInvariants(fallback);
  const requiredKeys = [
    'ok',
    'source',
    'secure',
    'issues',
    'issuesCount',
    'runId',
    'reporterSummary',
    'evidenceRefs',
    'fallbackUsed',
    'inconclusive',
    'contractVersion',
  ];
  const preferredShapeOk = requiredKeys.every((k) => Object.prototype.hasOwnProperty.call(preferred, k));
  const fallbackShapeOk = requiredKeys.every((k) => Object.prototype.hasOwnProperty.call(fallback, k));

  return {
    preferred,
    fallback,
    preferredCheck,
    fallbackCheck,
    shapeCompatible: preferredShapeOk && fallbackShapeOk,
    allPassed: preferredCheck.ok && fallbackCheck.ok && preferredShapeOk && fallbackShapeOk,
  };
}

async function runCookieScenario({
  scenario,
  targetId,
  mode,
  permitVerifySessionCookieFlags,
}) {
  const normalized = await runAgentMinimal({
    targetId,
    mode,
    preferredCapability: 'verify_session_cookie_flags',
    verifySessionCookieFlagsIpOrUrl: targetId,
    permitProbeStream: false,
    permitVerifySessionCookieFlags,
  });
  return buildCookieEvent(scenario, normalized);
}

export async function runVerifySessionCookieEvalHarness({
  secureTarget = 'https://localhost',
  issuesTarget = 'http://localhost',
  unreachableTarget = 'http://127.0.0.1:1',
  mode = 'discovery_mode',
} = {}) {
  const events = [];

  events.push(
    await runCookieScenario({
      scenario: 'reviewer_rejected_when_cookie_permit_false',
      targetId: secureTarget,
      mode,
      permitVerifySessionCookieFlags: false,
    }),
  );

  events.push(
    await runCookieScenario({
      scenario: 'cookie_check_success_path',
      targetId: secureTarget,
      mode,
      permitVerifySessionCookieFlags: true,
    }),
  );

  events.push(
    await runCookieScenario({
      scenario: 'cookie_check_issues_detected_path',
      targetId: issuesTarget,
      mode,
      permitVerifySessionCookieFlags: true,
    }),
  );

  events.push(
    await runCookieScenario({
      scenario: 'cookie_check_unreachable_or_failed_path',
      targetId: unreachableTarget,
      mode,
      permitVerifySessionCookieFlags: true,
    }),
  );

  const invariantChecks = await runCookieResultInvariantChecks({
    target: secureTarget,
    mode,
  });

  return {
    events,
    metrics: aggregateCookieEvalMetrics(events),
    invariants: invariantChecks,
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

function mergeStatusDistributionFromEvents(events) {
  return statusDistribution(events);
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

export async function runVerifySessionCookieEvalSnapshot({
  inputs = [],
  mode = 'discovery_mode',
  includeCaseMetrics = true,
} = {}) {
  const normalizedInputs = inputs.map((item, idx) => ({
    caseId: item?.caseId || `cookie_case_${idx + 1}`,
    secureTarget: item?.secureTarget || 'https://localhost',
    issuesTarget: item?.issuesTarget || 'http://localhost',
    unreachableTarget: item?.unreachableTarget || 'http://127.0.0.1:1',
    mode: item?.mode || mode,
  }));

  const caseReports = [];
  const events = [];
  for (const input of normalizedInputs) {
    const report = await runVerifySessionCookieEvalHarness({
      secureTarget: input.secureTarget,
      issuesTarget: input.issuesTarget,
      unreachableTarget: input.unreachableTarget,
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

  const metrics = aggregateCookieEvalMetrics(events);
  metrics.finalStatusDistribution = mergeStatusDistributionFromEvents(events);

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

export function compareVerifySessionCookieEvalSnapshots(baseSnapshot, nextSnapshot) {
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
    reviewerRejectRateDelta:
      Number(nextMetrics.reviewerRejectRate || 0) - Number(baseMetrics.reviewerRejectRate || 0),
    secureRateDelta: Number(nextMetrics.secureRate || 0) - Number(baseMetrics.secureRate || 0),
    issuesDetectedRateDelta:
      Number(nextMetrics.issuesDetectedRate || 0) - Number(baseMetrics.issuesDetectedRate || 0),
    inconclusiveFailureRateDelta:
      Number(nextMetrics.inconclusiveFailureRate || 0) -
      Number(baseMetrics.inconclusiveFailureRate || 0),
  };
}
