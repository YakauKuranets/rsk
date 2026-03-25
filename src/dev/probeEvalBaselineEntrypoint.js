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
  console.info('[DEV] __runProbeEvalBaseline is ready');
  console.info('[DEV] __runSessionLifecycleKnownBadPackV1 is ready');
  console.info('[DEV] __runArchiveBaselinePackV1 is ready');
  console.info('[DEV] __runArchiveEdgeCaseHarnessV1 is ready');
  console.info('[DEV] __runSafeArchiveFuzzLayerV1 is ready');
  console.info('[DEV] __runCredentialHygieneAuditorV1 is ready');
  console.info('[DEV] __runCredentialHygieneBaselinePackV1 is ready');
}
