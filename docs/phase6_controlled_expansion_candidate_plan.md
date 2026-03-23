# Phase 6: Controlled Expansion Candidate Plan (post-probe_stream)

## Goal

Choose exactly one next capability for minimal-agent expansion after `probe_stream`, without big-bang rewrite.

## Candidate set reviewed

Based on existing capability adapter surface:

1. `verify_session_cookie_flags`
2. `search_archive_records`
3. `probe_stream` (baseline/reference only, already integrated)

## Selection criteria

1. Read-only behavior by design.
2. Low operational and UX risk.
3. Clear semantics for reviewer policy decisions.
4. Predictable/typed input-output contract.
5. Good fit for eval/review (status buckets, mismatch checks, fallback policy).

## Candidate comparison

### A) verify_session_cookie_flags

- Inputs: `ipOrUrl` only (no camera credentials in request body).
- Semantics: check cookie flags (`Secure`, related issues) and return bounded issue list.
- Risk profile: low execution risk and low UX coupling.
- Eval fitness: high (binary-ish secure/not secure + issue list + evidence refs).

### B) search_archive_records

- Inputs: camera IP + login + password + date range + channel.
- Semantics: protocol probing + archive record search.
- Risk profile: higher (credential handling, broader surface, network/load side effects).
- Eval fitness: medium (more variable environment outcomes, stronger dependency on target state).

### C) probe_stream (already done)

- Kept as baseline path and not a "new" expansion target for Phase 6.

## Selected next capability

`verify_session_cookie_flags` is the best next candidate for controlled expansion.

## Why this choice

1. Most constrained read-only profile among new options.
2. Does not require user credential flow like archive search.
3. Easier reviewer policy gate (`permitVerifySessionCookieFlags` boolean + scope check).
4. Cleaner eval/review outcome space and easier run-to-run comparison.

## Risks (selected candidate)

1. Environment variability (TLS/offloading/proxy behavior may affect observed cookie flags).
2. Host availability/timeouts can produce inconclusive outcomes.
3. Potential semantic drift if fallback policy is not explicit.

## Minimal migration path

### 1) Backend contract (minimal)

- Extend `agent_minimal` planner/reviewer types with one additional action option for `verify_session_cookie_flags`.
- Keep strict allowlist: only `probe_stream` + `verify_session_cookie_flags`.
- Add compact capability result summary branch for cookie checks (`secure`, `issuesCount`, `errorCode`).
- Keep structured envelope unchanged in shape; only widen enums/summary fields minimally.

### 2) Agent gating (minimal)

- Add explicit reviewer gate flag:
  - `permitProbeStream`
  - `permitVerifySessionCookieFlags`
- Reject mixed/unexpected capability plans in minimal mode.

### 3) Consumer contract (minimal)

- Keep existing normalized result shape in frontend adapter.
- Add normalized fields for cookie-check summary only where needed (no UI screen required).
- Preserve existing fallback behavior principles and keep probe path untouched.

### 4) Eval path (minimal)

- Add cookie-check scenario group to harness (separate from probe baseline):
  - reviewer_rejected
  - capability_succeeded (secure/issue variants)
  - capability_failed/inconclusive
  - fallback rate and semantic-known rate
- Reuse existing snapshot/baseline compare mechanics.

## First implementation step (smallest safe increment)

Implement only backend+adapter skeleton for `verify_session_cookie_flags` in minimal-agent flow behind reviewer flag, with no new UI surface and no removal of existing probe behavior.
