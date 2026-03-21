import { useCallback, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { probeStreamCapability } from '../api/capabilities';
import { pushRuntimeLogEntry, runAgentMinimal } from '../api/tauri';

const SHADOW_PROBE_HIGH_LATENCY_MS = 1200;

function shouldLogShadowProbeRuntime(telemetry) {
  const mismatch = !telemetry.capability_probe_ok || !telemetry.capability_alive;
  const highLatency = telemetry.timing_delta_ms >= SHADOW_PROBE_HIGH_LATENCY_MS;
  return mismatch || highLatency;
}

function buildShadowProbeLogLine(targetId, telemetry) {
  const reasons = [];
  if (!telemetry.capability_probe_ok) reasons.push('capability_error');
  if (telemetry.capability_probe_ok && !telemetry.capability_alive) reasons.push('alive_mismatch');
  if (telemetry.timing_delta_ms >= SHADOW_PROBE_HIGH_LATENCY_MS) reasons.push('high_latency');
  const reasonText = reasons.join(',') || 'n/a';
  return `SHADOW_PROBE_STREAM|target=${targetId}|reasons=${reasonText}|ok=${telemetry.capability_probe_ok}|alive=${telemetry.capability_alive}|latency_ms=${telemetry.timing_delta_ms}|source=${telemetry.source}|status=${telemetry.final_status || 'n/a'}|runId=${telemetry.run_id || 'n/a'}`;
}

export function useStreamManager() {
  const [activeStreams, setActiveStreams] = useState({});
  const [shadowProbeStatus, setShadowProbeStatus] = useState({});

  const startStream = useCallback(async (targetId, rtspUrl) => {
    const wsUrl = await invoke('start_stream', { targetId, rtspUrl });
    setActiveStreams((prev) => ({ ...prev, [targetId]: wsUrl }));
    const probeStartedAt = new Date().toISOString();
    const probeTimer = typeof performance !== 'undefined' ? performance.now() : Date.now();

    // Shadow-mode capability check: read-only diagnostic path.
    // Does not affect legacy UX/behavior and is safe to ignore by consumers.
    void runAgentMinimal({
      targetId,
      mode: 'discovery_mode',
      permitProbeStream: true,
    })
      .then((agent) => {
        if (!agent?.ok) {
          throw new Error((agent?.errors || []).join('; ') || 'minimal-agent-envelope-invalid');
        }

        const probeFinished = typeof performance !== 'undefined' ? performance.now() : Date.now();
        const timingDeltaMs = Math.max(0, Math.round(probeFinished - probeTimer));
        const capabilityOk = agent.finalStatus === 'capability_succeeded';
        const capabilityAlive = capabilityOk && Boolean(agent.capabilityResultSummary?.alive);

        const telemetry = {
          legacy_started: true,
          capability_probe_ok: capabilityOk,
          capability_alive: capabilityAlive,
          final_status: agent.finalStatus,
          run_id: agent.runId,
          reporter_summary: agent.reporterSummary,
          reviewer_approved: Boolean(agent.reviewerVerdict?.approved),
          planner_action_count: Number(agent.plannerDecision?.actionCount ?? 0),
          checkedAt: probeStartedAt,
          timing_delta_ms: timingDeltaMs,
          source: 'minimal-agent',
        };
        setShadowProbeStatus((prev) => ({
          ...prev,
          [targetId]: telemetry,
        }));
        if (shouldLogShadowProbeRuntime(telemetry)) {
          void pushRuntimeLogEntry(buildShadowProbeLogLine(targetId, telemetry)).catch(() => {});
        }
        console.debug('[SHADOW_STREAM_TELEMETRY]', targetId, telemetry);
      })
      .catch(() => {
        // Rollback/compatibility: if minimal agent path fails, keep old probe capability path.
        return probeStreamCapability(targetId, 'discovery_mode');
      })
      .then((cap) => {
        if (!cap) return;
        const probeFinished = typeof performance !== 'undefined' ? performance.now() : Date.now();
        const timingDeltaMs = Math.max(0, Math.round(probeFinished - probeTimer));
        const telemetry = {
          legacy_started: true,
          capability_probe_ok: Boolean(cap.ok),
          capability_alive: Boolean(cap.alive),
          final_status: cap.ok ? 'capability_succeeded' : 'capability_failed',
          run_id: null,
          reporter_summary: cap.message || null,
          reviewer_approved: null,
          planner_action_count: null,
          checkedAt: probeStartedAt,
          timing_delta_ms: timingDeltaMs,
          source: cap.source || 'capability-fallback',
        };
        setShadowProbeStatus((prev) => ({
          ...prev,
          [targetId]: telemetry,
        }));
        if (shouldLogShadowProbeRuntime(telemetry)) {
          void pushRuntimeLogEntry(buildShadowProbeLogLine(targetId, telemetry)).catch(() => {});
        }
        console.debug('[SHADOW_STREAM_TELEMETRY]', targetId, telemetry);
      });

    return wsUrl;
  }, []);

  const stopStream = useCallback(async (targetId) => {
    await invoke('stop_stream', { targetId });
    setActiveStreams((prev) => {
      const next = { ...prev };
      delete next[targetId];
      return next;
    });
    setShadowProbeStatus((prev) => {
      const next = { ...prev };
      delete next[targetId];
      return next;
    });
  }, []);

  const stopAll = useCallback(async () => {
    await invoke('stop_all_streams');
    setActiveStreams({});
    setShadowProbeStatus({});
  }, []);

  return { activeStreams, shadowProbeStatus, startStream, stopStream, stopAll };
}
