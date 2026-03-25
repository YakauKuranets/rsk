export const SURFACE_SCAN_RESULT_CONTRACT_VERSION = 'surface_scan_result_v1';

const DEFAULT_SURFACE_SCAN_RESULT = {
  contractVersion: SURFACE_SCAN_RESULT_CONTRACT_VERSION,
  target_id: null,
  host: null,
  reachable: false,
  resultClass: 'inconclusive',
  services: [],
  web_endpoints: [],
  stream_hints: [],
  archive_hints: [],
  vendor_hints: [],
  auth_boundary_hints: [],
  evidenceRefs: [],
  confidence: 0,
};

function toStringArray(value) {
  if (!Array.isArray(value)) return [];
  return value.map((item) => String(item ?? '').trim()).filter(Boolean);
}

function uniqueBy(items = [], keyFn = (x) => x) {
  const seen = new Set();
  const out = [];
  for (const item of items) {
    const key = keyFn(item);
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(item);
  }
  return out;
}

function clampConfidence(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) return 0;
  return Math.max(0, Math.min(1, Number(n.toFixed(4))));
}

function safeClass(value) {
  return ['passed', 'failed', 'inconclusive'].includes(value) ? value : 'inconclusive';
}

function normalizeServiceItems(input = []) {
  const normalized = (Array.isArray(input) ? input : [])
    .map((item) => {
      const port = Number(item?.port);
      const service = String(item?.service || item?.name || '').trim();
      const protocol = String(item?.protocol || '').trim();
      if (!Number.isFinite(port) && !service) return null;
      return {
        port: Number.isFinite(port) ? port : null,
        service: service || 'unknown',
        protocol: protocol || null,
      };
    })
    .filter(Boolean);

  return uniqueBy(normalized, (x) => `${x.port ?? 'n'}|${x.service}|${x.protocol ?? 'n'}`);
}

function normalizeHintItems(input = []) {
  return uniqueBy(toStringArray(input));
}

function inferSurfaceFromSpiderRaw(raw = {}, targetIdHint = null) {
  const targetCard = raw?.targetCard || {};
  const discoveredTargets = Array.isArray(raw?.discoveredTargets) ? raw.discoveredTargets : [];
  const targetHost = String(targetCard?.host || targetIdHint || '').trim() || null;

  const targetCardServices = Array.isArray(targetCard?.openPorts) ? targetCard.openPorts : [];
  const discoveredServices = discoveredTargets.flatMap((t) =>
    Array.isArray(t?.openPorts)
      ? t.openPorts.map((p) => ({
          port: p?.port,
          service: p?.service,
          protocol: p?.protocol || null,
        }))
      : [],
  );

  const services = normalizeServiceItems([...targetCardServices, ...discoveredServices]);

  const jsEndpoints = toStringArray(raw?.jsEndpoints);
  const pages = (Array.isArray(raw?.pages) ? raw.pages : [])
    .map((p) => String(p?.url || p || '').trim())
    .filter(Boolean);
  const dirs = (Array.isArray(raw?.dirResults) ? raw.dirResults : [])
    .filter((d) => Number(d?.statusCode || 0) !== 404)
    .map((d) => String(d?.url || d?.path || '').trim())
    .filter(Boolean);
  const webEndpoints = normalizeHintItems([...jsEndpoints, ...pages, ...dirs]);

  const streamHints = normalizeHintItems([
    raw?.targetCard?.rtspStatus ? `rtsp_status:${raw.targetCard.rtspStatus}` : null,
    ...(Array.isArray(raw?.videoStreamInfo)
      ? raw.videoStreamInfo.map((v) => `${v?.protocol || 'stream'}:${v?.endpoint || 'unknown'}`)
      : []),
  ]);

  const archiveHints = normalizeHintItems([
    targetCard?.apiGuess ? `api_guess:${targetCard.apiGuess}` : null,
    ...dirs.filter((x) => /archive|isapi|onvif|record/i.test(x)).map((x) => `archive_path:${x}`),
  ]);

  const vendorHints = normalizeHintItems([
    targetCard?.vendorGuess ? `vendor:${targetCard.vendorGuess}` : null,
    ...toStringArray(raw?.techStack).map((x) => `tech:${x}`),
  ]);

  const authBoundaryHints = normalizeHintItems([
    ...(Array.isArray(raw?.moduleStatuses)
      ? raw.moduleStatuses
          .filter((m) => /credential|auth|session/i.test(String(m?.name || m?.module || '')))
          .map((m) => `module:${m?.name || m?.module}:${m?.status || 'unknown'}`)
      : []),
  ]);

  const evidenceRefs = normalizeHintItems([
    targetHost ? `target:${targetHost}` : null,
    `pages_crawled:${Number(raw?.pagesCrawled || 0)}`,
    `js_endpoints:${jsEndpoints.length}`,
    `dirs_non404:${dirs.length}`,
    `services:${services.length}`,
  ]);

  const reachable =
    Number(raw?.pagesCrawled || 0) > 0 || services.length > 0 || webEndpoints.length > 0 || discoveredTargets.length > 0;

  const resultClass = !reachable
    ? 'failed'
    : services.length > 0 || webEndpoints.length > 0
      ? 'passed'
      : 'inconclusive';

  const confidenceSignal =
    services.length * 0.12 +
    webEndpoints.length * 0.05 +
    vendorHints.length * 0.04 +
    streamHints.length * 0.04 +
    archiveHints.length * 0.04;

  return {
    target_id: targetIdHint || targetHost,
    host: targetHost,
    reachable,
    resultClass,
    services,
    web_endpoints: webEndpoints,
    stream_hints: streamHints,
    archive_hints: archiveHints,
    vendor_hints: vendorHints,
    auth_boundary_hints: authBoundaryHints,
    evidenceRefs,
    confidence: clampConfidence(reachable ? Math.max(0.2, confidenceSignal) : 0.05),
  };
}

