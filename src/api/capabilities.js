import { invoke } from '@tauri-apps/api/core';

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
    const res = await invoke('execute_capability', {
      req: {
        capability: 'verify_session_cookie_flags',
        mode,
        verifySessionCookieFlags: { ipOrUrl: target },
      },
    });

    if (res?.ok && res?.data?.type === 'verifySessionCookieFlags') {
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

    return {
      ok: false,
      source: 'capability',
      message: 'Unexpected capability response shape',
      secure: false,
      issues: [],
    };
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
