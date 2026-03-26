# Validation Quality Report v1

Version: `validation_quality_report_v1`  
Date: `2026-03-26`

Machine-readable source: `docs/validation_quality_report_v1.json`.
Coverage linkage source: `docs/coverage_matrix_v1.json`.

## Compact summary
- **Passed / failed / inconclusive trends:** mixed; inconclusive is expected in sparse-signal contours.
- **Strongest validation contours:** port scan contract consistency, archive normalization checks.
- **Weakest validation contours:** passive low-traffic windows, sparse surface/spider attribution.
- **Top false-safety hotspots:** auth/session fallback-limited certainty, over-read fingerprint hints, quiet passive windows.
- **Evidence quality hotspots:** indirect passive correlation; assumption-heavy attribution in sparse spider data.

## 1) Contract health
### Stable (runtime-grade / baseline-grade)
- `archive_result_v1` — runtime-grade.
- `port_scan_result_v1` — runtime-grade.
- `port_audit_result_v1` — baseline-grade.

### Consistency risk zones (partial-only)
- `passive_observation_result_v1` — partial-only; quality is window-dependent.
- `surface_scan_result_v1` + spider evidence layers — partial-only under weak crawl signal.

## 2) Baseline health
### Mature baseline packs
- `archiveBaselinePack`
- `portScanAuditBaselinePack`
- `credentialHygieneBaselinePack`

### Minimal/context-bounded baseline packs
- `passiveTrafficBaselinePack`
- `spiderBaselinePack`
- `session_lifecycle_known_bad_pack_v1`

## 3) Inconclusive analysis
### Inconclusive is normal when
- Passive traffic is too sparse in the observation window.
- Surface/spider discovery has low exposure and weak evidence.

### Inconclusive is a weakness signal when
- It appears on high-signal targets repeatedly.
- It appears in checks expected to be deterministic.

## 4) False-safety analysis
1. **Auth/session**: limited-path success can hide untested lifecycle edges.
2. **Spider fingerprint attribution**: hints can be mistaken for definitive ID.
3. **Passive observation**: quiet/noisy windows can be mistaken for stable behavior.

## 5) Coverage linkage
- Matrix source: `docs/coverage_matrix_v1.json`.
- `port scanner = strong` coverage aligns with runtime-grade validation confidence.
- `passive` and `surface/spider` partial coverage aligns with inconclusive-heavy behavior and caution requirements.

## 6) Next priorities
1. Phase 32 — Knowledge Vault v1 (shadow mode) artifact ingestion.
2. Cross-pack trend rollup to quantify trend directions over time.
3. False-safety guardrails to avoid overconfident decisions in partial-only zones.

## Validation grade legend
- **runtime-grade**: stable, deterministic contract behavior in mainline usage.
- **baseline-grade**: stable in baseline packs but context-limited.
- **partial-only**: useful but not sufficient for strong safety claims.
- **weak/risky**: high uncertainty or high false-safety risk.
