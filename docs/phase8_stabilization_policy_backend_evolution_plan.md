# Фаза 8: Stabilization / Policy / Backend-side Evolution Planning

## Цель фазы

Закрепить текущее состояние как **policy-driven architecture**:

1. Явно зафиксировать `standard` / `transitional` / `legacy` пути.
2. Формализовать deprecation boundaries и зависимости.
3. Подготовить безопасный backend-side evolution plan для target envelope model
   без big-bang rewrite, без расширения capability scope и без нового UI.

---

## 1) Inventory текущих путей

### A. Probe stream path

- Preferred execution: `probeStreamPreferred` (`runAgentMinimal` first, legacy fallback). 
- Legacy execution: `probeStreamCapability` (direct capability call).
- Semantic consumer contract: `shouldStopStreamOnProbe` uses `semanticAliveKnown` + `alive`.

### B. verify_session_cookie_flags path

- Preferred execution: `verifySessionCookieFlagsCapability` (`runAgentMinimal` first).
- Legacy execution: `execute_capability(verify_session_cookie_flags)` + `check_session_security` fallback.
- Contract normalization: `normalizeCookieResult` with `contractVersion='cookie_result_v1'`.

### C. Card-kind / selectors / frontend gating

- Derivation + selectors + gating policy in `cardKindAdapter`.
- Envelope-aware kind derivation via `targetEnvelope` (`kind`, `payload`, `version`).
- UI actions gated through `canRun*` helpers with env toggle `VITE_ENABLE_CARD_KIND_GATING`.

### D. Target envelope write/read paths

- Write path wraps targets with `buildTargetEnvelope`.
- Read path unwraps envelope via `unwrapTargetEnvelope` and keeps legacy compatibility.
- Current persistence API still key/value + JSON string; envelope is payload-level convention.

### E. Eval/baseline workflow

- Harness snapshots for probe/cookie paths.
- Baseline builder/comparison/classification.
- Compact reporting includes cookie invariants health and warning surface.

### F. Fallbacks

- Probe: fallback to `probeStreamCapability` on minimal-agent rejection/failure.
- Cookie: fallback to legacy capability and text-based `check_session_security`.
- Card-kind gating has global kill-switch via env flag.

---

## 2) Standard vs Transitional vs Legacy

## Standard (target architecture defaults)

1. **Minimal-agent-first for probe_stream and verify_session_cookie_flags**.
2. **Normalized consumer contracts** for capability outputs (`probeStreamPreferred`, `cookie_result_v1`).
3. **Envelope write-path** for new/updated targets (`buildTargetEnvelope`).
4. **Envelope-aware read compatibility** (`unwrapTargetEnvelope`).
5. **Eval/baseline as regression guardrail**, incl. cookie invariant health in compare/compact output.

## Transitional (kept intentionally during migration)

1. Card-kind gating controlled by env flag (rollout safety).
2. Mixed dataset support (envelope + legacy raw target objects).
3. Legacy capability fallback remains active behind preferred paths.
4. Legacy backend storage model (string payload) used with envelope-on-top strategy.

## Legacy (candidate for eventual deprecation)

1. Direct UI dependency on low-level capability helpers when preferred wrappers exist.
2. Text-parsed cookie fallback semantics from `check_session_security`.
3. Non-envelope target writes (should stop growing).
4. Implicit kind heuristics from legacy fields when envelope kind exists.

---

## 3) Policy table

| Path / Module | Current status | Why | Deprecation needed | Depends on |
|---|---|---|---|---|
| `src/api/capabilities.js::probeStreamPreferred` | **standard** | Minimal-agent-first, normalized result shape, explicit fallback boundary | No (keep as primary) | `runAgentMinimal`, legacy probe helper |
| `src/api/capabilities.js::probeStreamCapability` | **legacy** | Low-level direct capability call retained only for fallback | Yes (later, after backend parity + confidence) | Tauri `execute_capability` |
| `src/api/capabilities.js::verifySessionCookieFlagsCapability` | **standard** | Minimal-agent-first + normalized `cookie_result_v1` contract | No (keep as primary) | `runAgentMinimal`, legacy cookie helpers |
| `src/api/capabilities.js::verifySessionCookieFlagsLegacyCapability` + `check_session_security` parsing | **transitional → legacy** | Required compatibility, but text fallback is semantically weak | Yes (medium priority) | Tauri invoke path, legacy command outputs |
| `src/features/targets/targetEnvelope.js::buildTargetEnvelope` | **standard** | Canonical write path for normalized model introduction | No | `deriveCardKind`, save flows |
| `src/features/targets/targetEnvelope.js::unwrapTargetEnvelope` | **standard** | Required for read compatibility while data is mixed | No | load flows, selectors |
| `src/features/targets/cardKindAdapter.js` gating helpers | **transitional** | Correct policy layer, but rollout still behind env toggle | Maybe (remove toggle later) | UI actions/hooks |
| `src/hooks/useTargets.js` save/load (get_all/read/save) | **transitional** | Envelope-aware, but still loops over key/value API and JSON decode per target | No immediate deprecation; evolve backend under API | Tauri storage commands |
| `src/api/probeEvalHarness.js` + `probeEvalBaselineRunner.js` | **standard** | Regression and contract health safety rail for preferred paths | No | Preferred capabilities + minimal-agent |
| Env toggle `VITE_ENABLE_CARD_KIND_GATING` | **transitional safety control** | Allows gradual rollout and rollback | Yes (remove once stable) | deployment/runtime config |

