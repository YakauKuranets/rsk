export const TOOL_EXEC_STATE_KEY = 'hyperion_tool_executor_state_v1';
export const TOOL_EXEC_FAVORITE_SCENARIOS_KEY = 'hyperion_tool_executor_favorite_scenarios_v1';
export const TOOL_EXEC_RECENT_RUNS_KEY = 'hyperion_tool_executor_recent_runs_v1';
export const TOOL_EXEC_USER_SCENARIOS_KEY = 'hyperion_tool_executor_user_scenarios_v1';

function parseJsonSafe(raw) {
  if (typeof raw !== 'string' || !raw.trim()) return null;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

export function restoreToolExecState(raw, { tools = [], presets = {}, fallbackTool = 'nmap' } = {}) {
  const parsed = parseJsonSafe(raw);
  if (!parsed || typeof parsed !== 'object') {
    return {
      tool: fallbackTool,
      timeout: null,
      argsByTool: {},
      selectedPresetByTool: {},
      args: presets[fallbackTool] || '',
    };
  }

  const tool = tools.includes(parsed?.tool) ? parsed.tool : fallbackTool;
  const timeoutRaw = Number(parsed?.timeout);
  const timeout = Number.isFinite(timeoutRaw) && timeoutRaw > 0 ? timeoutRaw : null;
  const argsByTool = parsed?.argsByTool && typeof parsed.argsByTool === 'object' && !Array.isArray(parsed.argsByTool)
    ? parsed.argsByTool
    : {};
  const selectedPresetByTool = parsed?.selectedPresetByTool && typeof parsed.selectedPresetByTool === 'object' && !Array.isArray(parsed.selectedPresetByTool)
    ? parsed.selectedPresetByTool
    : {};

  return {
    tool,
    timeout,
    argsByTool,
    selectedPresetByTool,
    args: typeof argsByTool?.[tool] === 'string' ? argsByTool[tool] : (presets[tool] || ''),
  };
}

export function restoreObjectArray(raw, limit = 20) {
  const parsed = parseJsonSafe(raw);
  if (!Array.isArray(parsed)) return [];
  return parsed.filter((item) => item && typeof item === 'object').slice(0, limit);
}

export function restoreFavoriteScenarioIds(raw, allowedIds = []) {
  const parsed = parseJsonSafe(raw);
  if (!Array.isArray(parsed)) return [];
  const allowed = new Set(allowedIds.filter((id) => typeof id === 'string'));
  return parsed.filter((id) => typeof id === 'string' && allowed.has(id));
}
