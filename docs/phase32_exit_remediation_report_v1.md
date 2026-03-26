# Phase 32 Remediation Report v1

Generated at: 2026-03-26T18:33:04Z

Marker: `KV_EXIT_REMEDIATION_V1|status=blocked|blockers=4|ts=2026-03-26T18:33:04Z`

- graph_env_readiness: **blocked** (neo4j_unreachable)
- integrated_100_event_load: **blocked** (shadow_writes_failed)
- reconciliation_check: **blocked** (count_drift_out_of_tolerance)
- latency_benchmark: **blocked** (shadow_query_unreachable)
- blockers_resolved: **false**
- recommendation: **stay_in_phase32**

## Remaining blockers
- graph_env_not_ready:neo4j_unreachable
- integrated_100_event_load_blocked:shadow_writes_failed
- reconciliation_blocked:count_drift_out_of_tolerance
- latency_blocked_or_problematic:shadow_query_unreachable

## Stage markers
- KV_GRAPH_ENV_READY_V1|status=blocked|reason=neo4j_unreachable|neo4j_reachable=false|cypher_shell_present=true|env_complete=true|shadow_write_enabled=true|ts=2026-03-26T18:33:04Z
- KV_EXIT_REMEDIATION_V1|stage=integrated_load|status=blocked|reason=shadow_writes_failed|events=100|written=0|errored=100
- KV_EXIT_REMEDIATION_V1|stage=reconciliation|status=blocked|reason=count_drift_out_of_tolerance
- KV_EXIT_REMEDIATION_V1|stage=latency|status=blocked|reason=shadow_query_unreachable|baselineMs=506.42|shadowMs=1440.78
