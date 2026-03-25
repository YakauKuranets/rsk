# Phase 30.3b — Safe Archive Fuzz Layer v1

## Что добавлено

- Новый validation-oriented fuzz слой: `src/api/archiveSafeFuzzLayer.js`
  - `runSafeArchiveFuzzLayerV1(...)`
  - `archiveFuzzMetrics(...)`
  - `formatArchiveFuzzCompactSummary(...)`
- Replayable seeds (`DEFAULT_ARCHIVE_FUZZ_SEEDS_V1`) и controlled mutation classes.
- Safe limits v1:
  - `max_mutations_per_run`
  - `max_runtime_ms`

## Mutation classes (минимум)

1. `time_range`
2. `channel`
3. `parameter_presence`
4. `malformed_safe_success_like`
5. `auth_boundary_adjacent`

## Семантика

- Все mutation outputs проходят через `normalizeArchiveResultV1(...)`.
- Для каждой мутации есть:
  - `seed`
  - `mutationType`
  - `expectedBehavior`
  - `actualBehavior`
  - `status` (`passed|failed|inconclusive`)
- Baseline continuity не ломается:
  - при `includeContinuity=true` добавляются baseline + edge-case отчёты.

## Dev-runtime запуск

```js
const out = await window.__runSafeArchiveFuzzLayerV1();
console.log(out.compact);
console.table(out.mutationReports);
```

## Границы шага

- Без UI изменений
- Без graph/vault/agent интеграций
- Без big-bang rewrite
- Следующий шаг: `Phase 30.4 — Auth contract normalization`
