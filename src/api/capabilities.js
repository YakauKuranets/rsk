import { invoke } from '@tauri-apps/api/core';
import { runAgentMinimal } from './tauri';

export async function executeCapabilityRequest(req) {
  return invoke('execute_capability', { req });
}

export function validateCapabilityEnvelope(res, expectedType) {
  return Boolean(res?.ok && res?.data?.type === expectedType);
}

export function normalizeCapabilityError(res, fallbackMessage = 'Unexpected capability response shape') {
  return {
    ok: false,
    source: 'capability',
    message: res?.error?.message || fallbackMessage,
  };
}

export async function verifySessionCookieFlagsCapability(ipOrUrl, mode = 'discovery_mode') {
  const target = String(ipOrUrl || '').trim();
  if (!target) {
    return {
      ok: false,
      source: 'client-validation',
      message: 'ipOrUrl is empty',
      secure: false,
      issues: [],
    };
  }

  try {
    const agent = await runAgentMinimal({
      targetId: target,
      mode,
      preferredCapability: 'verify_session_cookie_flags',
      verifySessionCookieFlagsIpOrUrl: target,
      permitProbeStream: false,
      permitVerifySessionCookieFlags: true,
    });

    if (
      agent?.ok &&
      agent?.finalStatus === 'capability_succeeded' &&
      agent?.capabilityInvoked === 'verify_session_cookie_flags'
    ) {
      const rawData = agent?.raw?.capabilityResult?.data || {};
      const out =
        rawData?.verifySessionCookieFlags ||
        rawData?.verify_session_cookie_flags ||
        rawData?.verifySessionCookieflags ||
        {};

      const issues = Array.isArray(out?.issues) ? out.issues : [];
      const secure =
        typeof agent?.capabilityResultSummary?.secure === 'boolean'
          ? agent.capabilityResultSummary.secure
          : issues.length === 0;

      return {
        ok: true,
        source: 'minimal-agent',
        secure,
        issues,
        issuesCount:
          typeof agent?.capabilityResultSummary?.issuesCount === 'number'
            ? agent.capabilityResultSummary.issuesCount
            : issues.length,
        runId: agent.runId || null,
        reporterSummary: agent.reporterSummary || null,
        evidenceRefs: Array.isArray(agent.evidenceRefs) ? agent.evidenceRefs : [],
      };
    }
  } catch (_) {
    // fall through to legacy path
  }

  return verifySessionCookieFlagsLegacyCapability(target, mode);
}

async function verifySessionCookieFlagsLegacyCapability(target, mode = 'discovery_mode') {
  if (!target) {
    return {
      ok: false,
      source: 'client-validation',
      message: 'ipOrUrl is empty',
      secure: false,
      issues: [],
    };
  }

  try {
    const res = await executeCapabilityRequest({
      capability: 'verify_session_cookie_flags',
      mode,
      verifySessionCookieFlags: { ipOrUrl: target },
    });

    if (validateCapabilityEnvelope(res, 'verifySessionCookieFlags')) {
      const out = res.data.verifySessionCookieFlags || {};
      return {
        ok: true,
        source: 'capability',
        secure: Boolean(out.secure),
        issues: Array.isArray(out.issues) ? out.issues : [],
        evidenceRefs: Array.isArray(out.evidenceRefs) ? out.evidenceRefs : [],
      };
    }

    if (res?.error?.message) {
      const fallback = await invoke('check_session_security', { ip: target });
      return {
        ok: true,
        source: 'legacy-fallback',
        secure: String(fallback).includes('выглядят безопасно'),
        issues: String(fallback).replace('[SESSION_AUDIT] ', '').split(' | ').filter(Boolean),
        legacyText: String(fallback),
      };
    }

    return { ...normalizeCapabilityError(res), secure: false, issues: [] };
  } catch (_) {
    try {
      const fallback = await invoke('check_session_security', { ip: target });
      return {
        ok: true,
        source: 'legacy-fallback',
        secure: String(fallback).includes('выглядят безопасно'),
        issues: String(fallback).replace('[SESSION_AUDIT] ', '').split(' | ').filter(Boolean),
        legacyText: String(fallback),
      };
    } catch (fallbackError) {
      return {
        ok: false,
        source: 'error',
        message: String(fallbackError),
        secure: false,
        issues: [],
      };
    }
  }
}

