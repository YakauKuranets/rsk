import {
  DEFAULT_COOKIE_EVAL_BASELINE_INPUTS,
  DEFAULT_PROBE_EVAL_BASELINE_INPUTS,
  DEFAULT_SESSION_LIFECYCLE_KNOWN_BAD_PACK_V1,
  formatProbeEvalBaselineCompactReport,
  runProbeEvalBaselineRunner,
  runSessionLifecycleKnownBadPackV1Runner,
} from '../api/probeEvalBaselineRunner';
import {
  DEFAULT_ARCHIVE_BASELINE_PACK_V1_CASES,
  formatArchiveBaselineCompactSummary,
  runArchiveBaselinePackV1,
} from '../api/archiveBaselinePack';
import {
  DEFAULT_ARCHIVE_EDGE_CASES_V1,
  formatArchiveEdgeCaseCompactSummary,
  runArchiveEdgeCaseHarnessV1,
} from '../api/archiveEdgeCaseHarness';
import {
  DEFAULT_ARCHIVE_FUZZ_SEEDS_V1,
  formatArchiveFuzzCompactSummary,
  runSafeArchiveFuzzLayerV1,
} from '../api/archiveSafeFuzzLayer';
import {
  DEFAULT_CREDENTIAL_HYGIENE_PROFILES_V1,
  DEFAULT_CREDENTIAL_HYGIENE_EXPANDED_PROFILES_V1,
  formatCredentialHygieneCompactSummary,
  runCredentialHygieneAuditorV1,
} from '../api/credentialHygieneAuditor';
import {
  DEFAULT_CREDENTIAL_HYGIENE_BASELINE_CASES_V1,
  formatCredentialHygieneBaselineCompactSummary,
  runCredentialHygieneBaselinePackV1,
} from '../api/credentialHygieneBaselinePack';
import {
  DEFAULT_PORT_SCAN_AUDIT_BASELINE_CASES_V1,
  formatPortScanAuditBaselineCompactSummary,
  runPortScanAuditBaselinePackV1,
} from '../api/portScanAuditBaselinePack';

import {
  DEFAULT_SPIDER_BASELINE_CASES_V1,
  formatSpiderBaselineCompactSummaryV1,
  runSpiderBaselinePackV1,
} from '../api/spiderBaselinePack';


import {
  DEFAULT_PASSIVE_TRAFFIC_BASELINE_CASES_V1,
  formatPassiveTrafficBaselineCompactSummaryV1,
  runPassiveTrafficBaselinePackV1,
} from '../api/passiveTrafficBaselinePack';
import {
  auditHostPortsNormalized,
  scanHostPortsNormalized,
  spiderFullScanNormalized,
  runPassiveTrafficAnalyzerV1,
  runKvDualWriteDiagnostic,
  runKvReadAnalyticsV1,
} from '../api/tauri';

async function runFromDevConsole({
  inputs,
  mode = 'discovery_mode',
  capabilityMode = 'probe_stream',
  cookieProfile = 'local_tls',
  baseline = null,
} = {}) {
  const defaultInputs =
    capabilityMode === 'verify_session_cookie_flags'
      ? DEFAULT_COOKIE_EVAL_BASELINE_INPUTS
      : DEFAULT_PROBE_EVAL_BASELINE_INPUTS;
  const resolvedInputs = Array.isArray(inputs) && inputs.length > 0 ? inputs : defaultInputs;
  const result = await runProbeEvalBaselineRunner({
    inputs: resolvedInputs,
    mode,
    capabilityMode,
    cookieProfile,
    baseline,
  });

  const compact = formatProbeEvalBaselineCompactReport(result);
  // Compact stdout-like report in browser dev console.
  console.log('[PROBE_EVAL_BASELINE_REPORT]\n' + compact);
  return {
    compact,
    ...result,
  };
}

