# Phase 30.6c — Port scanner / audit baseline pack v1

## Scope
Add a controlled baseline-ready validation layer over existing:
- `port_scan_result_v1`
- `port_audit_result_v1`

## Added
- `src/api/portScanAuditBaselinePack.js`
  - `runPortScanAuditBaselinePackV1(...)`
  - `portScanAuditBaselineMetrics(...)`
  - `formatPortScanAuditBaselineCompactSummary(...)`
  - `DEFAULT_PORT_SCAN_AUDIT_BASELINE_CASES_V1`

## Baseline classes
- known-good
- known-bad
- ambiguous/inconclusive

## Case-level checks
- `classMatch`
- `issuesCountMatch`
- `hasScanContract`
- `hasAuditContract`
- `riskInterpretationConsistent`

## Runtime observability
- Compact marker: `PORT_SCAN_AUDIT_BASELINE_V1|...`
- Dev trigger: `window.__runPortScanAuditBaselinePackV1({ cases })`

## Constraints honored
- no contract redesign for scan/audit
- no UI changes
- no graph/spider enrichment
- no aggressive scanning expansion
