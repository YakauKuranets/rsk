# Phase 30.3a — Archive edge-case harness v1

## Что добавлено

- Новый harness: `src/api/archiveEdgeCaseHarness.js`
  - `runArchiveEdgeCaseHarnessV1(...)`
  - `archiveEdgeCaseMetrics(...)`
  - `formatArchiveEdgeCaseCompactSummary(...)`
- Harness расширяет baseline 30.3 (не заменяет):
  - при `includeBaseline=true` внутри отчёта прикладывается результат `runArchiveBaselinePackV1(...)`.

## Edge-case классы (v1)

1. `malformed_success`
2. `partial_access`
3. `timeout_unstable`
4. `parameter_edge`
5. `auth_boundary_ambiguity`

## Семантика

- Truth-layer: только `archive_result_v1` через `normalizeArchiveResultV1(...)`.
- Для каждого кейса есть:
  - `expectedClass`
  - `actualClass`
  - `status` (`passed|failed|inconclusive`)
  - checks (`classMatch`, `issuesCountMatch`, `contractVersionOk`)
- Метрики harness:
  - `byStatus`
  - `byCategory`
  - `byExpectedClass`
  - `byActualClass`
  - `classMatchRate`

## Dev-runtime запуск

```js
const out = await window.__runArchiveEdgeCaseHarnessV1();
console.log(out.compact);
console.table(out.edgeCaseReports);
```

## Границы шага

- Без UI изменений
- Без archive fuzz
- Без graph/vault/agent интеграций
- Подготовка к следующему шагу: `Phase 30.3b — Safe Archive Fuzz Layer v1`
