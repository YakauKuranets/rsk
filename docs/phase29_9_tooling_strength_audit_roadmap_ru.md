# Фаза 29.9 — аудит силы инструментов и roadmap усиления (без UI)

## 1) Этап: discovery / target intake
### 2) Подэтапы
- **1.1 External intake**: `discover_external_assets` (Shodan + crt.sh + DNS).
- **1.2 Local intake / camera sweep**: `unified_camera_scan` + `expand_target_input`.
- **1.3 Spider target acquisition**: phase 0 в `spider_full_scan`.

### 3) Что найдено
- Intake уже многоканальный: внешние источники (Shodan/crt.sh/DNS) + локальный sweep портов/RTSP/ONVIF/FTP.
- CIDR/диапазоны ограничены (до ~1024 адресов), что снижает риск runaway-сканов, но ограничивает coverage enterprise-сетей.
- В spider intake есть только фиксированный набор портов/сигнатур и частично эвристический fingerprint.

### 4) Сильные инструменты
- Быстрая конверсия target->asset list через несколько источников.
- Защитные timeouts и дедупликация на intake-уровне.

### 5) Слабые инструменты
- Нет глубокой нормализации/валидации identity target (asset identity graph отсутствует).
- Vendor/fingerprint эвристичны, без confidence calibration на known-bad/known-good наборах.

### 6) Где ложное чувство безопасности
- “Нашёл активы” != “понял attack surface полностью”: данные внешних API и порт-сигнатуры неполны.

### 7) Coverage matrix (для этапа)
- **Сильно**: internet footprint (subdomain/cert hints), base-port visibility.
- **Слабо**: скрытые сервисы, нестандартные порты, multi-homed asset correlation.

### 8) Blind spots
- Non-HTTP admin planes, VLAN-segmented устройства, IPv6-first сегменты.

### 9) Что усиливать первым
- Ввести target identity envelope v2 (stable asset keys + source confidence + first/last seen).

### 10) Что усиливать вторым
- Добавить активный/пассивный source scoring и conflict resolution между Shodan/crt.sh/DNS/local sweep.

### 11) Что пока не трогать
- UI target cards/flows.

### 12) Known-bad validation gaps
- Нет репрезентативного lab-pack для ложных fingerprint (fake banners, honeypot-like responses).

---

## 1) Этап: surface scanning
### 2) Подэтапы
- Tool executor (nmap/nikto/nuclei/ffuf/etc.).
- Spider dir/api/js endpoint discovery.
- Camera scan deep mode (RTSP/archive probes).

### 3) Что найдено
- Есть whitelist по тулзам и базовая защита от явно опасных аргументов.
- Findings extraction построен на regex/эвристиках; хорош для triage, слаб для “security-grade verdict”.

### 4) Сильные инструменты
- Широкий набор интегрированных CLI-сканеров.
- Быстрый operator loop: run -> findings -> replay.

### 5) Слабые инструменты
- Нет строгого DSL-профиля запуска и policy-tier guardrails для каждого инструмента.
- Нет quality gates по ложноположительным/ложноотрицательным на known datasets.

### 6) Где ложное чувство безопасности
- “Ничего не найдено” часто означает ограниченность шаблонов/словари/аргументов, а не отсутствие риска.

### 7) Coverage matrix
- **Сильно**: быстрый широкий triage.
- **Слабо**: deep protocol semantics, authenticated surface mapping consistency.

### 8) Blind spots
- Business-logic flaws, multi-step auth workflows, stateful race/abuse классы.

### 9) Что усиливать первым
- Библиотека reproducible scan profiles (safe/balanced/deep) с версионированием и expected coverage.

### 10) Что усиливать вторым
- Постпроцессор findings -> normalized evidence schema + confidence score.

### 11) Что пока не трогать
- Existing panel UX.

### 12) Known-bad validation gaps
- Нет “adversarial outputs pack” для устойчивости extraction (noise, truncation, localization variants).

---

## 1) Этап: auth-related checks
### 2) Подэтапы
- BAS scenarios (default creds, unauth API/RTSP).
- credential_auditor (RTSP credential probing).
- Частично spider auth headers parsing.

### 3) Что найдено
- Auth-аудит есть, но в значительной степени эвристический/операторский.
- BAS часть сценариев отмечена как “manual test required” (неполная автоматизация).

### 4) Сильные инструменты
- Практичная проверка weak/default credentials.
- Адаптивные задержки и ограничение конкурентности в credential audit.

