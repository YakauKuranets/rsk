export const SPIDER_EVIDENCE_REPORT_VERSION = 'spider_evidence_report_v1';

function toStringArray(value) {
  if (!Array.isArray(value)) return [];
  return value.map((x) => String(x ?? '').trim()).filter(Boolean);
}

function unique(items = []) {
  return [...new Set(items.filter(Boolean))];
}

function compactService(service) {
  const port = Number(service?.port);
  const name = String(service?.service || service?.name || 'unknown').trim() || 'unknown';
  const protocol = String(service?.protocol || '').trim();
  return {
    port: Number.isFinite(port) ? port : null,
    service: name,
    protocol: protocol || null,
    label: `${Number.isFinite(port) ? port : 'n/a'}/${name}${protocol ? `/${protocol}` : ''}`,
  };
}

export function buildSpiderEvidenceReportV1({
  surfaceScanResult,
  raw = {},
} = {}) {
  const surface = surfaceScanResult || {};
  const services = (Array.isArray(surface?.services) ? surface.services : []).map(compactService);
  const vendorHints = toStringArray(surface?.vendor_hints);
  const streamHints = toStringArray(surface?.stream_hints);
  const archiveHints = toStringArray(surface?.archive_hints);
  const authBoundaryHints = toStringArray(surface?.auth_boundary_hints);
  const evidenceRefs = toStringArray(surface?.evidenceRefs);
  const webEndpoints = toStringArray(surface?.web_endpoints);

  const fpHints = vendorHints.filter((x) => x.startsWith('fp_'));
  const likelyVendorHints = vendorHints.filter((x) => x.startsWith('vendor:') || x.startsWith('fp_vendor:'));

  const signalScore =
    services.length * 0.2 +
    webEndpoints.length * 0.1 +
    fpHints.length * 0.08 +
    authBoundaryHints.length * 0.08 +
    evidenceRefs.length * 0.03;

  const signalStrength = signalScore >= 1.2 ? 'strong' : signalScore >= 0.6 ? 'moderate' : 'weak';

  const limitations = unique([
    !surface?.reachable ? 'target_unreachable_or_unstable' : null,
    webEndpoints.length === 0 ? 'no_web_endpoint_signal' : null,
    services.length === 0 ? 'no_service_signal' : null,
    authBoundaryHints.includes('insufficient_signal') ? 'auth_boundary_signal_insufficient' : null,
  ]);

  return {
    reportVersion: SPIDER_EVIDENCE_REPORT_VERSION,
    target_id: surface?.target_id || null,
    host: surface?.host || null,
    createdAt: new Date().toISOString(),
    surfaceSummary: {
      reachable: Boolean(surface?.reachable),
      resultClass: surface?.resultClass || 'inconclusive',
      confidence: Number(surface?.confidence || 0),
      signalStrength,
      totalServices: services.length,
      totalWebEndpoints: webEndpoints.length,
      totalEvidenceRefs: evidenceRefs.length,
    },
    findings: {
      serviceFindings: services,
      vendorModelHints: {
        inferred: fpHints,
        explicit: likelyVendorHints,
      },
      streamArchiveHints: {
        stream: streamHints,
        archive: archiveHints,
      },
      authBoundaryHints,
    },
    support: {
      evidenceRefs,
      moduleStatuses: Array.isArray(raw?.moduleStatuses)
        ? raw.moduleStatuses.map((m) => ({
            module: String(m?.module || m?.name || 'unknown'),
            status: String(m?.status || 'unknown'),
          }))
        : [],
      pagesCrawled: Number(raw?.pagesCrawled || 0),
    },
    limitations,
  };
}

export function validateSpiderEvidenceReportV1Shape(report = {}) {
  const required = [
    'reportVersion',
    'target_id',
    'host',
    'createdAt',
    'surfaceSummary',
    'findings',
    'support',
    'limitations',
  ];
  const missingKeys = required.filter((k) => !Object.prototype.hasOwnProperty.call(report, k));
  const summary = report?.surfaceSummary || {};

  return {
    ok:
      missingKeys.length === 0 &&
      report?.reportVersion === SPIDER_EVIDENCE_REPORT_VERSION &&
      typeof summary?.reachable === 'boolean' &&
      ['passed', 'failed', 'inconclusive'].includes(summary?.resultClass) &&
      Array.isArray(report?.findings?.serviceFindings) &&
      Array.isArray(report?.findings?.authBoundaryHints) &&
      Array.isArray(report?.support?.evidenceRefs) &&
      Array.isArray(report?.limitations),
    missingKeys,
  };
}

export function formatSpiderEvidenceCompactSummaryV1(report = {}) {
  const summary = report?.surfaceSummary || {};
  return [
    'SPIDER_EVIDENCE_REPORT_V1',
    `target=${report?.target_id || 'n/a'}`,
    `host=${report?.host || 'n/a'}`,
    `class=${summary?.resultClass || 'inconclusive'}`,
    `reachable=${Boolean(summary?.reachable)}`,
    `signal=${summary?.signalStrength || 'weak'}`,
    `services=${Number(summary?.totalServices || 0)}`,
    `webEndpoints=${Number(summary?.totalWebEndpoints || 0)}`,
    `authHints=${Array.isArray(report?.findings?.authBoundaryHints) ? report.findings.authBoundaryHints.length : 0}`,
    `evidence=${Number(summary?.totalEvidenceRefs || 0)}`,
  ].join('|');
}
