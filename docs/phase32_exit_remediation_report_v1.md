# Phase 32 Remediation Report v1

Generated at: 2026-03-26T06:45:09Z

Marker: `KV_EXIT_REMEDIATION_V1|status=blocked|blockers=3|ts=2026-03-26T06:45:09Z`

- integrated_100_event_load: **blocked**
- reconciliation_check: **blocked**
- latency_benchmark: **blocked**
- blockers_resolved: **false**
- recommendation: **stay_in_phase32**

## Remaining blockers
- integrated_100_event_load_blocked
- reconciliation_blocked
- latency_blocked_or_problematic

## Stage markers
- KV_EXIT_REMEDIATION_V1|stage=integrated_load|status=blocked|reason=missing_env_or_cypher_shell|events=100
- KV_EXIT_REMEDIATION_V1|stage=reconciliation|status=blocked
- KV_EXIT_REMEDIATION_V1|stage=latency|status=blocked|baselineMs=585.86|shadowMs=-1