### 5) Слабые инструменты
- Нет формальной модели auth-state machine по вендорам/прошивкам.
- Мало строгих oracle-критериев “bypass vs expected denial”.

### 6) Где ложное чувство безопасности
- “blocked” по одному endpoint не доказывает отсутствие альтернативного пути входа.

### 7) Coverage matrix
- **Сильно**: default creds / unauth endpoints (базовый уровень).
- **Слабо**: MFA/session fixation/CSRF/chained-auth bypass.

### 8) Blind spots
- Token replay, lockout policy abuse, cross-protocol auth desync.

### 9) Что усиливать первым
- Vendor-specific auth test packs (Hikvision/Dahua/XM/etc.) с expected outcome матрицей.

### 10) Что усиливать вторым
- Unified auth evidence model (challenge, response, headers, cookies, retry traces).

### 11) Что пока не трогать
- Existing operator-grade BAS UI.

### 12) Known-bad validation gaps
- Нет controlled lab pack с intentionally misconfigured auth chains.

---

## 1) Этап: session / cookie checks
### 2) Подэтапы
- `verify_session_cookie_flags` capability через minimal-agent.
- Legacy fallback через `check_session_security` text parsing.
- Invariant checks (`cookie_result_v1`).

### 3) Что найдено
- Есть хороший контрактный слой нормализации результата и инварианты формы.
- Но security semantics узкие: фактически проверяются только cookie flags на одном ответе.

### 4) Сильные инструменты
- Чёткий preferred/fallback boundary.
- Invariant checks + baseline compare уже внедрены.

### 5) Слабые инструменты
- Нет path-level coverage по login/logout/refresh flows.
- Нет проверки session fixation / token rotation / scope leakage.

### 6) Где ложное чувство безопасности
- “secure=true” может означать только наличие флагов, но не безопасную session модель.

### 7) Coverage matrix
- **Сильно**: transport-level cookie hygiene signal.
- **Слабо**: full session lifecycle security.

### 8) Blind spots
- Re-auth boundary, CSRF token coupling, logout invalidation consistency.

### 9) Что усиливать первым
- Session lifecycle probe pack (login->refresh->logout->reuse token checks).

### 10) Что усиливать вторым
- Structured legacy fallback parity (уйти от text parsing как primary signal).

### 11) Что пока не трогать
- Existing invariant harness API surface (сначала расширять сценарии).

### 12) Known-bad validation gaps
- Нет набора cookie/session anti-pattern fixtures (missing rotation, weak SameSite policy variants).

---

## 1) Этап: probe_stream
### 2) Подэтапы
- capability `probe_stream`.
- preferred path `probeStreamPreferred` (minimal-agent first).
- fallback path `probeStreamCapability`.

### 3) Что найдено
- Probe_stream даёт бинарный liveness по состоянию процесса стрима, а не по end-to-end доступности медиаданных.
- semanticAliveKnown и fallback telemetry уже есть (это зрелый контроль качества маршрута).

### 4) Сильные инструменты
- Очень чистый контракт статусов (reviewer_rejected / capability_succeeded / capability_failed).
- Eval harness умеет ловить mismatch-индикаторы и fallback-rate drift.

### 5) Слабые инструменты
- Liveness завязан на локальный process status, мало сигналов про реальную quality/continuity stream.

### 6) Где ложное чувство безопасности
- “alive=true” может означать только живой ffmpeg-процесс, не факт валидного video pipeline.

### 7) Coverage matrix
- **Сильно**: orchestration-level health.
- **Слабо**: media-plane integrity (fps/packet loss/freeze/auth expiry).

### 8) Blind spots
- Stalled streams, keyframe starvation, intermittent auth expiration.

### 9) Что усиливать первым
- Добавить media-heartbeat assertions (frame cadence, decode success, timeout anomalies).

### 10) Что усиливать вторым
- Probe reason taxonomy (dead/stalled/auth_failed/network_jitter).

### 11) Что пока не трогать
- Current minimal-agent envelope contract.

### 12) Known-bad validation gaps
- Нет synthetic bad-stream pack (stutter, freeze, half-open TCP, RTSP auth flip).

---

## 1) Этап: archive-related paths
### 2) Подэтапы
- unified archive search (ISAPI->ONVIF->XM->FTP).
- archive endpoint probe.
- archive download/export fallback paths.

