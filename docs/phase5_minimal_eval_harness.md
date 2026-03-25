# Phase 5: Minimal Eval/Review Harness (probe_stream only)

## Scope

- For minimal-agent eval paths: `probe_stream` and `verify_session_cookie_flags`.
- No self-learning.
- No new capabilities.
- No backend storage/model rewrite.

## Harness entry

- `runProbeStreamEvalHarness(...)` in `src/api/probeEvalHarness.js`.
- `runVerifySessionCookieEvalHarness(...)` in `src/api/probeEvalHarness.js`.
- `runCookieResultInvariantChecks(...)` in `src/api/probeEvalHarness.js`.
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

### Cookie contract invariants (`cookie_result_v1`)

- `issues` is always an array of strings
- `issuesCount` matches `issues.length`
- `fallbackUsed` is consistent with `source`
- `inconclusive` does not conflict with confident states
- preferred and forced-fallback results expose the same consumer shape

## Session lifecycle known-bad pack v1 (controlled)

- Pack id: `session_lifecycle_known_bad_pack_v1`
- Cases:
  1. `known_good_local_tls` (`https://localhost`) — expected `secure`
  2. `known_bad_local_http` (`http://localhost`) — expected `insecure`
  3. `ambiguous_unreachable_local` (`http://127.0.0.1:1`) — expected `inconclusive`
- Runtime path: `verifySessionCookieFlagsCapability(...)` (preferred minimal-agent + legacy fallback boundary preserved).

### Manual run (browser console)

```js
const out = await window.__runSessionLifecycleKnownBadPackV1();
console.log(out.compact);
console.table(out.reports);
```

Interpretation:
- `status=passed` — expectation confirmed.
- `status=failed` — safety regression candidate (must investigate).
- `status=inconclusive` — environment/case ambiguity; not auto-pass and not auto-fail.

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
console.log(cookieReport.invariants);
```

## Cookie invariants-only check

```js
import { runCookieResultInvariantChecks } from '/src/api/probeEvalHarness.js';

const invariant = await runCookieResultInvariantChecks({
  target: 'https://localhost',
  mode: 'discovery_mode',
});
console.log(invariant.allPassed, invariant);
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

// cookie mode with explicit profile:
const outCookieTls = await run({
  capabilityMode: 'verify_session_cookie_flags',
  cookieProfile: 'local_tls',
});
console.log(outCookieTls.compact);
```

Compact report includes:
- `snapshotId`
- `baselineId`
- `classification`
- `summary`
- key metrics (mode-dependent):
  - probe mode: `fallbackRate`, `semanticAliveKnownRate`, `reviewerRejectRate`, `mismatchCount`
  - cookie mode: `secureRate`, `issuesDetectedRate`, `reviewerRejectRate`, `inconclusiveFailureRate`

## Notes

- If `aliveTargetId`/`deadTargetId` are not provided, related scenarios are skipped.
- `fallback_path_behavior` is executed only when `aliveTargetId` is provided.
- Snapshot format includes: `snapshotId`, `createdAt`, `inputs`, `events`, `metrics`.
- Baseline format (minimal): `baselineId`, `sourceSnapshot`, `metrics`, `createdAt`.
- Baseline inputs should stay controlled/stable; comparison is meaningful only for comparable inputs.
- `inconclusive` is not equal to `regressed` (it usually means mixed/noisy or insufficiently comparable signals).
- For cookie baseline, keep `secureTarget/issuesTarget/unreachableTarget` stable across runs to reduce environment noise.
- Runtime consumer boundary for cookie checks: `verifySessionCookieFlagsCapability(...)` now prefers minimal-agent path and falls back to legacy capability/session checker path for compatibility.
- Runtime cookie consumer contract is normalized as `cookie_result_v1`:
  - stable fields: `ok`, `source`, `secure`, `issues`, `issuesCount`, `runId`, `reporterSummary`, `evidenceRefs`
  - explicit semantics: `fallbackUsed`, `inconclusive`

## Controlled cookie baseline profiles

### `local_tls`

- profile id: `local_tls`
- assumptions:
  - `https://localhost` reachable (secure cookie candidate)
  - `http://localhost` reachable (issues contrast)
  - `http://127.0.0.1:1` remains unreachable (failure path)
- intent: balanced local profile for secure/issues/failure coverage.

### `local_plain_http`

- profile id: `local_plain_http`
- assumptions:
  - `http://localhost` reachable
  - HTTPS may be absent in local setup
  - `http://127.0.0.1:1` remains unreachable
- intent: emphasize issues-detected + failure behavior in plain local environments.
