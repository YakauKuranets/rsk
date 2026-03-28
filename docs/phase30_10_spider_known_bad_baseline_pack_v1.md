# Phase 30.10 — Spider known-bad / baseline pack v1

## Goal
Add a small, controlled baseline/known-bad validation layer on top of existing normalized spider outputs.

## Scope
- Reuse `surface_scan_result_v1` as the truth-layer.
- Evaluate spider enrichment/auth-boundary/evidence as explainable helper signals.
- Keep explicit `passed` / `failed` / `inconclusive` semantics.
- Do not change UI/LAB/graph/ValidationAgent.
- Do not introduce aggressive spider behavior.

## Added
- `runSpiderBaselinePackV1(...)`
- baseline case definitions with categories:
  - `known-good`
  - `known-bad`
  - `ambiguous`
- case-level checks:
  - `classMatch`
  - `hasSurfaceContract`
  - `enrichmentPresentWhenExpected`
  - `authBoundaryHintConsistency`
  - `evidenceReportConsistency`
- `spiderBaselineMetrics(...)`
- `formatSpiderBaselineCompactSummaryV1(...)`
- runtime marker format `SPIDER_BASELINE_V1|...`

## Continuity
Output includes continuity-ready block to keep the roadmap handoff explicit.
