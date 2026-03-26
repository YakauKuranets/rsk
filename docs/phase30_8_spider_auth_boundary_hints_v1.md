# Phase 30.8 — Spider auth-boundary hints v1

## Scope
Add explainable auth-boundary hint generation over existing normalized spider/surface outputs.

## Added
- `src/api/spiderAuthBoundaryHints.js`
  - `deriveSpiderAuthBoundaryHintsV1(...)`
  - `applySpiderAuthBoundaryHintsV1(...)`
  - `formatSpiderAuthBoundaryHintsV1Marker(...)`
- additive integration in `src/api/surfaceScanResultContract.js`
  - `normalizeSpiderFullScanResultV1(...)` now also applies auth-boundary hints
- additive integration in `src/api/tauri.js`
  - `spiderFullScanNormalized(...)` now returns `authBoundaryMarker`

## Hint semantics
Auth-boundary hints remain hint-only and can include:
- `likely_auth_required`
- `partial_exposure_possible`
- `boundary_ambiguous`
- `insufficient_signal`

## Runtime observability
- Marker: `SPIDER_AUTH_BOUNDARY_HINTS_V1|...`
- Dev trigger: `window.__runSpiderAuthBoundaryHintsV1(...)`

## Constraints honored
- no aggressive auth probing
- no credential pressure / brute-like behavior
- no UI/LAB/graph/ValidationAgent changes
- no baseline pack in this step
