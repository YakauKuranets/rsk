import {
  SURFACE_SCAN_RESULT_CONTRACT_VERSION,
  normalizeSurfaceScanResultV1,
} from './surfaceScanResultContract';

export const PASSIVE_OBSERVATION_RESULT_CONTRACT_VERSION = 'passive_observation_result_v1';

function clampWindow(value) {
  const n = Number(value);
  if (!Number.isFinite(n) || n <= 0) return 30;
  return Math.min(3600, Math.max(1, Math.round(n)));
}

function normalizeConfidence(value) {
  const v = String(value || '').trim().toLowerCase();
  if (['high', 'medium', 'low'].includes(v)) return v;
  return 'low';
}

function normalizeDirection(value) {
  const v = String(value || '').trim().toLowerCase();
  if (['inbound', 'outbound', 'lateral', 'unknown'].includes(v)) return v;
  return 'unknown';
}

function toEvidence(items = []) {
  if (!Array.isArray(items)) return [];
  return [...new Set(items.map((x) => String(x ?? '').trim()).filter(Boolean))];
}

function classifyConfidence({ packetsCaptured = 0, observedCount = 0, unexpectedCount = 0 }) {
  if (packetsCaptured >= 30 || observedCount >= 4 || unexpectedCount >= 2) return 'high';
  if (packetsCaptured >= 5 || observedCount >= 2) return 'medium';
  return 'low';
}

function deriveObservedServices(raw = {}) {
  const creds = Array.isArray(raw?.capturedCredentials) ? raw.capturedCredentials : [];
  const byKey = new Map();

  for (const item of creds) {
    const port = Number(item?.destPort);
    if (!Number.isFinite(port)) continue;
    const protocol = String(item?.protocol || 'tcp').toLowerCase();
    const direction = 'outbound';
    const key = `${port}|${protocol}|${direction}`;
    byKey.set(key, {
      port,
      protocol,
      direction,
      frequency: Number((byKey.get(key)?.frequency || 0) + 1),
    });
  }

  return [...byKey.values()].sort((a, b) => b.frequency - a.frequency || a.port - b.port);
}

function deriveOutboundHints(raw = {}) {
  const creds = Array.isArray(raw?.capturedCredentials) ? raw.capturedCredentials : [];
  return toEvidence(
    creds.map((x) => {
      const ip = String(x?.destIp || '').trim();
      const port = Number(x?.destPort);
      const protocol = String(x?.protocol || 'tcp').toLowerCase();
      if (!ip || !Number.isFinite(port)) return null;
      return `outbound:${ip}:${port}/${protocol}`;
    }),
  );
}

function correlateWithSurface(observedServices = [], surfaceScanResult = {}) {
  const surface = normalizeSurfaceScanResultV1(surfaceScanResult);
  const surfacePorts = new Set(
    (Array.isArray(surface?.services) ? surface.services : [])
      .map((x) => Number(x?.port))
      .filter((x) => Number.isFinite(x)),
  );
  const observedPorts = [...new Set(observedServices.map((x) => Number(x?.port)).filter(Number.isFinite))];

  const matchedPorts = observedPorts.filter((p) => surfacePorts.has(p));
  const unexpectedPorts = observedPorts.filter((p) => !surfacePorts.has(p));
  const missingSurfacePorts = [...surfacePorts].filter((p) => !observedPorts.includes(p));

  return {
    surfaceContractVersion: SURFACE_SCAN_RESULT_CONTRACT_VERSION,
    matched_ports: matchedPorts,
    unexpected_ports: unexpectedPorts,
    missing_surface_ports: missingSurfacePorts,
  };
}

function deriveUnexpectedCommunication(observedServices = [], correlation = {}) {
  const unexpectedPorts = Array.isArray(correlation?.unexpected_ports)
    ? correlation.unexpected_ports
    : [];

  return observedServices
    .filter((item) => unexpectedPorts.includes(Number(item?.port)))
    .map((item) => ({
      port: Number(item?.port),
      protocol: String(item?.protocol || 'tcp').toLowerCase(),
      direction: normalizeDirection(item?.direction),
      reason: 'port_not_seen_in_surface_scan_result_v1',
      frequency: Number(item?.frequency || 0),
    }));
}

function classifyResultClass({ unexpectedCount = 0, packetsCaptured = 0, observedCount = 0 }) {
  if (unexpectedCount > 0) return 'failed';
  if (packetsCaptured <= 1 && observedCount <= 1) return 'inconclusive';
  return 'passed';
}

