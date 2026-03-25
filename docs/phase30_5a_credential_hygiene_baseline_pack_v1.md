# Phase 30.5a — Credential Hygiene baseline / known-bad expansion

## Scope
Expand Credential Hygiene Auditor v1 into a baseline-ready controlled layer without active spraying.

## Added
- `src/api/credentialHygieneBaselinePack.js`
  - `runCredentialHygieneBaselinePackV1(...)`
  - `credentialHygieneBaselineMetrics(...)`
  - `formatCredentialHygieneBaselineCompactSummary(...)`
  - `DEFAULT_CREDENTIAL_HYGIENE_BASELINE_CASES_V1`
- `src/api/credentialHygieneAuditor.js`
  - expanded profiles via `DEFAULT_CREDENTIAL_HYGIENE_EXPANDED_PROFILES_V1`

## Baseline classes
- known-good
- known-bad
- ambiguous/inconclusive

## Case-level checks
- `classMatch`
- `issuesCountMatch`
- `hasAuthContract`
- `boundaryInterpretationConsistent`

## Continuity
Baseline pack runs through existing `runCredentialHygieneAuditorV1(...)` and keeps `auth_result_v1` as the truth-layer.

## Runtime observability
- Compact marker: `CREDENTIAL_HYGIENE_BASELINE_V1|...`
- Dev trigger: `window.__runCredentialHygieneBaselinePackV1(...)`

## Constraints honored
- No UI changes
- No graph/Neo4j/ValidationAgent changes
- No brute/spraying behavior
