# Phase 32.5r.1 — Graph environment closure + remediation rerun

## Scope
- Environment/runtime closure only for Knowledge Vault shadow path.
- No UI changes.
- No new graph analytics/features.
- No ValidationAgent changes.

## Prerequisites (local/staging)
1. Copy env template:
   - `cp infra/neo4j-shadow/.env.example infra/neo4j-shadow/.env`
2. Ensure required keys are set in `.env`:
   - `NEO4J_USER`
   - `NEO4J_PASSWORD`
   - `NEO4J_BOLT_PORT`
   - `NEO4J_DATABASE`
   - `KV_SHADOW_MODE=true`
   - `KV_DUAL_WRITE_ENABLED=true`
3. Ensure Neo4j is running:
   - `docker compose -f infra/neo4j-shadow/docker-compose.yml --env-file infra/neo4j-shadow/.env up -d`
4. Ensure `cypher-shell` is available in PATH.

## Readiness diagnostic
Run:
- `scripts/graph/kv_graph_env_readiness_v1.sh`

Expected marker:
- `KV_GRAPH_ENV_READY_V1|status=pass|...`

Reported fields:
- `neo4j_reachable`
- `cypher_shell_present`
- `env_complete`
- `shadow_write_enabled`

## One-click closure + rerun
Run:
- `scripts/graph/kv_env_closure_and_remediation_rerun_v1.sh`

This will:
1. bootstrap `.env` from `.env.example` if missing,
2. try `docker compose up -d` if Docker Compose is available,
3. run readiness diagnostic,
4. rerun integrated load, reconciliation, and latency benchmark,
5. refresh remediation JSON/Markdown reports.

## Honest failure policy
If blockers remain, reports must keep:
- `overall_status=blocked`
- `blockers_resolved=false`
- exact blocker identifiers with reasons in `remaining_blockers`.
