import {
  PASSIVE_OBSERVATION_RESULT_CONTRACT_VERSION,
  formatPassiveObservationCompactSummaryV1,
  normalizePassiveObservationResultV1,
  validatePassiveObservationResultV1Shape,
} from './passiveObservationResultContract';

export const PASSIVE_TRAFFIC_BASELINE_PACK_VERSION = 'passive_traffic_baseline_pack_v1';

export const DEFAULT_PASSIVE_TRAFFIC_BASELINE_CASES_V1 = [
  {
    caseId: 'passive_known_good_expected_outbound_https',
    category: 'known-good',
    expectedClass: 'passed',
    surfaceInput: {
      target_id: 'passive-good-1',
      host: 'passive-good.local',
      services: [{ port: 443, service: 'https', protocol: 'tcp' }],
    },
    rawTraffic: {
      interface: 'eth0',
      durationSecs: 30,
      packetsCaptured: 16,
      capturedCredentials: [
        { protocol: 'tcp', destIp: '1.1.1.1', destPort: 443 },
        { protocol: 'tcp', destIp: '1.0.0.1', destPort: 443 },
      ],
      unencryptedProtocols: [],
      warnings: ['passive_capture_only'],
    },
  },
  {
    caseId: 'passive_known_bad_unexpected_high_port',
    category: 'known-bad',
    expectedClass: 'failed',
    surfaceInput: {
      target_id: 'passive-bad-1',
      host: 'passive-bad.local',
      services: [{ port: 554, service: 'rtsp', protocol: 'tcp' }],
    },
    rawTraffic: {
      interface: 'eth0',
      durationSecs: 30,
      packetsCaptured: 24,
      capturedCredentials: [
        { protocol: 'tcp', destIp: '203.0.113.10', destPort: 31337 },
        { protocol: 'tcp', destIp: '203.0.113.10', destPort: 31337 },
        { protocol: 'tcp', destIp: '203.0.113.11', destPort: 2323 },
      ],
      unencryptedProtocols: ['telnet'],
      warnings: ['passive_capture_only'],
    },
  },
  {
    caseId: 'passive_ambiguous_single_packet',
    category: 'ambiguous',
    expectedClass: 'inconclusive',
    surfaceInput: {
      target_id: 'passive-ambiguous-1',
      host: 'passive-ambiguous.local',
      services: [],
    },
    rawTraffic: {
      interface: 'eth0',
      durationSecs: 30,
      packetsCaptured: 1,
      capturedCredentials: [{ protocol: 'tcp', destIp: '198.51.100.3', destPort: 443 }],
      unencryptedProtocols: [],
      warnings: ['passive_capture_only'],
    },
  },
];

function safeClass(value) {
  return ['passed', 'failed', 'inconclusive'].includes(value) ? value : 'inconclusive';
}

function buildCaseReport(caseDef = {}) {
  const observation = normalizePassiveObservationResultV1(caseDef?.rawTraffic || {}, {
    targetId: caseDef?.surfaceInput?.target_id,
    host: caseDef?.surfaceInput?.host,
    surfaceScanResult: caseDef?.surfaceInput || {},
    observationWindowSec: caseDef?.rawTraffic?.durationSecs,
    interface: caseDef?.rawTraffic?.interface,
  });

  const expectedClass = safeClass(caseDef?.expectedClass);
  const classMatch = safeClass(observation?.resultClass) === expectedClass;
  const hasPassiveContract =
    observation?.contractVersion === PASSIVE_OBSERVATION_RESULT_CONTRACT_VERSION &&
    validatePassiveObservationResultV1Shape(observation).ok;

  const correlation = observation?.service_correlation || {};
  const unexpectedCount = Array.isArray(observation?.unexpected_communication)
    ? observation.unexpected_communication.length
    : 0;
  const correlationConsistent =
    Number(unexpectedCount) ===
    (Array.isArray(correlation?.unexpected_ports) ? correlation.unexpected_ports.length : 0);

  const status =
    classMatch && hasPassiveContract && correlationConsistent
      ? 'passed'
      : !hasPassiveContract || !correlationConsistent
        ? 'failed'
        : 'inconclusive';

  return {
    caseId: caseDef?.caseId || 'passive_case_unknown',
    category: caseDef?.category || 'ambiguous',
    status,
    expected: {
      class: expectedClass,
    },
    actual: {
      class: safeClass(observation?.resultClass),
      confidence: observation?.confidence || 'low',
    },
    checks: {
      classMatch,
      hasPassiveContract,
      correlationConsistency: correlationConsistent,
    },
    marker: formatPassiveObservationCompactSummaryV1(observation),
    observation,
  };
}

