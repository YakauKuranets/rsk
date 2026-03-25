# Phase 30.5 — Credential Hygiene Auditor v1

## Scope
A controlled, profile-driven credential hygiene validation layer built on top of `auth_result_v1`.

## Added
- `src/api/credentialHygieneAuditor.js`
  - `DEFAULT_CREDENTIAL_HYGIENE_PROFILES_V1`
  - `runCredentialHygieneAuditorV1(...)`
  - `credentialHygieneMetrics(...)`
  - `formatCredentialHygieneCompactSummary(...)`

## Design constraints honored
- No active spraying or brute-like behavior.
- No UI redesign.
- No graph/Neo4j integration.
- No ValidationAgent integration.
- Controlled profiles only.

## Contract semantics
Auditor uses `auth_result_v1` as truth-layer and reports hygiene state based on:
- `default_credential_detected`
- `weak_password_detected`
- `auth_required`
- `auth_boundary_strength`
- `partial_access_detected`
- `resultClass` restricted to `passed|failed|inconclusive`

## Runtime observability
- Compact summary emits marker:
  - `CREDENTIAL_HYGIENE_V1|...`

## Dev runtime trigger
- `window.__runCredentialHygieneAuditorV1({ profiles, mode })`

## Out of scope
- Baseline/known-bad expansion depth (next step).
