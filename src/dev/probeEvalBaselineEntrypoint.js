import {
  DEFAULT_COOKIE_EVAL_BASELINE_INPUTS,
  DEFAULT_PROBE_EVAL_BASELINE_INPUTS,
  formatProbeEvalBaselineCompactReport,
  runProbeEvalBaselineRunner,
} from '../api/probeEvalBaselineRunner';

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
  console.info('[DEV] __runProbeEvalBaseline is ready');
}
