# Phase 30.6b — Port audit contract normalization

## Scope
Introduce `port_audit_result_v1` as a normalized risk-interpretation layer on top of `port_scan_result_v1`.

## Added
- `src/api/portAuditResultContract.js`
  - `normalizePortAuditResultV1(...)`
  - `normalizePortAuditFromScanResultV1(...)`
  - `validatePortAuditResultV1Shape(...)`
  - `formatPortAuditResultV1Marker(...)`
- `src/api/tauri.js`
  - additive adapter `auditHostPortsNormalized(host, options)` returning:
    - `raw`
    - `portScanResult`
    - `portAuditResult`
    - runtime marker `PORT_AUDIT_RESULT_V1|...`

## Semantics
`port_audit_result_v1` fields:
- `target_id`
- `audited_ports`
- `unexpected_open_ports`
- `sensitive_ports_exposed`
- `auth_boundary_findings`
- `plaintext_service_detected`
- `legacy_service_detected`
- `risk_level`
- `issues`
- `issuesCount`
- `recommendations`
- `evidenceRefs`
- `confidence`
- `resultClass`

## Notes
- Scanner discovery and audit judgment are separated.
- `issues` remains a string array and `issuesCount` is synchronized.
- `resultClass` constrained to `passed|failed|inconclusive`.

## Dev runtime trigger
- `window.__runPortAuditNormalizationV1({ host, expectedOpenPorts })`

## Out of scope
- baseline pack for port audit
- graph ingest / UI redesign / aggressive scanning expansion