---

## 4) Safe backend-side evolution plan (без big-bang)

## Принципы

1. **API compatibility first**: keep existing Tauri command names and payload contracts while introducing envelope-first semantics behind them.
2. **Dual-read / single-write policy**:
   - read: support envelope + legacy raw payload;
   - write: always persist envelope format for modified/new targets.
3. **No mass migration requirement**: data normalizes gradually on touch/write.
4. **Feature freeze on scope**: no new capabilities, no UI expansion, no self-learning.

## План шагов

### Step 8.1 — Policy hardening (current doc + guardrails)

- Declare `buildTargetEnvelope` as mandatory write wrapper in all save/update paths.
- Keep `unwrapTargetEnvelope` as mandatory read adapter.
- Keep fallback boundaries explicit and measurable via eval/baseline.

### Step 8.2 — Backend storage adapter (first-class envelope without rewrite)

- Introduce backend-side helper layer (adapter) around existing storage commands:
  - on `save_target`: validate/normalize incoming payload as envelope (if not envelope, wrap server-side);
  - on `read_target`: return canonical envelope JSON (or backward-compatible raw with metadata marker if needed);
  - on `get_all_targets`: optionally add envelope metadata index in future, while preserving current key list return.
- Keep underlying storage engine unchanged.

### Step 8.3 — Read-path stabilization metrics

- Add backend logging counters (non-UI) for:
  - envelope reads,
  - legacy reads,
  - write-time auto-wrap events,
  - envelope validation failures.
- Use counts to decide when legacy path is low enough for stricter policy.

### Step 8.4 — Gradual strictness increase

- After stability window:
  - make non-envelope writes warning-level in logs,
  - then reject clearly malformed payloads,
  - eventually disable implicit legacy write shape.
- Keep legacy read compatibility longer than write compatibility.

---

## 5) Что можно делать уже сейчас без массовой миграции

1. Enforce write-through envelope in all existing frontend save/update paths (already in `useTargets.saveTarget`).
2. Keep lazy migration-by-touch: any edited target becomes envelope.
3. Keep legacy records readable indefinitely through adapter.
4. Compare preferred vs fallback behavior through existing eval/baseline harness before any backend strictness change.

---

## 6) Compatibility layers, которые пока нужно оставить

1. `unwrapTargetEnvelope` dual-read path.
2. Minimal-agent fallback to legacy capability execution.
3. Card-kind gating env toggle for rollback safety.
4. Legacy storage command surface (`get_all_targets`, `read_target`, `save_target`, `delete_target`).

---

## 7) Minimal safest backend-side step (next)

**Самый безопасный следующий шаг:**

- Add backend-side envelope normalization adapter inside existing `save_target`/`read_target` command handling,
  preserving command signatures and storage engine.

Почему это минимально и безопасно:

1. Не меняет UI и capability scope.
2. Не требует массовой миграции данных.
3. Сразу снижает риск drift между frontend envelope policy и backend persisted shape.
4. Оставляет полную обратную совместимость на чтении.

---

## 8) Явные deprecation boundaries

1. New writes in raw legacy target shape — **discouraged now**, deprecate later.
2. Direct probe/cookie low-level helpers in UI flows — **no new callsites**.
3. Cookie text fallback parsing (`check_session_security`) — keep until structured legacy parity is proven stable.
4. Global gating toggle — keep during stabilization window, then retire.

---

## 9) Definition of Done for Phase 8 (planning/policy)

1. Inventory complete for probe/cookie, card-kind gating, envelope write/read, eval/baseline, fallbacks.
2. Policy table with status/dependencies/deprecation direction published.
3. Backend evolution path defined as incremental adapter-based plan.
4. First backend step identified with explicit non-goals (no big-bang rewrite, no new capability/UI).