export function normalizeSurfaceScanResultV1(input = {}) {
  const normalized = {
    ...DEFAULT_SURFACE_SCAN_RESULT,
    ...input,
    contractVersion: SURFACE_SCAN_RESULT_CONTRACT_VERSION,
    target_id: input?.target_id ? String(input.target_id) : null,
    host: input?.host ? String(input.host) : null,
    reachable: Boolean(input?.reachable),
    resultClass: safeClass(input?.resultClass),
    services: normalizeServiceItems(input?.services),
    web_endpoints: normalizeHintItems(input?.web_endpoints),
    stream_hints: normalizeHintItems(input?.stream_hints),
    archive_hints: normalizeHintItems(input?.archive_hints),
    vendor_hints: normalizeHintItems(input?.vendor_hints),
    auth_boundary_hints: normalizeHintItems(input?.auth_boundary_hints),
    evidenceRefs: normalizeHintItems(input?.evidenceRefs),
    confidence: clampConfidence(input?.confidence),
  };

  return normalized;
}

export function normalizeSpiderFullScanResultV1(raw = {}, options = {}) {
  const targetIdHint = String(options?.targetId || raw?.targetCard?.host || '').trim() || null;
  const inferred = inferSurfaceFromSpiderRaw(raw, targetIdHint);
  return normalizeSurfaceScanResultV1(inferred);
}

export function validateSurfaceScanResultV1Shape(input = {}) {
  const requiredKeys = [
    'contractVersion',
    'target_id',
    'host',
    'reachable',
    'resultClass',
    'services',
    'web_endpoints',
    'stream_hints',
    'archive_hints',
    'vendor_hints',
    'auth_boundary_hints',
    'evidenceRefs',
    'confidence',
  ];

  const missingKeys = requiredKeys.filter((k) => !Object.prototype.hasOwnProperty.call(input, k));
  const resultClassValid = ['passed', 'failed', 'inconclusive'].includes(input?.resultClass);
  const servicesValid = Array.isArray(input?.services);
  const arraysValid = [
    input?.web_endpoints,
    input?.stream_hints,
    input?.archive_hints,
    input?.vendor_hints,
    input?.auth_boundary_hints,
    input?.evidenceRefs,
  ].every(Array.isArray);
  const confidenceValid =
    typeof input?.confidence === 'number' && Number.isFinite(input.confidence) && input.confidence >= 0 && input.confidence <= 1;

  return {
    ok:
      missingKeys.length === 0 &&
      input?.contractVersion === SURFACE_SCAN_RESULT_CONTRACT_VERSION &&
      resultClassValid &&
      servicesValid &&
      arraysValid &&
      confidenceValid,
    missingKeys,
  };
}

export function formatSurfaceScanResultV1Marker(result = {}) {
  return [
    'SURFACE_SCAN_RESULT_V1',
    `target=${result?.target_id || 'n/a'}`,
    `host=${result?.host || 'n/a'}`,
    `reachable=${Boolean(result?.reachable)}`,
    `class=${result?.resultClass || 'inconclusive'}`,
    `services=${Array.isArray(result?.services) ? result.services.length : 0}`,
    `webEndpoints=${Array.isArray(result?.web_endpoints) ? result.web_endpoints.length : 0}`,
    `confidence=${Number(result?.confidence || 0).toFixed(4)}`,
  ].join('|');
}
