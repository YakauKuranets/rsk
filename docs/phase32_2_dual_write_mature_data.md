# Phase 32.2 — Dual-write для зрелых данных (shadow mode)

## Scope
- Primary storage remains unchanged.
- Neo4j is shadow-only.
- No UI changes.
- No ValidationAgent changes.
- No graph-based runtime decisions.

## What is dual-written in this phase
- Capability run metadata (`Run`, `Capability`, `ValidationPath`).
- Session/cookie findings.
- Stream findings.
- Archive search findings (mature stream-adjacent contour).
- Evidence refs as hashed refs only (`sha256:<ref>`), no raw content.
- Device/service metadata from normalized capability context.

## Safety constraints
- If Neo4j is unavailable, primary flow continues.
- Graph write errors are non-fatal.
- Raw passwords/tokens/cookies are not written.

## Runtime markers
- `KV_DUAL_WRITE_V1|status=queued|...`
- `KV_DUAL_WRITE_V1|status=skipped|...`
- `KV_DUAL_WRITE_V1|status=error|...` (diagnostic stderr path)

## Diagnostic path
- Command: `kv_dual_write_diagnostic`
- JS adapter: `runKvDualWriteDiagnostic()`
- Dev console trigger: `window.__runKvDualWriteDiagnostic()`

## Limits
- This phase does not add graph analytics/ranking.
- This phase does not enable graph reads in runtime decisions.
- Full dual-write expansion to other contours is deferred.
