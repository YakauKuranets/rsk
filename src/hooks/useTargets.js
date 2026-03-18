import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export function useTargets() {
  const [targets, setTargets] = useState([]);

  const loadTargets = async () => {
    try {
      // get_all_targets returns string keys, NOT target objects
      const keys = await invoke('get_all_targets');
      const loaded = [];
      for (const key of keys) {
        try {
          // Must call read_target for EACH key to get the actual data
          const jsonStr = await invoke('read_target', { targetId: key });
          const obj = typeof jsonStr === "string" ? JSON.parse(jsonStr) : jsonStr;
          if (obj && typeof obj === "object") loaded.push(obj);
        } catch (e) {
          console.warn("Failed to read target", key, e);
        }
      }
      setTargets(loaded);
    } catch (err) {
      console.error('Failed to load targets:', err);
    }
  };

  const saveTarget = async (data) => {
    // FIX: correct param names for Rust fn save_target(target_id, payload)
    const targetId = data.id || `nvr_${Date.now()}`;
    const payload = JSON.stringify({ ...data, id: targetId });
    await invoke('save_target', { targetId, payload });
    await loadTargets();
  };

  const deleteTarget = async (id) => {
    // FIX: correct param name for Rust fn delete_target(target_id)
    await invoke('delete_target', { targetId: id });
    await loadTargets();
  };

  useEffect(() => {
    loadTargets();
  }, []);

  return { targets, loadTargets, saveTarget, deleteTarget };
}