### 3) Что найдено
- Есть практичный multi-protocol fallback chain.
- ONVIF search в unified path пока заглушка-делегирование (не полноценная реализация в этом слое).

### 4) Сильные инструменты
- Высокая операционная живучесть за счёт fallback.
- Богатая логика загрузки/экспорта в main.rs.

### 5) Слабые инструменты
- Много protocol-specific веток; сложно доказывать одинаковые security свойства между ветками.
- Слабая унификация evidence/trace по стадиям fallback.

### 6) Где ложное чувство безопасности
- Успешный fallback download != корректность/полнота таймлайна архива.

### 7) Coverage matrix
- **Сильно**: pragmatic retrieval across vendor variance.
- **Слабо**: integrity guarantees (timeline completeness, tamper evidence).

### 8) Blind spots
- Gap detection в архиве, timezone skew, duplicate/overlap fragments.

### 9) Что усиливать первым
- Unified archive evidence schema (attempt graph + reasons + integrity checks).

### 10) Что усиливать вторым
- Archive consistency validator (time continuity, checksum/chunk map).

### 11) Что пока не трогать
- Existing fallback ordering.

### 12) Known-bad validation gaps
- Нет lab-pack с битым ONVIF/ISAPI XML, ложными URI, time-skew и partial-export кейсами.

---

## 1) Этап: capability adapters
### 2) Подэтапы
- `execute_capability` маршрутизация.
- allowed_modes policy.
- capability output envelopes.

### 3) Что найдено
- Контракт capability-слоя аккуратный и расширяемый.
- Набор capabilities пока узкий (probe_stream / archive_search / cookie_flags).

### 4) Сильные инструменты
- Явная валидация mode + input.
- Единый response envelope с typed data/error.

### 5) Слабые инструменты
- Недостаточно capability-level semantic checks и confidence metadata.

### 6) Где ложное чувство безопасности
- Typed envelope может создавать ощущение зрелости даже при узкой глубине самой проверки.

### 7) Coverage matrix
- **Сильно**: contract stability.
- **Слабо**: breadth/depth of capabilities.

### 8) Blind spots
- No capability for auth-lifecycle, stream-quality, archive-integrity, adversarial fingerprint checks.

### 9) Что усиливать первым
- Capability taxonomy v2 (signal class, confidence, false-safety caveats обязательны в ответе).

### 10) Что усиливать вторым
- Добавить 2-3 высокоприоритетные capabilities без ломки контракта v1.

### 11) Что пока не трогать
- Текущий mode policy (Discovery/Verified/Analysis) как каркас.

### 12) Known-bad validation gaps
- Нет contract-level chaos tests (unexpected payload nesting, partial data, stale evidence refs).

---

## 1) Этап: minimal-agent driven paths
### 2) Подэтапы
- planner/reviewer/execute/reporter minimal loop.
- reviewer permits.
- normalized tauri envelope на фронте.

### 3) Что найдено
- Архитектурно сильный “policy-first” контур.
- Но planner фактически single-action и ограничен 2 capability-ветками.

### 4) Сильные инструменты
- Прозрачный trace envelope и финальные статусы.
- Хорошая совместимость с eval/baseline.

### 5) Слабые инструменты
- Низкая expressiveness planner-а (нет multi-step reasoning с проверкой гипотез).

### 6) Где ложное чувство безопасности
- Наличие “agent” не означает coverage: сейчас это узкий orchestration wrapper.

### 7) Coverage matrix
- **Сильно**: governance and explainability.
- **Слабо**: autonomous testing depth.

### 8) Blind spots
- Inter-capability chaining quality, conflict resolution between signals.

### 9) Что усиливать первым
- Планировщик v1.5: ограниченный multi-step (max 2-3 actions) с hard guardrails.

### 10) Что усиливать вторым
- Reviewer policy packs per mode + per lab profile.

### 11) Что пока не трогать
- Envelope поля и статусная модель.

### 12) Known-bad validation gaps
- Нет replay-набора reviewer edge-cases (permit mismatch, missing args, contradictory signals).

---

## 1) Этап: eval / baseline / invariant checks
### 2) Подэтапы
- probe/cookie harness.
- baseline build + snapshot compare.
- contract health / invariants.

### 3) Что найдено
- Это один из самых зрелых контуров: есть snapshotы, delta-метрики, invariant health.
- Но охват ограничен двумя capability направлениями.

### 4) Сильные инструменты
- Явные дельты и guardrail на unsafe compare при провале инвариантов.

