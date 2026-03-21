# Phase 5: Minimal Eval/Review Harness (probe_stream only)

## Scope

- Only for `probe_stream` minimal-agent path.
- No self-learning.
- No new capabilities.
- No backend storage/model rewrite.

## Harness entry

- `runProbeStreamEvalHarness(...)` in `src/api/probeEvalHarness.js`.
- `runProbeStreamEvalSnapshot(...)` in `src/api/probeEvalHarness.js`.
- `compareProbeEvalSnapshots(...)` in `src/api/probeEvalHarness.js`.

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

## Snapshot run example (multi-input)

```js
import { runProbeStreamEvalSnapshot } from '/src/api/probeEvalHarness.js';

const snapshot = await runProbeStreamEvalSnapshot({
  mode: 'discovery_mode',
  inputs: [
    { caseId: 'site_a', aliveTargetId: 'alive_a', deadTargetId: 'dead_a' },
    { caseId: 'site_b', aliveTargetId: 'alive_b', deadTargetId: 'dead_b' },
  ],
});

console.log(snapshot.snapshotId, snapshot.createdAt);
console.log(snapshot.metrics);
console.table(snapshot.events);
```

## Compare two snapshots

```js
import { compareProbeEvalSnapshots } from '/src/api/probeEvalHarness.js';

const delta = compareProbeEvalSnapshots(snapshotPrev, snapshotNext);
console.log(delta);
```

## Notes

- If `aliveTargetId`/`deadTargetId` are not provided, related scenarios are skipped.
- `fallback_path_behavior` is executed only when `aliveTargetId` is provided.
- Snapshot format includes: `snapshotId`, `createdAt`, `inputs`, `events`, `metrics`.