export async function probeStreamCapability(targetId, mode = 'discovery_mode') {
  const target = String(targetId || '').trim();
  if (!target) {
    return {
      ok: false,
      source: 'client-validation',
      message: 'targetId is empty',
      alive: false,
      evidenceRefs: [],
    };
  }

  try {
    const res = await executeCapabilityRequest({
      capability: 'probe_stream',
      mode,
      probeStream: { targetId: target },
    });

    if (validateCapabilityEnvelope(res, 'probeStream')) {
      const out = res.data.probeStream || {};
      return {
        ok: true,
        source: 'capability',
        alive: Boolean(out.alive),
        targetId: out.targetId || target,
        evidenceRefs: Array.isArray(out.evidenceRefs) ? out.evidenceRefs : [],
      };
    }

    return { ...normalizeCapabilityError(res), alive: false, evidenceRefs: [] };
  } catch (error) {
    return {
      ok: false,
      source: 'error',
      message: String(error),
      alive: false,
      evidenceRefs: [],
    };
  }
}

// DEPRECATION BOUNDARY:
// - `probeStreamCapability` is a low-level/legacy helper kept for fallback compatibility.
// - UI/workflow consumers should prefer `probeStreamPreferred`, which uses runAgentMinimal first.
export function shouldStopStreamOnProbe(probe) {
  if (!probe?.semanticAliveKnown) return false;
  return !Boolean(probe?.alive);
}

export async function probeStreamPreferred(targetId, mode = 'discovery_mode', options = {}) {
  const target = String(targetId || '').trim();
  const forceLegacyFallback = Boolean(options?.forceLegacyFallback);
  if (!target) {
    return {
      ok: false,
      source: 'client-validation',
      message: 'targetId is empty',
      alive: false,
      runId: null,
      finalStatus: 'capability_failed',
      reporterSummary: null,
      semanticAliveKnown: false,
      fallbackUsed: false,
      evidenceRefs: [],
    };
  }

  try {
    if (forceLegacyFallback) {
      throw new Error('forced-legacy-fallback');
    }

    const agent = await runAgentMinimal({
      targetId: target,
      mode,
      permitProbeStream: true,
    });

    if (!agent?.ok) {
      throw new Error((agent?.errors || []).join('; ') || 'minimal-agent-envelope-invalid');
    }

    if (agent.finalStatus === 'reviewer_rejected') {
      throw new Error('minimal-agent-reviewer-rejected');
    }
    const alive = agent.finalStatus === 'capability_succeeded' && Boolean(agent.capabilityResultSummary?.alive);
    return {
      ok: agent.finalStatus === 'capability_succeeded',
      source: 'minimal-agent',
      alive,
      targetId: agent.targetId || target,
      runId: agent.runId || null,
      finalStatus: agent.finalStatus,
      reporterSummary: agent.reporterSummary || null,
      reviewerApproved: Boolean(agent.reviewerVerdict?.approved),
      plannerActionCount: Number(agent.plannerDecision?.actionCount ?? 0),
      semanticAliveKnown: true,
      fallbackUsed: false,
      evidenceRefs: Array.isArray(agent.evidenceRefs) ? agent.evidenceRefs : [],
    };
  } catch (_) {
    const fallback = await probeStreamCapability(target, mode);
    return {
      ...fallback,
      source: fallback.source || 'legacy-probe-fallback',
      runId: null,
      finalStatus: fallback.ok ? 'capability_succeeded' : 'capability_failed',
      reporterSummary: fallback.message || null,
      reviewerApproved: null,
      plannerActionCount: null,
      semanticAliveKnown: fallback.source === 'capability',
      fallbackUsed: true,
    };
  }
}