### 5) Слабые инструменты
- Нет аналогичного eval-контура для archive/auth-lifecycle/surface-quality.

### 6) Где ложное чувство безопасности
- “Бейзлайн зелёный” по probe/cookie не говорит о здоровье остальных подсистем.

### 7) Coverage matrix
- **Сильно**: regression detection для текущих двух контрактов.
- **Слабо**: cross-tool regression и attack-class regression.

### 8) Blind spots
- Нет unified score по threat-class coverage.

### 9) Что усиливать первым
- Расширить harness на archive + auth-lifecycle + scanner extraction quality.

### 10) Что усиливать вторым
- Ввести threat-class weighted baseline score.

### 11) Что пока не трогать
- Текущую схему snapshot/baseline compare.

### 12) Known-bad validation gaps
- Нет стандартизированного known-bad lab pack как входа в harness.

---

## 5 самых слабых мест инструментального контура
1. **Session security слишком узкая по семантике** (flags-only, без lifecycle).
2. **Probe_stream ограничен process-level liveness** без media-quality oracle.
3. **Archive fallback мощный, но слабый integrity-контроль.**
4. **Auth/BAS частично эвристичен и не полностью автоматизирован.**
5. **Planner minimal-agent пока слишком узкий (single-step-ish).**

## 5 самых сильных мест
1. **Capability envelope + mode policy** как стабильный контракт.
2. **Minimal-agent traceability** (planner/reviewer/final status).
3. **Eval/baseline/invariant discipline** для probe/cookie.
4. **Практичная multi-tool scanner интеграция** с быстрым triage.
5. **Multi-protocol archive fallback** (операционная живучесть в лабе).

## Blind spots (глобально)
- Session lifecycle security.
- Media-plane integrity.
- Archive timeline integrity / tamper evidence.
- Cross-protocol auth desync.
- Threat-class coverage accounting (what is truly covered vs not).

## Что усиливать первым / вторым / позже
### Первым (P0)
- Session lifecycle capability + known-bad fixtures.
- Probe_stream media-heartbeat checks.
- Archive integrity validator (continuity + checksum metadata).

### Вторым (P1)
- Planner v1.5 (2-3 step chaining) + reviewer policy packs.
- Capability taxonomy v2 (confidence + false-safety caveats).
- Extended eval harness for archive/auth/scanner outputs.

### Позже (P2)
- Threat-class weighted score and governance dashboards.
- Advanced cross-source asset correlation engine.

## 13) Глобальный план по фазам и подфазам (без big-bang rewrite)
### Фаза A — Signal Integrity Foundation
- A.1: Session lifecycle checks (login/refresh/logout/reuse).
- A.2: Stream media-heartbeat reason taxonomy.
- A.3: Archive integrity evidence schema.

### Фаза B — Contract Expansion (safe incremental)
- B.1: Новые capabilities в v1 envelope без breaking changes.
- B.2: Capability taxonomy v2 fields (confidence, caveats, threatClass).
- B.3: Legacy fallback parity hardening.

### Фаза C — Eval Expansion
- C.1: Harness scenarios for new capabilities.
- C.2: Known-bad/known-good lab packs as mandatory eval inputs.
- C.3: Baseline compare with threat-class deltas.

### Фаза D — Minimal Agent Depth
- D.1: Planner v1.5 bounded chaining.
- D.2: Reviewer policy packs by mode/lab profile.
- D.3: Conflict-resolution rules between capabilities.

### Фаза E — Stabilization & Governance
- E.1: Regression gates in CI for invariant + baseline drift.
- E.2: Coverage scorecard per threat class.
- E.3: Sunset weakest legacy fallbacks after parity.

## 14) Риски
- Переусложнение capability contracts без реального прироста signal quality.
- Рост latency из-за deeper probes.
- Неполная репрезентативность lab-pack (danger: переобучение на лабораторные кейсы).

## 15) Прогресс %
- Discovery of current state: **100%** (по текущему коду и документированным harness-путям).
- Roadmap definition: **100%**.
- Implementation of roadmap items: **0%** (на этой фазе intentionally только анализ + план).

## 16) Следующий минимальный практический шаг
- **Step 29.9.1 (маленький, не-UI):** добавить новый dev-harness сценарий `session_lifecycle_known_bad_pack_v1` (3-5 кейсов) и вывести его метрики в текущий baseline runner без изменения UI.
