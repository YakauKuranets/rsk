import { useCallback, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export function useStreamManager() {
  const [activeStreams, setActiveStreams] = useState({});

  const startStream = useCallback(async (targetId, rtspUrl) => {
    const wsUrl = await invoke('start_stream', { targetId, rtspUrl });
    setActiveStreams((prev) => ({ ...prev, [targetId]: wsUrl }));
    return wsUrl;
  }, []);

  const stopStream = useCallback(async (targetId) => {
    await invoke('stop_stream', { targetId });
    setActiveStreams((prev) => {
      const next = { ...prev };
      delete next[targetId];
      return next;
    });
  }, []);

  const stopAll = useCallback(async () => {
    await invoke('stop_all_streams');
    setActiveStreams({});
  }, []);

  return { activeStreams, startStream, stopStream, stopAll };
}
