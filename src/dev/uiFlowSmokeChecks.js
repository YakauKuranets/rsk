const PASS = 'PASS';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function createInitialState() {
  return {
    targets: [
      { id: 'cam-1', type: 'camera', name: 'Yard Cam', host: '10.0.0.21:554', url: 'rtsp://10.0.0.21/stream1' },
      { id: 'hub-1', type: 'hub', name: 'Main HUB', host: '10.0.0.10:8080' },
    ],
    selectedTargetId: null,
    activeScenario: null,
    customScenario: null,
    recentRuns: [],
    workChain: { steps: [], currentIndex: 0, done: false },
  };
}

function selectTarget(state, targetId) {
  return { ...state, selectedTargetId: targetId };
}

function removeTarget(state, targetId) {
  const targets = state.targets.filter((t) => t.id !== targetId);
  return {
    ...state,
    targets,
    selectedTargetId: state.selectedTargetId === targetId ? (targets[0]?.id || null) : state.selectedTargetId,
  };
}

function applyQuickScenario(state, key) {
  const quick = {
    fast_stream: ['stream_open', 'isapi_info'],
    archive_focus: ['archive_search', 'archive_open'],
  };
  const steps = quick[key] || [];
  return {
    ...state,
    activeScenario: key,
    workChain: { steps, currentIndex: 0, done: steps.length === 0 },
  };
}

function applyCustomScenario(state, scenario) {
  const steps = Array.isArray(scenario?.steps) ? scenario.steps.filter(Boolean) : [];
  return {
    ...state,
    customScenario: { name: scenario?.name || 'Пользовательский', steps },
    workChain: { steps, currentIndex: 0, done: steps.length === 0 },
  };
}

function saveRecentRun(state, run) {
  return { ...state, recentRuns: [run, ...state.recentRuns].slice(0, 10) };
}

function applyRecentRunToDraft(state, runId) {
  const run = state.recentRuns.find((r) => r.id === runId);
  return {
    ...state,
    customScenario: run ? { name: `Повтор ${run.label}`, steps: [...(run.steps || [])] } : state.customScenario,
  };
}

function repeatRecentRun(state, runId) {
  const run = state.recentRuns.find((r) => r.id === runId);
  if (!run) return state;
  return {
    ...state,
    workChain: { steps: [...(run.steps || [])], currentIndex: 0, done: false },
  };
}

function applyRecentRunToCurrentTarget(state, runId) {
  const run = state.recentRuns.find((r) => r.id === runId);
  if (!run) return state;
  return {
    ...state,
    recentRuns: state.recentRuns.map((r) => (r.id === runId ? { ...r, appliedTargetId: state.selectedTargetId } : r)),
  };
}

function moveWorkChainNext(state) {
  const nextIndex = state.workChain.currentIndex + 1;
  const done = nextIndex >= state.workChain.steps.length;
  return {
    ...state,
    workChain: {
      ...state.workChain,
      currentIndex: done ? state.workChain.steps.length : nextIndex,
      done,
    },
  };
}

function hasWebHint(target) {
  const text = `${target?.url || ''} ${target?.endpoint || ''} ${target?.name || ''} ${target?.host || ''}`.toLowerCase();
  return /https?:\/\/|\bwww\.|:80\b|:443\b|web|portal|admin/.test(text);
}

function buildActionStatuses(target) {
  const isHub = String(target?.type || '').toLowerCase() === 'hub';
  const hasHost = String(target?.host || target?.ip || '').trim().length > 0;
  const webHint = hasWebHint(target);
  return {
    stream: isHub ? 'Ограничено для HUB' : hasHost ? 'Готово' : 'Нет host/ip',
    isapi: hasHost ? (webHint ? 'Готово' : 'Нужен web-endpoint') : 'Нет host/ip',
    onvif: hasHost ? (webHint ? 'Готово' : 'Нужен web-endpoint') : 'Нет host/ip',
    archiveSearch: hasHost ? (webHint ? 'Готово' : 'Нужен web-endpoint') : 'Нет host/ip',
    archive: hasHost ? 'Готово' : 'Нет host/ip',
  };
}

