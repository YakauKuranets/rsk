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
  console.info('[DEV] __runProbeEvalBaseline is ready');
  console.info('[DEV] __runSessionLifecycleKnownBadPackV1 is ready');
  console.info('[DEV] __runArchiveBaselinePackV1 is ready');
}