export function passiveObservationMetrics(caseReports = []) {
  const total = caseReports.length;
  const byStatus = { passed: 0, failed: 0, inconclusive: 0 };
  const byCategory = { 'known-good': 0, 'known-bad': 0, ambiguous: 0, other: 0 };

  for (const item of caseReports) {
    byStatus[item.status] = Number(byStatus[item.status] || 0) + 1;
    const cat = ['known-good', 'known-bad', 'ambiguous'].includes(item.category)
      ? item.category
      : 'other';
    byCategory[cat] = Number(byCategory[cat] || 0) + 1;
  }

  const rate = (n) => (total ? Number((n / total).toFixed(4)) : 0);
  const classMatchCount = caseReports.filter((x) => x?.checks?.classMatch).length;
  const contractCount = caseReports.filter((x) => x?.checks?.hasPassiveContract).length;
  const correlationCount = caseReports.filter((x) => x?.checks?.correlationConsistency).length;

  return {
    total,
    byStatus,
    byCategory,
    classMatchRate: rate(classMatchCount),
    passiveContractCoverageRate: rate(contractCount),
    correlationConsistencyRate: rate(correlationCount),
  };
}

export async function runPassiveTrafficBaselinePackV1({
  cases = DEFAULT_PASSIVE_TRAFFIC_BASELINE_CASES_V1,
  includeContinuity = true,
} = {}) {
  const normalizedCases = (Array.isArray(cases) ? cases : []).map((item, idx) => ({
    caseId: item?.caseId || `passive_traffic_case_${idx + 1}`,
    category: item?.category || 'ambiguous',
    expectedClass: safeClass(item?.expectedClass),
    surfaceInput: item?.surfaceInput || {},
    rawTraffic: item?.rawTraffic || {},
  }));

  const caseReports = normalizedCases.map((item) => buildCaseReport(item));
  const metrics = passiveObservationMetrics(caseReports);

  return {
    packId: PASSIVE_TRAFFIC_BASELINE_PACK_VERSION,
    passiveContractVersion: PASSIVE_OBSERVATION_RESULT_CONTRACT_VERSION,
    createdAt: new Date().toISOString(),
    caseReports,
    metrics,
    continuity: includeContinuity
      ? {
          readyForNextStep: true,
          nextStepHint: 'phase31_2_coverage_matrix_v1',
        }
      : null,
  };
}

export function formatPassiveTrafficBaselineCompactSummaryV1(report = {}) {
  const metrics = report?.metrics || {};
  return [
    `packId=${report?.packId || PASSIVE_TRAFFIC_BASELINE_PACK_VERSION}`,
    `passiveContractVersion=${report?.passiveContractVersion || PASSIVE_OBSERVATION_RESULT_CONTRACT_VERSION}`,
    `total=${Number(metrics?.total || 0)}`,
    `passed=${Number(metrics?.byStatus?.passed || 0)}`,
    `failed=${Number(metrics?.byStatus?.failed || 0)}`,
    `inconclusive=${Number(metrics?.byStatus?.inconclusive || 0)}`,
    `classMatchRate=${Number(metrics?.classMatchRate || 0).toFixed(4)}`,
    `contractCoverageRate=${Number(metrics?.passiveContractCoverageRate || 0).toFixed(4)}`,
    `correlationConsistencyRate=${Number(metrics?.correlationConsistencyRate || 0).toFixed(4)}`,
    `PASSIVE_TRAFFIC_BASELINE_V1|cases=${Number(metrics?.total || 0)}`,
  ].join(' | ');
}
