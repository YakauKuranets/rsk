# Phase 30.6 — Surface / spider contract normalization

## Scope
Introduce `surface_scan_result_v1` as a normalized consumer contract for spider/surface outputs.

## Added
- `src/api/surfaceScanResultContract.js`
  - `normalizeSurfaceScanResultV1(...)`
  - `normalizeSpiderFullScanResultV1(...)`
  - `validateSurfaceScanResultV1Shape(...)`
  - `formatSurfaceScanResultV1Marker(...)`
- `src/api/tauri.js`
  - additive adapter `spiderFullScanNormalized(...)` returning:
    - `raw`
    - `surfaceScanResult`
    - runtime marker `SURFACE_SCAN_RESULT_V1|...`

## Semantics
`surface_scan_result_v1` fields:
- `target_id`
- `host`
- `reachable`
- `resultClass`
- `services`
- `web_endpoints`
- `stream_hints`
- `archive_hints`
- `vendor_hints`
- `auth_boundary_hints`
- `evidenceRefs`
- `confidence`

## Notes
- Discovery data is normalized into surface/hint fields only.
- No risk-interpretation layer added in this step.
- No aggressive spider/auth behavior introduced.

## Dev runtime trigger
- `window.__runSurfaceScanNormalizationV1({ targetUrl, maxDepth, maxPages })`

## Out of scope
- `port_scan_result_v1`
- `port_audit_result_v1`
- baseline packs for surface contract
- graph ingest / spider enrichment / UI redesign