export function registerProbeEvalBaselineEntrypoint() {
  if (typeof window === 'undefined') return;
  if (window.__runProbeEvalBaseline) return;

  window.__runProbeEvalBaseline = runFromDevConsole;
  window.__runSessionLifecycleKnownBadPackV1 = async ({
    mode = 'discovery_mode',
    cases = DEFAULT_SESSION_LIFECYCLE_KNOWN_BAD_PACK_V1,
  } = {}) => {
    const result = await runSessionLifecycleKnownBadPackV1Runner({ mode, cases });
    console.log('[SESSION_LIFECYCLE_KNOWN_BAD_PACK_V1]\n' + result.compact);
    console.table(result.reports);
    return result;
  };
  window.__runArchiveBaselinePackV1 = async ({
    cases = DEFAULT_ARCHIVE_BASELINE_PACK_V1_CASES,
  } = {}) => {
    const result = await runArchiveBaselinePackV1({ cases });
    const compact = formatArchiveBaselineCompactSummary(result);
    console.log('[ARCHIVE_BASELINE_PACK_V1]\n' + compact);
    console.table(result.caseReports);
    return {
      compact,
      ...result,
    };
  };
  window.__runArchiveEdgeCaseHarnessV1 = async ({
    cases = DEFAULT_ARCHIVE_EDGE_CASES_V1,
    includeBaseline = true,
  } = {}) => {
    const result = await runArchiveEdgeCaseHarnessV1({ cases, includeBaseline });
    const compact = formatArchiveEdgeCaseCompactSummary(result);
    console.log('[ARCHIVE_EDGE_CASE_HARNESS_V1]\n' + compact);
    console.table(result.edgeCaseReports);
    return {
      compact,
      ...result,
    };
  };
  window.__runSafeArchiveFuzzLayerV1 = async ({
    seeds = DEFAULT_ARCHIVE_FUZZ_SEEDS_V1,
    maxMutationsPerRun,
    maxRuntimeMs,
    includeContinuity = true,
  } = {}) => {
    const result = await runSafeArchiveFuzzLayerV1({
      seeds,
      maxMutationsPerRun,
      maxRuntimeMs,
      includeContinuity,
    });
    const compact = formatArchiveFuzzCompactSummary(result);
    console.log('[SAFE_ARCHIVE_FUZZ_LAYER_V1]\n' + compact);
    console.table(result.mutationReports);
    return {
      compact,
      ...result,
    };
  };
  window.__runCredentialHygieneAuditorV1 = async ({
    profiles = DEFAULT_CREDENTIAL_HYGIENE_PROFILES_V1,
    mode = 'discovery_mode',
  } = {}) => {
    const result = await runCredentialHygieneAuditorV1({ profiles, mode });
    const compact = formatCredentialHygieneCompactSummary(result);
    console.log('[CREDENTIAL_HYGIENE_AUDITOR_V1]\n' + compact);
    console.table(result.reports);
    return {
      compact,
      ...result,
    };
  };
  window.__runCredentialHygieneBaselinePackV1 = async ({
    cases = DEFAULT_CREDENTIAL_HYGIENE_BASELINE_CASES_V1,
    profiles = DEFAULT_CREDENTIAL_HYGIENE_EXPANDED_PROFILES_V1,
    mode = 'discovery_mode',
    includeContinuity = true,
  } = {}) => {
    const result = await runCredentialHygieneBaselinePackV1({
      cases,
      profiles,
      mode,
      includeContinuity,
    });
    const compact = formatCredentialHygieneBaselineCompactSummary(result);
    console.log('[CREDENTIAL_HYGIENE_BASELINE_PACK_V1]\n' + compact);
    console.table(result.caseReports);
    return {
      compact,
      ...result,
    };
  };
  window.__runSurfaceScanNormalizationV1 = async ({
    targetUrl = 'http://localhost',
    maxDepth = 1,
    maxPages = 10,
  } = {}) => {
    const result = await spiderFullScanNormalized({
      targetUrl,
      maxDepth,
      maxPages,
      dirBruteforce: false,
      enableVulnVerification: false,
      enableOsintImport: false,
      enableTopologyDiscovery: false,
      enableSnapshotRefresh: false,
      enableVideoStreamAnalyzer: false,
      enableCredentialDepthAudit: false,
      enablePassiveArpDiscovery: false,
      enableUptimeMonitoring: false,
      enableNeighborDiscovery: false,
      enableThreatIntel: false,
      enableScheduledAudits: false,
    });
    console.log(`[SURFACE_SCAN_NORMALIZATION_V1]\\n${result.marker}`);
    if (result.fingerprintMarker) {
      console.log(`[SPIDER_FINGERPRINT_ENRICHMENT_V1]\\n${result.fingerprintMarker}`);
    }
    if (result.authBoundaryMarker) {
      console.log(`[SPIDER_AUTH_BOUNDARY_HINTS_V1]\\n${result.authBoundaryMarker}`);
    }
    if (result.evidenceMarker) {
      console.log(`[SPIDER_EVIDENCE_REPORT_V1]\\n${result.evidenceMarker}`);
    }
    console.log(result.surfaceScanResult);
    return result;
  };
  window.__runSpiderFingerprintEnrichmentV1 = async ({
    targetUrl = 'http://localhost',
    maxDepth = 1,
    maxPages = 10,
  } = {}) => {
    const result = await spiderFullScanNormalized({
      targetUrl,
      maxDepth,
      maxPages,
      dirBruteforce: false,
      enableVulnVerification: false,
      enableOsintImport: false,
      enableTopologyDiscovery: false,
      enableSnapshotRefresh: false,
      enableVideoStreamAnalyzer: false,
      enableCredentialDepthAudit: false,
      enablePassiveArpDiscovery: false,
      enableUptimeMonitoring: false,
      enableNeighborDiscovery: false,
      enableThreatIntel: false,
      enableScheduledAudits: false,
    });
    console.log(`[SPIDER_FINGERPRINT_ENRICHMENT_V1]\\n${result.fingerprintMarker || 'n/a'}`);
    console.log(result.surfaceScanResult);
    return result;
  };
  window.__runSpiderAuthBoundaryHintsV1 = async ({
    targetUrl = 'http://localhost',
    maxDepth = 1,
    maxPages = 10,
  } = {}) => {
    const result = await spiderFullScanNormalized({
      targetUrl,
      maxDepth,
      maxPages,
      dirBruteforce: false,
      enableVulnVerification: false,
      enableOsintImport: false,
      enableTopologyDiscovery: false,
      enableSnapshotRefresh: false,
      enableVideoStreamAnalyzer: false,
      enableCredentialDepthAudit: false,
      enablePassiveArpDiscovery: false,
      enableUptimeMonitoring: false,
      enableNeighborDiscovery: false,
      enableThreatIntel: false,
      enableScheduledAudits: false,
    });
    console.log(`[SPIDER_AUTH_BOUNDARY_HINTS_V1]\\n${result.authBoundaryMarker || 'n/a'}`);
    console.log(result.surfaceScanResult);
    return result;
  };
  window.__runSpiderEvidenceReportV1 = async ({
    targetUrl = 'http://localhost',
    maxDepth = 1,
    maxPages = 10,
  } = {}) => {
    const result = await spiderFullScanNormalized({
      targetUrl,
      maxDepth,
      maxPages,
      dirBruteforce: false,
      enableVulnVerification: false,
      enableOsintImport: false,
      enableTopologyDiscovery: false,
      enableSnapshotRefresh: false,
      enableVideoStreamAnalyzer: false,
      enableCredentialDepthAudit: false,
      enablePassiveArpDiscovery: false,
      enableUptimeMonitoring: false,
      enableNeighborDiscovery: false,
      enableThreatIntel: false,
      enableScheduledAudits: false,
    });
    console.log(`[SPIDER_EVIDENCE_REPORT_V1]\\n${result.evidenceMarker || 'n/a'}`);
    console.log(result.evidenceReport);
    return result;
  };
  window.__runSpiderBaselinePackV1 = async ({
    cases = DEFAULT_SPIDER_BASELINE_CASES_V1,
    includeContinuity = true,
  } = {}) => {
    const result = await runSpiderBaselinePackV1({ cases, includeContinuity });
    const compact = formatSpiderBaselineCompactSummaryV1(result);
    console.log('[SPIDER_BASELINE_PACK_V1]\n' + compact);
    console.table(result.caseReports);
    return {
      compact,
      ...result,
    };
  };

  window.__runPassiveTrafficAnalyzerV1 = async ({
    interfaceName = 'any',
    durationSecs = 15,
    targetId = null,
    host = null,
    surfaceScanResult = null,
  } = {}) => {
    const result = await runPassiveTrafficAnalyzerV1({
      interfaceName,
      durationSecs,
      targetId,
      host,
      surfaceScanResult,
    });
    console.log(`[PASSIVE_OBSERVATION_V1]\n${result.marker}`);
    console.log(result.passiveObservation);
    return result;
  };
  window.__runPassiveTrafficBaselinePackV1 = async ({
    cases = DEFAULT_PASSIVE_TRAFFIC_BASELINE_CASES_V1,
    includeContinuity = true,
  } = {}) => {
    const result = await runPassiveTrafficBaselinePackV1({ cases, includeContinuity });
    const compact = formatPassiveTrafficBaselineCompactSummaryV1(result);
    console.log('[PASSIVE_TRAFFIC_BASELINE_PACK_V1]\n' + compact);
    console.table(result.caseReports);
    return {
      compact,
      ...result,
    };
  };

  window.__runKvDualWriteDiagnostic = async () => {
    const marker = await runKvDualWriteDiagnostic();
    console.log(`[KV_DUAL_WRITE_V1]\n${marker}`);
    return marker;
  };

  window.__runKvReadAnalyticsV1 = async () => {
    const report = await runKvReadAnalyticsV1();
    console.log(`[KV_READ_ANALYTICS_V1]\n${report.marker}`);
    console.log(report);
    return report;
  };
  window.__runPortScanNormalizationV1 = async ({
    host = '127.0.0.1',
  } = {}) => {
    const result = await scanHostPortsNormalized(host);
    console.log(`[PORT_SCAN_NORMALIZATION_V1]\\n${result.marker}`);
    console.log(result.portScanResult);
    return result;
  };
  window.__runPortAuditNormalizationV1 = async ({
    host = '127.0.0.1',
    expectedOpenPorts = [80, 443, 554],
  } = {}) => {
    const result = await auditHostPortsNormalized(host, { expectedOpenPorts });
    console.log(`[PORT_AUDIT_NORMALIZATION_V1]\\n${result.marker}`);
    console.log(result.portAuditResult);
    return result;
  };
  window.__runPortScanAuditBaselinePackV1 = async ({
    cases = DEFAULT_PORT_SCAN_AUDIT_BASELINE_CASES_V1,
  } = {}) => {
    const result = await runPortScanAuditBaselinePackV1({ cases });
    const compact = formatPortScanAuditBaselineCompactSummary(result);
    console.log('[PORT_SCAN_AUDIT_BASELINE_PACK_V1]\n' + compact);
    console.table(result.caseReports);
    return {
      compact,
      ...result,
    };
  };
  console.info('[DEV] __runProbeEvalBaseline is ready');
  console.info('[DEV] __runSessionLifecycleKnownBadPackV1 is ready');
  console.info('[DEV] __runArchiveBaselinePackV1 is ready');
  console.info('[DEV] __runArchiveEdgeCaseHarnessV1 is ready');
  console.info('[DEV] __runSafeArchiveFuzzLayerV1 is ready');
  console.info('[DEV] __runCredentialHygieneAuditorV1 is ready');
  console.info('[DEV] __runCredentialHygieneBaselinePackV1 is ready');
  console.info('[DEV] __runSurfaceScanNormalizationV1 is ready');
  console.info('[DEV] __runSpiderFingerprintEnrichmentV1 is ready');
  console.info('[DEV] __runSpiderAuthBoundaryHintsV1 is ready');
  console.info('[DEV] __runSpiderEvidenceReportV1 is ready');
  console.info('[DEV] __runSpiderBaselinePackV1 is ready');
  console.info('[DEV] __runPassiveTrafficAnalyzerV1 is ready');
  console.info('[DEV] __runPassiveTrafficBaselinePackV1 is ready');
  console.info('[DEV] __runKvDualWriteDiagnostic is ready');
  console.info('[DEV] __runKvReadAnalyticsV1 is ready');
  console.info('[DEV] __runPortScanNormalizationV1 is ready');
  console.info('[DEV] __runPortAuditNormalizationV1 is ready');
  console.info('[DEV] __runPortScanAuditBaselinePackV1 is ready');
}
