# Phase 5: Minimal Eval/Review Harness (probe_stream only)

## Scope

- For minimal-agent eval paths: `probe_stream` and `verify_session_cookie_flags`.
- No self-learning.
- No new capabilities.
- No backend storage/model rewrite.

## Harness entry

- `runProbeStreamEvalHarness(...)` in `src/api/probeEvalHarness.js`.
- `runVerifySessionCookieEvalHarness(...)` in `src/api/probeEvalHarness.js`.
- `runProbeStreamEvalSnapshot(...)` in `src/api/probeEvalHarness.js`.
- `runVerifySessionCookieEvalSnapshot(...)` in `src/api/probeEvalHarness.js`.
- `compareProbeEvalSnapshots(...)` in `src/api/probeEvalHarness.js`.
- `runProbeEvalBaselineRunner(...)` in `src/api/probeEvalBaselineRunner.js`.

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

### Cookie-path metrics (verify_session_cookie_flags)

- `finalStatusDistribution`
- `reviewerRejectRate`
- `secureRate`
- `issuesDetectedRate`
- `inconclusiveFailureRate`

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

## Cookie eval harness example

```js
import { runVerifySessionCookieEvalHarness } from '/src/api/probeEvalHarness.js';

const cookieReport = await runVerifySessionCookieEvalHarness({
  secureTarget: 'https://localhost',
  issuesTarget: 'http://localhost',
  unreachableTarget: 'http://127.0.0.1:1',
  mode: 'discovery_mode',
});

console.log(cookieReport.metrics);
console.table(cookieReport.events);
```

## Compare two snapshots

```js
import { compareProbeEvalSnapshots } from '/src/api/probeEvalHarness.js';

const delta = compareProbeEvalSnapshots(snapshotPrev, snapshotNext);
console.log(delta);
```

## Cookie snapshot + compare

```js
import {
  runVerifySessionCookieEvalSnapshot,
  compareVerifySessionCookieEvalSnapshots,
} from '/src/api/probeEvalHarness.js';

const cookieA = await runVerifySessionCookieEvalSnapshot({
  inputs: [{ caseId: 'cookie_a', secureTarget: 'https://localhost', issuesTarget: 'http://localhost', unreachableTarget: 'http://127.0.0.1:1' }],
});
const cookieB = await runVerifySessionCookieEvalSnapshot({
  inputs: [{ caseId: 'cookie_a', secureTarget: 'https://localhost', issuesTarget: 'http://localhost', unreachableTarget: 'http://127.0.0.1:1' }],
});

const cookieDelta = compareVerifySessionCookieEvalSnapshots(cookieA, cookieB);
console.log(cookieDelta);
```

## Dev-only baseline runner

```js
import {
  runProbeEvalBaselineRunner,
  buildProbeEvalBaseline,
  compareSnapshotAgainstBaseline,
} from '/src/api/probeEvalBaselineRunner.js';

// 1) Create baseline from first run
const first = await runProbeEvalBaselineRunner({
  inputs: [
    { caseId: 'site_a', aliveTargetId: 'alive_a', deadTargetId: 'dead_a' },
    { caseId: 'site_b', aliveTargetId: 'alive_b', deadTargetId: 'dead_b' },
  ],
});
const baseline = buildProbeEvalBaseline(first.snapshot);

// 2) Run again and compare against baseline
const second = await runProbeEvalBaselineRunner({
  baseline,
  inputs: baseline.sourceSnapshot.inputs,
});

console.log(second.comparison.classification); // improved | unchanged | regressed | inconclusive
console.log(second.comparison.summary); // human-readable delta summary
```

## Dev entrypoint (operational glue)

1. Start dev app:

```bash
npm run dev
```

2. In browser devtools console run:

```js
const run = window.__runProbeEvalBaseline;
const out = await run();
console.log(out.compact);

// cookie mode:
const outCookie = await run({ capabilityMode: 'verify_session_cookie_flags' });
console.log(outCookie.compact);
```

Compact report includes:
- `snapshotId`
- `baselineId`
- `classification`
- `summary`
- key metrics (`fallbackRate`, `semanticAliveKnownRate`, `reviewerRejectRate`, `mismatchCount`)

## Notes

- If `aliveTargetId`/`deadTargetId` are not provided, related scenarios are skipped.
- `fallback_path_behavior` is executed only when `aliveTargetId` is provided.
- Snapshot format includes: `snapshotId`, `createdAt`, `inputs`, `events`, `metrics`.
- Baseline format (minimal): `baselineId`, `sourceSnapshot`, `metrics`, `createdAt`.
- Baseline inputs should stay controlled/stable; comparison is meaningful only for comparable inputs.
- `inconclusive` is not equal to `regressed` (it usually means mixed/noisy or insufficiently comparable signals).
- For cookie baseline, keep `secureTarget/issuesTarget/unreachableTarget` stable across runs to reduce environment noise.
