// src/hooks/useNvrPanel.js
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export function useNvrPanel() {
  const [nvrProbeResults, setNvrProbeResults] = useState([]);
  const [nvrDeviceInfo, setNvrDeviceInfo] = useState(null);
  const [isapiSearchResults, setIsapiSearchResults] = useState([]);
  const [isapiFrom, setIsapiFrom] = useState('2026-01-01T00:00:00Z');
  const [isapiTo, setIsapiTo] = useState('2026-12-31T23:59:59Z');
  const [isapiSearchAuth, setIsapiSearchAuth] = useState({ login: 'admin', pass: '' });
  const [onvifDeviceInfo, setOnvifDeviceInfo] = useState(null);
  const [onvifRecordingTokens, setOnvifRecordingTokens] = useState([]);
  const [onvifSearchAuth, setOnvifSearchAuth] = useState({ login: 'admin', pass: '' });
  const [archiveProbeResults, setArchiveProbeResults] = useState([]);

  const fetchNvrDeviceInfo = async (terminal) => {
    try {
      const info = await invoke('fetch_nvr_device_info', { terminal });
      setNvrDeviceInfo(info);
    } catch (e) {
      console.error(e);
    }
  };

  const fetchIsapiDeviceInfo = async (terminal) => {
    try {
      const info = await invoke('fetch_onvif_device_info', { terminal });
      setOnvifDeviceInfo(info);
    } catch (e) {
      console.error(e);
    }
  };

  const searchIsapiRecordings = async (terminal) => {
    try {
      const results = await invoke('search_isapi_recordings', {
        terminal,
        from: isapiFrom,
        to: isapiTo,
        login: isapiSearchAuth.login,
        pass: isapiSearchAuth.pass,
      });
      setIsapiSearchResults(results);
    } catch (e) {
      console.error(e);
    }
  };

  const searchOnvifRecordings = async (terminal) => {
    try {
      const tokens = await invoke('search_onvif_recordings', { terminal });
      setOnvifRecordingTokens(tokens);
    } catch (e) {
      console.error(e);
    }
  };

  const probeArchiveEndpoints = async (terminal) => {
    try {
      const results = await invoke('probe_archive_export_endpoints', { terminal });
      setArchiveProbeResults(results);
    } catch (e) {
      console.error(e);
    }
  };

  return {
    nvrProbeResults,
    setNvrProbeResults,
    nvrDeviceInfo,
    setNvrDeviceInfo,
    isapiSearchResults,
    setIsapiSearchResults,
    isapiFrom,
    setIsapiFrom,
    isapiTo,
    setIsapiTo,
    isapiSearchAuth,
    setIsapiSearchAuth,
    onvifDeviceInfo,
    setOnvifDeviceInfo,
    onvifRecordingTokens,
    setOnvifRecordingTokens,
    onvifSearchAuth,
    setOnvifSearchAuth,
    archiveProbeResults,
    setArchiveProbeResults,
    fetchNvrDeviceInfo,
    fetchIsapiDeviceInfo,
    searchIsapiRecordings,
    searchOnvifRecordings,
    probeArchiveEndpoints,
  };
}
