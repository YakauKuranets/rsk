# Phase 30.6a — Port scanner contract normalization

## Scope
Introduce `port_scan_result_v1` as a normalized discovery-layer consumer contract for port scanner outputs.

## Added
- `src/api/portScanResultContract.js`
  - `normalizePortScanResultV1(...)`
  - `normalizeScanHostPortsResultV1(...)`
  - `validatePortScanResultV1Shape(...)`
  - `formatPortScanResultV1Marker(...)`
- `src/api/tauri.js`
  - additive adapter `scanHostPortsNormalized(host)` returning:
    - `raw`
    - `portScanResult`
    - runtime marker `PORT_SCAN_RESULT_V1|...`

## Semantics
`port_scan_result_v1` fields:
- `target_id`
- `host`
- `reachable`
- `open_ports`
- `services`
- `protocol`
- `banner`
- `vendor_hints`
- `evidenceRefs`
- `confidence`
- `resultClass`

## Notes
- Discovery and hints only; no risk interpretation added here.
- `resultClass` is constrained to `passed|failed|inconclusive`.
- Raw scanner output remains available but is no longer the only consumer shape.

## Dev runtime trigger
- `window.__runPortScanNormalizationV1({ host })`

## Out of scope
- `port_audit_result_v1`
- baseline packs for port scan
- graph ingest / aggressive scanning expansion / UI redesign