export function normalizePassiveObservationResultV1(raw = {}, options = {}) {
  const surfaceScanResult = options?.surfaceScanResult || {};
  const targetId = String(options?.targetId || surfaceScanResult?.target_id || '').trim() || null;
  const host = String(options?.host || surfaceScanResult?.host || '').trim() || null;
  const interfaceName = String(raw?.interface || options?.interface || '').trim() || null;

  const observedServices = deriveObservedServices(raw).map((x) => ({
    port: Number(x?.port),
    protocol: String(x?.protocol || 'tcp').toLowerCase(),
    direction: normalizeDirection(x?.direction),
    frequency: Number(x?.frequency || 0),
  }));
  const serviceCorrelation = correlateWithSurface(observedServices, surfaceScanResult);
  const unexpectedCommunication = deriveUnexpectedCommunication(observedServices, serviceCorrelation);
  const outboundHints = deriveOutboundHints(raw);

  const packetsCaptured = Number(raw?.packetsCaptured || 0);
  const observationWindowSec = clampWindow(raw?.durationSecs ?? options?.observationWindowSec ?? 30);
  const resultClass = classifyResultClass({
    unexpectedCount: unexpectedCommunication.length,
    packetsCaptured,
    observedCount: observedServices.length,
  });

  const confidence = normalizeConfidence(
    classifyConfidence({
      packetsCaptured,
      observedCount: observedServices.length,
      unexpectedCount: unexpectedCommunication.length,
    }),
  );

  return {
    contractVersion: PASSIVE_OBSERVATION_RESULT_CONTRACT_VERSION,
    target_id: targetId,
    host,
    interface: interfaceName,
    observed_services: observedServices,
    outbound_hints: outboundHints,
    unexpected_communication: unexpectedCommunication,
    service_correlation: serviceCorrelation,
    observation_window_sec: observationWindowSec,
    packets_captured: packetsCaptured,
    resultClass,
    confidence,
    evidence: toEvidence([
      `interface:${interfaceName || 'n/a'}`,
      `packets_captured:${packetsCaptured}`,
      `unencrypted_protocols:${Array.isArray(raw?.unencryptedProtocols) ? raw.unencryptedProtocols.length : 0}`,
      ...(Array.isArray(raw?.warnings) ? raw.warnings : []),
      ...outboundHints,
    ]),
    createdAt: new Date().toISOString(),
  };
}

export function validatePassiveObservationResultV1Shape(result = {}) {
  const requiredKeys = [
    'contractVersion',
    'target_id',
    'host',
    'interface',
    'observed_services',
    'outbound_hints',
    'unexpected_communication',
    'service_correlation',
    'observation_window_sec',
    'packets_captured',
    'resultClass',
    'confidence',
    'evidence',
    'createdAt',
  ];

  const missingKeys = requiredKeys.filter((k) => !Object.prototype.hasOwnProperty.call(result, k));

  return {
    ok:
      missingKeys.length === 0 &&
      result?.contractVersion === PASSIVE_OBSERVATION_RESULT_CONTRACT_VERSION &&
      Array.isArray(result?.observed_services) &&
      Array.isArray(result?.outbound_hints) &&
      Array.isArray(result?.unexpected_communication) &&
      typeof result?.service_correlation === 'object' &&
      ['passed', 'failed', 'inconclusive'].includes(result?.resultClass) &&
      ['high', 'medium', 'low'].includes(result?.confidence) &&
      Array.isArray(result?.evidence),
    missingKeys,
  };
}

export function formatPassiveObservationCompactSummaryV1(result = {}) {
  return [
    'PASSIVE_OBSERVATION_V1',
    `target=${result?.target_id || 'n/a'}`,
    `host=${result?.host || 'n/a'}`,
    `iface=${result?.interface || 'n/a'}`,
    `class=${result?.resultClass || 'inconclusive'}`,
    `confidence=${result?.confidence || 'low'}`,
    `packets=${Number(result?.packets_captured || 0)}`,
    `observedServices=${Array.isArray(result?.observed_services) ? result.observed_services.length : 0}`,
    `unexpected=${Array.isArray(result?.unexpected_communication) ? result.unexpected_communication.length : 0}`,
    `window=${Number(result?.observation_window_sec || 0)}s`,
  ].join('|');
}
