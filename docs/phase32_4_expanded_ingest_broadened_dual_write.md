# Phase 32.4 — Expanded ingest / broadened dual-write (shadow mode)

## Scope
- Keep sled as primary storage.
- Keep Neo4j as shadow-only.
- Keep non-fatal dual-write semantics.
- No UI changes.
- No ValidationAgent integration.
- No graph influence on runtime decisions.

## Expanded ingest in this phase
- Archive findings beyond initial subset (sanitized summaries only).
- Auth/hygiene mature summaries.
- Surface/spider normalized summaries.
- Scanner/audit normalized summaries.
- Passive observation summaries.
- ProfilePack + case-reference links.
- ReviewDecision and Environment links where stable.

## Runtime markers
- Existing V1 marker remains for capability dual-write.
- Broadened ingest uses `KV_DUAL_WRITE_V2|...`.

## Diagnostic path
- Tauri command: `kv_shadow_ingest_projection_v2`
- Dev trigger: `window.__runKvShadowIngestProjectionV2Diagnostic()`

## Safety constraints
- No raw passwords/tokens/cookies.
- No raw evidence payload content.
- Evidence refs are hashed before graph ingest.
- Neo4j failures must not break primary flow.

## Deferred
- Graph-based ranking hints.
- ValidationAgent integration.
- Runtime decision coupling.
- Storage migration.
