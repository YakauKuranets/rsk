# Phase 30.7 — Spider fingerprint enrichment v1

## Scope
Add a read-only, explainable fingerprint enrichment layer over normalized surface scan results.

## Added
- `src/api/spiderFingerprintEnrichment.js`
  - `deriveSpiderFingerprintEnrichmentV1(...)`
  - `applySpiderFingerprintEnrichmentV1(...)`
  - `formatSpiderFingerprintEnrichmentV1Marker(...)`
- additive integration in `src/api/surfaceScanResultContract.js`
  - `normalizeSpiderFullScanResultV1(...)` now applies enrichment hints safely
- additive integration in `src/api/tauri.js`
  - `spiderFullScanNormalized(...)` now returns `fingerprintMarker`

## Enrichment hints
- vendor/model keyword inference
- service-combination hints
- banner correlation hints
- confidence delta remains bounded and explainable

## Runtime observability
- `SPIDER_FINGERPRINT_ENRICHMENT_V1|...`
- dev trigger: `window.__runSpiderFingerprintEnrichmentV1(...)`

## Constraints honored
- no aggressive spider behavior
- no UI/LAB/graph/ValidationAgent changes
- no baseline pack in this step
- enrichment remains hint-layer (not risk-interpretation)
