function unique(items = []) {
  return [...new Set(items.filter(Boolean))];
}

function toLowerText(value) {
  return String(value || '').trim().toLowerCase();
}

function collectStatusCodes(raw = {}) {
  const pages = Array.isArray(raw?.pages) ? raw.pages : [];
  const dirs = Array.isArray(raw?.dirResults) ? raw.dirResults : [];
  return [...pages, ...dirs]
    .map((x) => Number(x?.statusCode))
    .filter((n) => Number.isFinite(n));
}

export function deriveSpiderAuthBoundaryHintsV1(surfaceResult = {}, raw = {}) {
  const endpoints = Array.isArray(surfaceResult?.web_endpoints) ? surfaceResult.web_endpoints : [];
  const services = Array.isArray(surfaceResult?.services) ? surfaceResult.services : [];
  const moduleStatuses = Array.isArray(raw?.moduleStatuses) ? raw.moduleStatuses : [];
  const statusCodes = collectStatusCodes(raw);
  const reachable = Boolean(surfaceResult?.reachable);

  const endpointText = endpoints.map((x) => toLowerText(x)).join(' | ');
  const serviceText = services
    .map((s) => toLowerText(s?.service || s?.name || s))
    .filter(Boolean)
    .join(' | ');

  const authModuleSignals = moduleStatuses
    .filter((m) => /credential|auth|session/i.test(String(m?.name || m?.module || '')))
    .map((m) => `${toLowerText(m?.name || m?.module)}:${toLowerText(m?.status || 'unknown')}`);

  const hasAuthPathWords = /(login|signin|auth|session|token|admin)/i.test(endpointText);
  const hasProtectedResponses = statusCodes.some((c) => c === 401 || c === 403);
  const hasPublicResponses = statusCodes.some((c) => c === 200);
  const hasMgmtServiceExposure = /(telnet|ftp|rtsp|onvif|isapi)/i.test(serviceText);

  const hints = [];
  if (hasAuthPathWords || hasProtectedResponses || authModuleSignals.length > 0) {
    hints.push('likely_auth_required');
  }

  if ((hasPublicResponses && hasMgmtServiceExposure) || (hasProtectedResponses && hasPublicResponses)) {
    hints.push('partial_exposure_possible');
  }

  if ((hasAuthPathWords && hasPublicResponses) || (hasProtectedResponses && hasPublicResponses)) {
    hints.push('boundary_ambiguous');
  }

  if (!reachable || (endpoints.length === 0 && services.length === 0 && statusCodes.length === 0)) {
    hints.push('insufficient_signal');
  }

  const normalizedHints = unique(hints);
  const evidenceRefs = unique([
    `auth_hints_v1:auth_module_signals=${authModuleSignals.length}`,
    `auth_hints_v1:protected_codes=${statusCodes.filter((c) => c === 401 || c === 403).length}`,
    `auth_hints_v1:public_codes=${statusCodes.filter((c) => c === 200).length}`,
    `auth_hints_v1:endpoint_auth_words=${hasAuthPathWords ? 1 : 0}`,
    `auth_hints_v1:mgmt_exposure=${hasMgmtServiceExposure ? 1 : 0}`,
  ]);

  const confidenceDelta = Math.min(0.08, normalizedHints.length * 0.015 + (hasProtectedResponses ? 0.02 : 0));

  return {
    hints: normalizedHints,
    evidenceRefs,
    confidenceDelta,
  };
}

function clampConfidence(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) return 0;
  return Math.max(0, Math.min(1, Number(n.toFixed(4))));
}

export function applySpiderAuthBoundaryHintsV1(surfaceResult = {}, raw = {}) {
  const derived = deriveSpiderAuthBoundaryHintsV1(surfaceResult, raw);
  return {
    surfaceResult: {
      ...surfaceResult,
      auth_boundary_hints: unique([
        ...(Array.isArray(surfaceResult?.auth_boundary_hints) ? surfaceResult.auth_boundary_hints : []),
        ...derived.hints,
      ]),
      evidenceRefs: unique([
        ...(Array.isArray(surfaceResult?.evidenceRefs) ? surfaceResult.evidenceRefs : []),
        ...derived.evidenceRefs,
      ]),
      confidence: clampConfidence(Number(surfaceResult?.confidence || 0) + derived.confidenceDelta),
    },
    authBoundaryHints: derived,
  };
}

export function formatSpiderAuthBoundaryHintsV1Marker(derived = {}, targetId = 'n/a') {
  const hints = Array.isArray(derived?.hints) ? derived.hints : [];
  return [
    'SPIDER_AUTH_BOUNDARY_HINTS_V1',
    `target=${targetId || 'n/a'}`,
    `hints=${hints.length}`,
    `likelyAuth=${hints.includes('likely_auth_required')}`,
    `partial=${hints.includes('partial_exposure_possible')}`,
    `ambiguous=${hints.includes('boundary_ambiguous')}`,
    `insufficient=${hints.includes('insufficient_signal')}`,
    `confidenceDelta=${Number(derived?.confidenceDelta || 0).toFixed(4)}`,
  ].join('|');
}
