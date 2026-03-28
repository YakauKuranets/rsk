# Phase 30.3 — Archive baseline / known-bad pack v1

## Что добавлено

- Новый baseline-ready раннер: `src/api/archiveBaselinePack.js`
  - `runArchiveBaselinePackV1(...)`
  - `archiveBaselineMetrics(...)`
  - `formatArchiveBaselineCompactSummary(...)`
- Controlled pack `archive_baseline_known_bad_pack_v1` с 3 классами кейсов:
  1. `known-good`
  2. `known-bad`
  3. `ambiguous / inconclusive`
- Все кейсы прогоняются через единый consumer-контракт `archive_result_v1` (`normalizeArchiveResultV1`).
- Добавлен dev-runtime запуск через entrypoint:
  - `window.__runArchiveBaselinePackV1(...)`

## Семантика

- Truth-layer только `archive_result_v1`.
- Явное различие `passed | failed | inconclusive`.
- Проверки кейса:
  - class match (`expectedClass` vs `actualClass`)
  - `issuesCount === issues.length`
  - `contractVersion === archive_result_v1`

## Выход раннера

- `caseReports`
- `metrics` (`byStatus`, `byActualClass`, `passRate`, `failedCount`, `inconclusiveCount`)
- compact summary (baseline-friendly строка)

## Ручной запуск (dev console)

```js
const out = await window.__runArchiveBaselinePackV1();
console.log(out.compact);
console.table(out.caseReports);
```

## Назначение шага

- Подготовка к следующему этапу: `Phase 30.3a — Archive edge-case harness v1`.
- Без UI redesign, без archive fuzz, без agent/graph/knowledge-vault интеграций.
