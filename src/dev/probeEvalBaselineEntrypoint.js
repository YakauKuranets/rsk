import {
  DEFAULT_PROBE_EVAL_BASELINE_INPUTS,
  formatProbeEvalBaselineCompactReport,
  runProbeEvalBaselineRunner,
} from '../api/probeEvalBaselineRunner';

async function runFromDevConsole({ inputs, mode = 'discovery_mode', baseline = null } = {}) {
  const resolvedInputs = Array.isArray(inputs) && inputs.length > 0 ? inputs : DEFAULT_PROBE_EVAL_BASELINE_INPUTS;
  const result = await runProbeEvalBaselineRunner({
    inputs: resolvedInputs,
    mode,
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
