# Phase 30.4b — Rollout `auth_result_v1` beyond cookie path

## Goal
Extend `auth_result_v1` from cookie/session normalization into additional auth-related consumer paths with additive, backward-safe integration.

## What was rolled out
- `src/api/archiveResultContract.js`
  - `normalizeArchiveResultV1(...)` now also emits additive `authResult` via `normalizeArchiveAuthResultV1(...)`.
- `src/api/authResultContract.js`
  - Added `normalizeArchiveAuthResultV1({ archiveResult })` to map archive auth semantics into `auth_result_v1`.

## Shape consistency and observability
- Archive baseline, edge-case, and safe-fuzz reports now validate:
  - `auth_result_v1` shape (`validateAuthResultV1Shape`)
  - `authResult.target_id` consistency with archive `target_id`
  - `authResult.auth_path_type` consistency with archive path type (`archive:<archive_path_type>`)
- Compact summaries now include an `AUTH_RESULT_V1|present=X/Y` runtime marker.

## Semantics mapping (archive -> auth)
- `target_id` -> propagated.
- `auth_path_type` -> `archive:<archive_path_type>`.
- `auth_required` -> derived from `search_requires_auth || export_requires_auth`.
- `partial_access_detected` -> propagated.
- `issues`/`issuesCount` -> normalized and consistent.
- `resultClass` -> derived with explicit handling for no-auth/partial/ambiguous states.

## Compatibility
- Existing `archive_result_v1` fields remain unchanged.
- `authResult` is additive and does not replace existing archive result fields.

## Out of scope
- No UI changes.
- No brute/spraying logic.
- No ValidationAgent / graph / vault changes.
