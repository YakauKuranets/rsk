export const PORT_SCAN_RESULT_CONTRACT_VERSION = 'port_scan_result_v1';

const DEFAULT_PORT_SCAN_RESULT = {
  contractVersion: PORT_SCAN_RESULT_CONTRACT_VERSION,
  target_id: null,
  host: null,
  reachable: false,
  open_ports: [],
  services: [],
  protocol: null,
  banner: null,
  vendor_hints: [],
  evidenceRefs: [],
  confidence: 0,
  resultClass: 'inconclusive',
};

function clampConfidence(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) return 0;
  return Math.max(0, Math.min(1, Number(n.toFixed(4))));
}

function safeClass(value) {
  return ['passed', 'failed', 'inconclusive'].includes(value) ? value : 'inconclusive';
}

function toStringArray(value) {
  if (!Array.isArray(value)) return [];
  return value.map((x) => String(x ?? '').trim()).filter(Boolean);
}

function unique(items = []) {
  return [...new Set(items)];
}

function normalizeOpenPorts(input = []) {
  const ports = (Array.isArray(input) ? input : [])
    .map((item) => {
      if (typeof item === 'number') return item;
      return Number(item?.port ?? item);
    })
    .filter((n) => Number.isFinite(n) && n > 0 && n <= 65535)
    .map((n) => Math.trunc(n));
  return unique(ports).sort((a, b) => a - b);
}

function inferProtocol(rows = []) {
  const protocols = unique(
    (Array.isArray(rows) ? rows : [])
      .map((r) => String(r?.protocol || '').trim().toLowerCase())
      .filter(Boolean),
  );
  if (protocols.length === 0) return null;
  if (protocols.length === 1) return protocols[0];
  return 'mixed';
}

function inferBanner(rows = []) {
  const banners = (Array.isArray(rows) ? rows : [])
    .map((r) => String(r?.banner || r?.product || r?.version || '').trim())
    .filter(Boolean);
  return banners[0] || null;
}

function inferVendorHints(rows = []) {
  const hints = (Array.isArray(rows) ? rows : [])
    .flatMap((r) => [r?.vendor, r?.manufacturer, r?.product, r?.service])
    .map((x) => String(x || '').trim())
    .filter(Boolean)
    .slice(0, 20)
    .map((x) => `hint:${x}`);
  return unique(hints);
}

export function normalizePortScanResultV1(input = {}) {
  return {
    ...DEFAULT_PORT_SCAN_RESULT,
    ...input,
    contractVersion: PORT_SCAN_RESULT_CONTRACT_VERSION,
    target_id: input?.target_id ? String(input.target_id) : null,
    host: input?.host ? String(input.host) : null,
    reachable: Boolean(input?.reachable),
    open_ports: normalizeOpenPorts(input?.open_ports),
    services: unique(toStringArray(input?.services)),
    protocol: input?.protocol ? String(input.protocol) : null,
    banner: input?.banner ? String(input.banner) : null,
    vendor_hints: unique(toStringArray(input?.vendor_hints)),
    evidenceRefs: unique(toStringArray(input?.evidenceRefs)),
    confidence: clampConfidence(input?.confidence),
    resultClass: safeClass(input?.resultClass),
  };
}

export function normalizeScanHostPortsResultV1(raw, options = {}) {
  const rows = Array.isArray(raw) ? raw : [];
  const host = String(options?.host || '').trim() || null;

  const openPorts = normalizeOpenPorts(rows);
  const services = unique(
    rows
      .map((r) => String(r?.service || r?.name || '').trim())
      .filter(Boolean),
  );

  const reachable = openPorts.length > 0 || rows.some((r) => Boolean(r?.alive || r?.responding));
  const resultClass = openPorts.length > 0
    ? 'passed'
    : rows.length === 0
      ? 'failed'
      : 'inconclusive';

  const evidenceRefs = unique([
    host ? `host:${host}` : null,
    `rows:${rows.length}`,
    `open_ports:${openPorts.length}`,
    ...rows.slice(0, 10).map((r) => `port:${Number(r?.port || 0)}:${String(r?.service || 'unknown')}`),
  ].filter(Boolean));

  const confidence = reachable
    ? Math.max(0.25, Math.min(0.95, openPorts.length * 0.12 + services.length * 0.08))
    : rows.length === 0
      ? 0.1
      : 0.3;

  return normalizePortScanResultV1({
    target_id: host,
    host,
    reachable,
    open_ports: openPorts,
    services,
    protocol: inferProtocol(rows),
    banner: inferBanner(rows),
    vendor_hints: inferVendorHints(rows),
    evidenceRefs,
    confidence,
    resultClass,
  });
}

export function validatePortScanResultV1Shape(input = {}) {
  const requiredKeys = [
    'contractVersion',
    'target_id',
    'host',
    'reachable',
    'open_ports',
    'services',
    'protocol',
    'banner',
    'vendor_hints',
    'evidenceRefs',
    'confidence',
    'resultClass',
  ];

  const missingKeys = requiredKeys.filter((k) => !Object.prototype.hasOwnProperty.call(input, k));
  const openPortsValid = Array.isArray(input?.open_ports) && input.open_ports.every((p) => Number.isFinite(Number(p)));
  const servicesValid = Array.isArray(input?.services);
  const hintsValid = Array.isArray(input?.vendor_hints) && Array.isArray(input?.evidenceRefs);
  const confidenceValid = typeof input?.confidence === 'number' && Number.isFinite(input.confidence) && input.confidence >= 0 && input.confidence <= 1;
  const classValid = ['passed', 'failed', 'inconclusive'].includes(input?.resultClass);

  return {
    ok:
      missingKeys.length === 0 &&
      input?.contractVersion === PORT_SCAN_RESULT_CONTRACT_VERSION &&
      openPortsValid &&
      servicesValid &&
      hintsValid &&
      confidenceValid &&
      classValid,
    missingKeys,
  };
}

export function formatPortScanResultV1Marker(result = {}) {
  return [
    'PORT_SCAN_RESULT_V1',
    `target=${result?.target_id || 'n/a'}`,
    `host=${result?.host || 'n/a'}`,
    `reachable=${Boolean(result?.reachable)}`,
    `class=${result?.resultClass || 'inconclusive'}`,
    `open_ports=${Array.isArray(result?.open_ports) ? result.open_ports.length : 0}`,
    `services=${Array.isArray(result?.services) ? result.services.length : 0}`,
    `confidence=${Number(result?.confidence || 0).toFixed(4)}`,
  ].join('|');
}
