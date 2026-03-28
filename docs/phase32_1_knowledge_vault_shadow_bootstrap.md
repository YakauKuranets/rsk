# Phase 32.1 — Knowledge Vault v1 (shadow mode) / Neo4j shadow bootstrap

## Scope
- Shadow-only graph backend bootstrap.
- No UI changes.
- No ValidationAgent changes.
- No runtime decision-path influence.
- No dual-write in this phase.

## Added artifacts
- `infra/neo4j-shadow/.env.example` — connection/config template.
- `infra/neo4j-shadow/docker-compose.yml` — Neo4j shadow service.
- `infra/neo4j-shadow/schema_v1.cypher` — minimal schema constraints/indexes.
- `scripts/graph/neo4j_shadow_apply_schema.sh` — schema apply helper.
- `scripts/graph/neo4j_shadow_healthcheck.sh` — health marker helper (`KV_SHADOW_HEALTH_V1|...`).
- `docs/knowledge_vault_schema_v1.json` — machine-readable schema definition and safety policy.

## Safety boundaries
- Graph remains foundation-only in shadow mode.
- No raw secrets/tokens/cookies storage.
- No graph-based ranking or decision logic.

## Quick start
1. `cp infra/neo4j-shadow/.env.example infra/neo4j-shadow/.env`
2. `docker compose -f infra/neo4j-shadow/docker-compose.yml up -d`
3. `scripts/graph/neo4j_shadow_apply_schema.sh`
4. `scripts/graph/neo4j_shadow_healthcheck.sh`

## Expected marker
- `KV_SHADOW_HEALTH_V1|status=ok|...`

## Continuity
- Prepared for Phase 32.2 dual-write of mature data only.
