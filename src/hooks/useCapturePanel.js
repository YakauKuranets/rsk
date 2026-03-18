// src/hooks/useCapturePanel.js
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../utils/toast';

export function useCapturePanel() {
  const [captureUrl, setCaptureUrl] = useState('');
  const [captureDuration, setCaptureDuration] = useState(120);
  const [captureFilename, setCaptureFilename] = useState('');
  const [portScanHost, setPortScanHost] = useState('');
  const [portScanResult, setPortScanResult] = useState([]);

  const runPortScan = async () => {
    if (!portScanHost.trim()) return;
    try {
      const result = await invoke('scan_host_ports', { host: portScanHost.trim() });
      setPortScanResult(result);
    } catch (e) {
      toast(`Ошибка: ${e}`);
    }
  };

  const captureWithFfmpeg = async () => {
    if (!captureUrl.trim()) return;
    try {
      await invoke('capture_archive_segment', {
        url: captureUrl.trim(),
        duration: captureDuration,
        filename: captureFilename.trim() || undefined,
      });
      toast('Захват запущен');
    } catch (e) {
      toast(`Ошибка захвата: ${e}`);
    }
  };

  const captureHttp = async () => {
    if (!captureUrl.trim()) return;
    try {
      await invoke('download_http_archive', {
        url: captureUrl.trim(),
        filename: captureFilename.trim() || undefined,
      });
      toast('HTTP загрузка запущена');
    } catch (e) {
      toast(`Ошибка: ${e}`);
    }
  };

  return {
    captureUrl,
    setCaptureUrl,
    captureDuration,
    setCaptureDuration,
    captureFilename,
    setCaptureFilename,
    portScanHost,
    setPortScanHost,
    portScanResult,
    setPortScanResult,
    runPortScan,
    captureWithFfmpeg,
    captureHttp,
  };
}
