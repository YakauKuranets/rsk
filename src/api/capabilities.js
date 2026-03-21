import { invoke } from '@tauri-apps/api/core';

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