function buildAggregate(statuses) {
  const values = Object.values(statuses);
  const readyCount = values.filter((s) => s === 'Готово').length;
  const limitedCount = values.filter((s) => s === 'Ограничено для HUB').length;
  const blockedCount = values.length - readyCount - limitedCount;
  return { readyCount, limitedCount, blockedCount };
}

function buildProfile(target, statuses) {
  const typeText = String(target?.type || '').toLowerCase();
  const text = `${target?.name || ''} ${target?.host || ''} ${target?.url || ''} ${target?.endpoint || ''}`.toLowerCase();
  if (typeText === 'hub') return { label: 'HUB', note: statuses.archive };
  if (/(cam|camera|nvr|dvr|rtsp|onvif|554)/.test(text)) return { label: 'Камера / NVR', note: statuses.stream };
  if (/https?:\/\/|\bwww\.|web|portal|admin/.test(text)) return { label: 'Web-цель', note: statuses.isapi };
  if (String(target?.host || '').trim()) return { label: 'Сетевой узел', note: statuses.stream };
  return { label: 'Неопределённая цель', note: 'Недостаточно данных для запуска' };
}

export function runUiFlowSmokeChecks() {
  let state = createInitialState();

  state = selectTarget(state, 'cam-1');
  assert(state.selectedTargetId === 'cam-1', 'Выбор цели: selectedTarget не обновился');

  state = removeTarget(state, 'cam-1');
  assert(state.selectedTargetId === 'hub-1', 'Удаление выбранной цели: не произошёл fallback на доступную цель');

  state = applyQuickScenario(state, 'fast_stream');
  assert(state.workChain.steps.length === 2 && state.activeScenario === 'fast_stream', 'Быстрый сценарий: шаги не применились');

  state = applyCustomScenario(state, { name: 'Мой сценарий', steps: ['isapi_info', 'archive_search'] });
  assert(state.customScenario?.steps?.length === 2, 'Пользовательский сценарий: шаги не сохранились');

  state = saveRecentRun(state, { id: 'run-1', label: 'Проверка камеры', steps: ['stream_open', 'archive_search'] });
  state = applyRecentRunToDraft(state, 'run-1');
  assert(state.customScenario?.name?.includes('Повтор'), 'Недавний запуск → подставить: не обновлён draft');

  state = repeatRecentRun(state, 'run-1');
  assert(state.workChain.steps[0] === 'stream_open', 'Недавний запуск → повторить: не поднялись шаги цепочки');

  state = applyRecentRunToCurrentTarget(state, 'run-1');
  assert(state.recentRuns[0]?.appliedTargetId === state.selectedTargetId, 'Недавний запуск → на текущую цель: не зафиксирована цель');

  state = moveWorkChainNext(state);
  assert(state.workChain.currentIndex === 1 && !state.workChain.done, 'Цепочка: переход к следующему шагу не сработал');
  state = moveWorkChainNext(state);
  assert(state.workChain.done, 'Цепочка: завершение не сработало');

  const hubTarget = state.targets.find((t) => t.id === 'hub-1');
  const statuses = buildActionStatuses(hubTarget);
  const aggregate = buildAggregate(statuses);
  const profile = buildProfile(hubTarget, statuses);
  assert(profile.label === 'HUB', 'Профиль совместимости: неверная классификация HUB');
  assert(aggregate.limitedCount >= 1, 'Агрегированная сводка: ожидалось хотя бы одно ограниченное действие');
  assert(typeof statuses.archive === 'string', 'Reason/status-подсказки: статус архива не рассчитан');

  return {
    status: PASS,
    checks: [
      'выбор цели',
      'удаление выбранной цели',
      'быстрый сценарий',
      'пользовательский сценарий',
      'недавний запуск (подставить/повторить/на текущую цель)',
      'рабочая цепочка (шаг → следующий → завершение)',
      'профиль/reason-status/агрегированная сводка',
    ],
  };
}
