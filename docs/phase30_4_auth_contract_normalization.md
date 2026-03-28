# Phase 30.4 — Auth Contract Normalization (`auth_result_v1`)

## Scope
This change introduces a normalized adapter contract for authentication-boundary outcomes and wires it into existing cookie capability paths without changing core execution semantics.

## Added
- `src/api/authResultContract.js`
  - `AUTH_RESULT_CONTRACT_VERSION = "auth_result_v1"`
  - `normalizeAuthResultV1(input)`
  - `normalizeSessionCookieAuthResultV1({ targetId, cookieResult })`
  - `validateAuthResultV1Shape(input)`

## Integration points
- `src/api/capabilities.js`
  - Cookie capability normalization now embeds `authResult` based on `auth_result_v1`.
  - Target identity is threaded through to normalization so `target_id` remains stable.

- `src/api/probeEvalHarness.js`
  - Invariant checks now require `authResult` presence in normalized cookie outputs.
  - Harness contract health now also validates `auth_result_v1` shape compatibility.

## Compatibility
- Existing `cookie_result_v1` fields are preserved.
- `authResult` is additive and backward-safe for existing consumers.

## Out of scope
- No UI changes.
- No brute/spraying or credential attack logic.
- No graph/agent/vault orchestration changes.
