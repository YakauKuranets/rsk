# Phase 30.2 — Archive contract normalization (`archive_result_v1`)

## Что сделано

- Введён единый normalizer consumer-контракта: `src/api/archiveResultContract.js`.
- Добавлена интеграция в archive-related handlers в `src/App.jsx`:
  - ISAPI search
  - ONVIF search
  - archive export probe
  - ISAPI export/download job
  - ONVIF export/download
- Для каждого пути теперь формируется нормализованный `archive_result_v1` и публикуется в runtime logs c маркером `ARCHIVE_RESULT_V1|...`.

## Минимальные поля `archive_result_v1`

- `target_id`
- `archive_path_type`
- `search_supported`
- `search_requires_auth`
- `export_supported`
- `export_requires_auth`
- `partial_access_detected`
- `timeout_detected`
- `integrity_status`
- `issues`
- `issuesCount`
- `evidenceRefs`
- `confidence`
- `resultClass`

Дополнительно: `contractVersion = "archive_result_v1"`.

## Инварианты

- `issues` всегда массив строк.
- `issuesCount` всегда равен `issues.length`.
- `resultClass` нормализован в одно из: `passed | failed | inconclusive`.
- `evidenceRefs` additive и безопасно расширяемы.

## Как запускать/проверять

1. Запустить приложение.
2. Выполнить любой archive flow:
   - ISAPI search
   - ONVIF search
   - probe export endpoints
   - ISAPI download/export
   - ONVIF download
3. Открыть Runtime Logs и найти строки с `ARCHIVE_RESULT_V1|...`.
4. Проверить, что для happy-path и error/timeout-path формируются корректные `resultClass`, `issuesCount` и `confidence`.

## Зачем это нужно

- Убирает прямую зависимость consumer-слоя от сырого смешанного вывода.
- Готовит архивный контур к Phase 30.3 (baseline / known-bad pack v1) без UI-редизайна.
