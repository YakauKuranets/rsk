# Phase 31.1 — Passive traffic analyzer v1

## Goal
Introduce a passive observation layer that complements `surface_scan_result_v1` without replacing it.

## Guardrails
- Passive-only observation (read/listen only).
- No active probes, no packet injection, no traffic manipulation.
- No UI/graph/ValidationAgent changes in this phase.

## Added
- `passive_observation_result_v1` contract and helpers:
  - `normalizePassiveObservationResultV1(...)`
  - `validatePassiveObservationResultV1Shape(...)`
  - `formatPassiveObservationCompactSummaryV1(...)`
- `runPassiveTrafficAnalyzerV1(...)` in tauri API adapter:
  - wraps `analyze_traffic`
  - maps raw capture to passive normalized output
  - emits marker `PASSIVE_OBSERVATION_V1|...`
- `runPassiveTrafficBaselinePackV1(...)` and metrics:
  - known-good / known-bad / ambiguous baseline cases
  - `passiveObservationMetrics(...)`
  - compact baseline summary formatter
- Dev runtime triggers:
  - `window.__runPassiveTrafficAnalyzerV1(...)`
  - `window.__runPassiveTrafficBaselinePackV1(...)`

## Semantics
- Surface remains the truth-layer for expected service exposure.
- Passive analyzer adds observed behavior context and correlation, including unexpected communication.
- Explicit result classes are preserved: `passed`, `failed`, `inconclusive`.

## Continuity
Baseline output includes roadmap handoff hint: `phase31_2_coverage_matrix_v1`.
