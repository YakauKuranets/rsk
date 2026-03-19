// src/hooks/useHubRecon.js
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast';

export function useHubRecon() {
  const [reconUserId, setReconUserId] = useState('');
  const [reconChannelId, setReconChannelId] = useState('0');
  const [reconDate, setReconDate] = useState('2026-02-19');
  const [reconResults, setReconResults] = useState([]);
  const [reconRunning, setReconRunning] = useState(false);
  const [addressQuery, setAddressQuery] = useState('');
  const [hubSearch, setHubSearch] = useState('');
  const [hubResults, setHubResults] = useState([]);

  const runHubRecon = async () => {
    setReconRunning(true);
    try {
      const results = await invoke('recon_hub_archive_routes', {
        userId: reconUserId,
        channelId: parseInt(reconChannelId, 10),
        date: reconDate,
      });
      setReconResults(results || []);
    } catch (e) {
      toast(`Ошибка: ${e}`);
    }
    setReconRunning(false);
  };

  const searchGlobalHub = async (onAddTerminal) => {
    if (!addressQuery.trim()) return;
    try {
      const [lat, lng] = await invoke('geocode_address', { address: addressQuery });
      const results = await invoke('search_global_hub', { lat, lng, query: addressQuery });
      onAddTerminal && results.forEach((r) => onAddTerminal(r));
    } catch (e) {
      toast(`Ошибка: ${e}`);
    }
  };

  return {
    reconUserId,
    setReconUserId,
    reconChannelId,
    setReconChannelId,
    reconDate,
    setReconDate,
    reconResults,
    setReconResults,
    reconRunning,
    setReconRunning,
    addressQuery,
    setAddressQuery,
    hubSearch,
    setHubSearch,
    hubResults,
    setHubResults,
    runHubRecon,
    searchGlobalHub,
  };
}
