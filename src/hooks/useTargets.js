import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

function normalizeTargetRecords(raw) {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((item) => {
      if (!item) return null;
      if (typeof item === 'string') {
        try {
          return JSON.parse(item);
        } catch {
          return null;
        }
      }
      return item;
    })
    .filter(Boolean);
}

export function useTargets() {
  const [targets, setTargets] = useState([]);

  const loadTargets = async () => {
    try {
      const raw = await invoke('get_all_targets');
      setTargets(normalizeTargetRecords(raw));
    } catch (err) {
      console.error('Failed to load targets:', err);
    }
  };

  const saveTarget = async (data) => {
    await invoke('save_target', { data: JSON.stringify(data) });
    await loadTargets();
  };

  const deleteTarget = async (id) => {
    await invoke('delete_target', { id });
    await loadTargets();
  };

  useEffect(() => {
    loadTargets();
  }, []);

  return { targets, loadTargets, saveTarget, deleteTarget };
}
