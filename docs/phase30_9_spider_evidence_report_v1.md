# Phase 30.9 — Spider evidence report v1

## Scope
Add an explainable report/evidence layer over current normalized spider/surface outputs.

## Added
- `src/api/spiderEvidenceReport.js`
  - `buildSpiderEvidenceReportV1(...)`
  - `validateSpiderEvidenceReportV1Shape(...)`
  - `formatSpiderEvidenceCompactSummaryV1(...)`
- additive integration in `src/api/tauri.js`
  - `spiderFullScanNormalized(...)` now returns:
    - `evidenceReport`
    - `evidenceMarker` (`SPIDER_EVIDENCE_REPORT_V1|...`)

## Report structure
Grouped, explainable sections:
- surface summary
- service findings
- vendor/model hints
- stream/archive hints
- auth-boundary hints
- evidence/support
- limitations

## Constraints honored
- report remains additive and explainable
- no aggressive probing/crawling expansion
- no UI/LAB/graph/ValidationAgent changes
- no baseline pack in this step

## Dev runtime trigger
- `window.__runSpiderEvidenceReportV1(...)`
