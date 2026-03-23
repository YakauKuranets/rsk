import { useCallback, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { probeStreamPreferred } from '../api/capabilities';
import { pushRuntimeLogEntry } from '../api/tauri';

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

    // Preferred probe policy for UI/workflow consumers:
    // use probeStreamPreferred (minimal-agent first, legacy fallback internalized in API layer).
    // Shadow-mode capability check: read-only diagnostic path.
    // Does not affect legacy UX/behavior and is safe to ignore by consumers.
    void probeStreamPreferred(targetId, 'discovery_mode')
      .then((probe) => {
        const probeFinished = typeof performance !== 'undefined' ? performance.now() : Date.now();
        const timingDeltaMs = Math.max(0, Math.round(probeFinished - probeTimer));
        const capabilityOk = probe.finalStatus === 'capability_succeeded';
        const capabilityAlive = capabilityOk && Boolean(probe.alive);

        const telemetry = {
          legacy_started: true,
          capability_probe_ok: capabilityOk,
          capability_alive: capabilityAlive,
          final_status: probe.finalStatus || (capabilityOk ? 'capability_succeeded' : 'capability_failed'),
          run_id: probe.runId || null,
          reporter_summary: probe.reporterSummary || null,
          reviewer_approved: probe.reviewerApproved ?? null,
          planner_action_count: probe.plannerActionCount ?? null,
          checkedAt: probeStartedAt,
          timing_delta_ms: timingDeltaMs,
          source: probe.source || 'probe-preferred',
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
        const probeFinished = typeof performance !== 'undefined' ? performance.now() : Date.now();
        const timingDeltaMs = Math.max(0, Math.round(probeFinished - probeTimer));
        const telemetry = {
          legacy_started: true,
          capability_probe_ok: false,
          capability_alive: false,
          final_status: 'capability_failed',
          run_id: null,
          reporter_summary: 'probe-preferred-error',
          reviewer_approved: null,
          planner_action_count: null,
          checkedAt: probeStartedAt,
          timing_delta_ms: timingDeltaMs,
          source: 'probe-preferred-error',
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
