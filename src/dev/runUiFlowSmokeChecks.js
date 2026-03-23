import { runUiFlowSmokeChecks } from './uiFlowSmokeChecks.js';

try {
  const report = runUiFlowSmokeChecks();
  console.log('[ui-flow-smoke] status:', report.status);
  report.checks.forEach((check, index) => {
    console.log(`[ui-flow-smoke] ${index + 1}. ${check}`);
  });
} catch (error) {
  console.error('[ui-flow-smoke] FAIL:', error?.message || error);
  process.exitCode = 1;
}
