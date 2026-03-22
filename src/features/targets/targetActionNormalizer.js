function safeUrlParse(value) {
  try {
    return new URL(value);
  } catch {
    return null;
  }
}

function extractHostLike(raw) {
  const text = String(raw || '').trim();
  if (!text) return '';
  const withScheme = /^[a-z][a-z0-9+.-]*:\/\//i.test(text) ? text : `http://${text}`;
  const parsed = safeUrlParse(withScheme);
  if (!parsed) return text.replace(/\/+$/, '');
  return parsed.host || text;
}

function toWebUrl(rawHostOrUrl) {
  const text = String(rawHostOrUrl || '').trim();
  if (!text) return '';
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(text)) return text;
  return `http://${text}`;
}

export function normalizeTargetForLinkedAction(target, actionType = 'generic') {
  const base = target && typeof target === 'object' ? { ...target } : {};
  const rawEndpoint = base.host || base.ip || base.url || '';
  const normalizedHost = extractHostLike(rawEndpoint);
  const normalizedWebUrl = toWebUrl(rawEndpoint);

  const mode = String(actionType || 'generic');
  const webActions = new Set(['isapi_info', 'isapi_search', 'onvif_info', 'onvif_recordings', 'archive_endpoints', 'archive_search', 'hub_archive']);
  const streamActions = new Set(['stream']);

  if (webActions.has(mode)) {
    return {
      ...base,
      host: normalizedHost || base.host || base.ip || '',
      endpoint: normalizedWebUrl || base.endpoint || '',
      url: normalizedWebUrl || base.url || '',
    };
  }

  if (streamActions.has(mode)) {
    return {
      ...base,
      host: normalizedHost || base.host || base.ip || '',
      endpoint: normalizedHost || base.endpoint || '',
    };
  }

  return {
    ...base,
    host: normalizedHost || base.host || base.ip || '',
    endpoint: normalizedHost || base.endpoint || '',
  };
}

