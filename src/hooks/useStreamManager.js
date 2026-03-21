import { useCallback, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { probeStreamCapability } from '../api/capabilities';

export function useStreamManager() {
  const [activeStreams, setActiveStreams] = useState({});
  const [shadowProbeStatus, setShadowProbeStatus] = useState({});

  const startStream = useCallback(async (targetId, rtspUrl) => {
    const wsUrl = await invoke('start_stream', { targetId, rtspUrl });
    setActiveStreams((prev) => ({ ...prev, [targetId]: wsUrl }));

    // Shadow-mode capability check: read-only diagnostic path.
    // Does not affect legacy UX/behavior and is safe to ignore by consumers.
    void probeStreamCapability(targetId, 'discovery_mode')
      .then((cap) => {
        setShadowProbeStatus((prev) => ({
          ...prev,
          [targetId]: {
            legacyAssumedAlive: true,
            capabilityOk: cap.ok,
            capabilityAlive: Boolean(cap.alive),
            source: cap.source || 'capability',
            checkedAt: new Date().toISOString(),
          },
        }));
      })
      .catch(() => {
        setShadowProbeStatus((prev) => ({
          ...prev,
          [targetId]: {
            legacyAssumedAlive: true,
            capabilityOk: false,
            capabilityAlive: false,
            source: 'shadow-error',
            checkedAt: new Date().toISOString(),
          },
        }));
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
