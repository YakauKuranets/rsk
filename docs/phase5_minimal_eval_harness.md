# Phase 5: Minimal Eval/Review Harness (probe_stream only)

## Scope

- Only for `probe_stream` minimal-agent path.
- No self-learning.
- No new capabilities.
- No backend storage/model rewrite.

## Harness entry

- `runProbeStreamEvalHarness(...)` in `src/api/probeEvalHarness.js`.

## Reproducible scenarios

1. `reviewer_rejected_when_permit_false`
2. `capability_succeeded_on_alive_target`
3. `capability_failed_on_dead_target`
4. `fallback_path_behavior` (forced legacy fallback)
5. `semanticAliveKnown_sensitive_behavior` (synthetic guard check)

## Metrics

- `finalStatusDistribution`
- `reviewerRejectRate`
- `fallbackRate`
- `semanticAliveKnownRate`
- `mismatchIndications`

## Manual run example (browser console)

```js
import { runProbeStreamEvalHarness } from '/src/api/probeEvalHarness.js';

const report = await runProbeStreamEvalHarness({
  aliveTargetId: 'known_alive_target_id',
  deadTargetId: 'known_dead_target_id',
  mode: 'discovery_mode',
});

console.log(report.metrics);
console.table(report.events);
```

## Notes

- If `aliveTargetId`/`deadTargetId` are not provided, related scenarios are skipped.
- `fallback_path_behavior` is executed only when `aliveTargetId` is provided.
