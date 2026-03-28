# Phase 32.3 — Read-only analytics from graph

## Scope
- Read-only graph analytics only.
- No changes to dual-write semantics.
- No UI changes.
- No ValidationAgent integration.
- No graph influence on runtime decisions.

## Added
- Rust analytics module: `src-tauri/src/graph_read_analytics.rs`
- Tauri command: `kv_read_analytics_v1`
- JS adapter: `runKvReadAnalyticsV1()`
- Dev trigger: `window.__runKvReadAnalyticsV1()`
- Machine-readable analytics spec: `docs/kv_read_analytics_v1.json`

## Supported analytics queries
1. capability frequency by vendor/device
2. most common finding types by capability
3. evidence linkage density
4. repeated validation paths
5. inconclusive/weak clusters
6. coverage hints from mature subset

## Marker
- `KV_READ_ANALYTICS_V1|status=...|queries=...|failed=...`

## Failure behavior
- Neo4j read failures are non-fatal.
- If graph is sparse/empty, returns limited/inconclusive analytics instead of hard failure.
