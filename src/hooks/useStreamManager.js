import { useCallback, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { probeStreamCapability } from '../api/capabilities';
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
  return `SHADOW_PROBE_STREAM|target=${targetId}|reasons=${reasonText}|ok=${telemetry.capability_probe_ok}|alive=${telemetry.capability_alive}|latency_ms=${telemetry.timing_delta_ms}|source=${telemetry.source}`;
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
    void probeStreamCapability(targetId, 'discovery_mode')
      .then((cap) => {
        const probeFinished = typeof performance !== 'undefined' ? performance.now() : Date.now();
        const timingDeltaMs = Math.max(0, Math.round(probeFinished - probeTimer));
        const telemetry = {
          legacy_started: true,
          capability_probe_ok: cap.ok,
          capability_alive: Boolean(cap.alive),
          checkedAt: probeStartedAt,
          timing_delta_ms: timingDeltaMs,
          source: cap.source || 'capability',
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
          checkedAt: probeStartedAt,
          timing_delta_ms: timingDeltaMs,
          source: 'shadow-error',
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
