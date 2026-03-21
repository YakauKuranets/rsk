import { useCallback, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { probeStreamCapability } from '../api/capabilities';

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
