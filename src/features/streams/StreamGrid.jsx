import React, { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { MapContainer, TileLayer, Marker, Popup, useMap } from 'react-leaflet';
import L from 'leaflet';
import icon from 'leaflet/dist/images/marker-icon.png';
import iconShadow from 'leaflet/dist/images/marker-shadow.png';
import MultiStreamGrid from './MultiStreamGrid';
import { toast } from '../../utils/toast';

const DefaultIcon = L.icon({ iconUrl: icon, shadowUrl: iconShadow, iconSize: [25, 41], iconAnchor: [12, 41] });
L.Marker.prototype.options.icon = DefaultIcon;

function MapController({ center }) {
  const map = useMap();
  useEffect(() => { if (center) map.setView(center, 14); }, [center, map]);
  return null;
}

function buildRtspUrlFromPath(activePath, terminal, channelIndex, cleanHost) {
  if (!activePath.toLowerCase().startsWith('rtsp://')) {
    const safePath = activePath.replace(/channel=1|ch1|Channels\/1/g, (match) => match.replace('1', channelIndex));
    return `rtsp://${terminal.login}:${terminal.password}@${cleanHost}/${safePath.replace(/^\//, '')}`;
  }

  const encodedLogin = encodeURIComponent(terminal.login || 'admin');
  const encodedPass = encodeURIComponent(terminal.password || '');
  return activePath
    .replace(/channel=1\b/g, `channel=${channelIndex}`)
    .replace(/ch1\b/g, `ch${channelIndex}`)
    .replace(/Channels\/101\b/g, `Channels/${channelIndex}01`)
    .replace(/\/11(\b|$)/g, `/${channelIndex}1$1`)
    .replace('{login}', encodedLogin)
    .replace('{password}', encodedPass);
}

export default function StreamGrid({
  mapCenter,
  groupedMapTargets,
  fetchFtpRoot,
  setNemesisTarget,
  handleLocalArchive,
  handleFetchNvrDeviceInfo,
  handleFetchOnvifDeviceInfo,
  hubCookie,
}) {
  const [gridSize, setGridSize] = useState(4);
  const [slots, setSlots] = useState(Array(16).fill(null));
  const [pendingSlot, setPendingSlot] = useState(null);

  const nextSlot = useMemo(() => {
    const idx = slots.slice(0, gridSize).findIndex((s) => !s);
    return idx >= 0 ? idx : 0;
  }, [gridSize, slots]);

  const startInSlot = async (slotIndex, terminal, channel) => {
    try {
      const targetId = `slot_${slotIndex}_${terminal.id}_${channel.id}`;

      if (slots[slotIndex]?.targetId) {
        await invoke('stop_stream', { targetId: slots[slotIndex].targetId });
      }

      let wsUrl = '';
      let resolvedRtsp = '';

      if (terminal.type === 'hub') {
        wsUrl = await invoke('start_hub_stream', {
          targetId,
          userId: terminal.hub_id.toString(),
          channelId: channel.index.toString(),
          cookie: hubCookie,
        });
        resolvedRtsp = 'hub';
      } else {
        const cleanHost = terminal.host.replace(/^(http:\/\/|https:\/\/|rtsp:\/\/)/i, '').split('/')[0];
        const activePath = await invoke('probe_rtsp_path', {
          host: cleanHost,
          login: terminal.login,
          pass: terminal.password,
        });

        resolvedRtsp = buildRtspUrlFromPath(activePath, terminal, channel.index, cleanHost);
        wsUrl = await invoke('start_stream', { targetId, rtspUrl: resolvedRtsp });
      }

      setSlots((prev) => {
        const next = [...prev];
        next[slotIndex] = {
          targetId,
          wsUrl,
          rtspUrl: resolvedRtsp,
          terminal,
          channel,
          cameraName: `${terminal.name} :: ${channel.name}`,
          hubCookie,
        };
        return next;
      });
    } catch (err) {
      toast(`Ошибка запуска потока: ${err}`);
    }
  };

  const stopSlot = async (slotIndex) => {
    const slot = slots[slotIndex];
    if (!slot) return;

    try {
      await invoke('stop_stream', { targetId: slot.targetId });
    } catch (err) {
      console.error('stop stream error', err);
    }

    setSlots((prev) => {
      const next = [...prev];
      next[slotIndex] = null;
      return next;
    });
  };

  const stopAll = async () => {
    await invoke('stop_all_streams');
    setSlots(Array(16).fill(null));
  };

  return (
    <div style={{ flex: 1, position: 'relative' }}>
      <MapContainer center={mapCenter} zoom={13} style={{ height: '100%', width: '100%' }} zoomControl={false}>
        <MapController center={mapCenter} />
        <TileLayer url="https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png" />

        {Array.from(groupedMapTargets.values()).map(site => (
          <Marker key={site.id} position={[site.lat, site.lng]}>
            <Popup>
              <div style={{ color: '#000', minWidth: '150px' }}>
                <strong>{site.siteName}</strong><br/>
                <div style={{ marginTop: '6px', marginBottom: '6px', color: '#444', fontSize: '11px' }}>
                  Терминалов: {site.terminals.length}
                </div>
                <div style={{ marginTop: '8px', maxHeight: '300px', overflowY: 'auto' }}>
                  {site.terminals.map((t) => (
                    <div key={t.id} style={{ borderTop: '1px solid #ddd', paddingTop: '8px', marginTop: '8px' }}>
                      <div style={{ fontWeight: 700, fontSize: '12px' }}>{t.name}</div>
                      <div style={{ color: '#666', fontSize: '10px', marginBottom: '6px' }}>{t.host}</div>

                      {t.channels?.map(ch => (
                        <button key={ch.id} onClick={() => startInSlot(pendingSlot ?? nextSlot, t, ch)} style={{ display: 'block', width: '100%', marginBottom: '4px', padding: '6px', cursor: 'pointer', backgroundColor: '#111', color: '#00f0ff', border: '1px solid #00f0ff', fontSize: '11px' }}>
                          ▶ СЛОТ {((pendingSlot ?? nextSlot) + 1)}: {ch.name}
                        </button>
                      ))}

                      {t.type === 'hub' ? (
                          <button onClick={() => fetchFtpRoot('video1')} style={{ display: 'block', width: '100%', marginTop: '8px', padding: '6px', cursor: 'pointer', backgroundColor: '#1a4a4a', color: '#00f0ff', border: '1px solid #00f0ff', fontSize: '11px', fontWeight: 'bold' }}>
                            📁 АРХИВ ХАБА (FTP)
                          </button>
                      ) : (
                          <>
                            <button onClick={() => setNemesisTarget({ host: t.host, login: t.login || 'admin', password: t.password || '', name: t.name, channels: t.channels })} style={{ display: 'block', width: '100%', marginTop: '8px', padding: '6px', cursor: 'pointer', background: 'linear-gradient(90deg, #2a0808, #0a0808)', color: '#ff003c', border: '1px solid #ff003c', fontSize: '11px', fontWeight: 'bold', letterSpacing: '1px' }}>
                              ☢ NEMESIS ARCHIVE
                            </button>
                            <button onClick={() => handleLocalArchive(t)} style={{ display: 'block', width: '100%', marginTop: '6px', padding: '6px', cursor: 'pointer', backgroundColor: '#4a1a1a', color: '#ff9900', border: '1px solid #ff9900', fontSize: '11px', fontWeight: 'bold' }}>
                              ⏳ ЗАПРОС ПАМЯТИ
                            </button>
                            <button onClick={() => handleFetchNvrDeviceInfo(t)} style={{ display: 'block', width: '100%', marginTop: '6px', padding: '6px', cursor: 'pointer', backgroundColor: '#1a1a4a', color: '#9fc2ff', border: '1px solid #6a88ff', fontSize: '11px', fontWeight: 'bold' }}>
                              ℹ ISAPI DEVICE INFO
                            </button>
                            <button onClick={() => handleFetchOnvifDeviceInfo(t)} style={{ display: 'block', width: '100%', marginTop: '6px', padding: '6px', cursor: 'pointer', backgroundColor: '#1a3a1a', color: '#a8ffb0', border: '1px solid #47c45a', fontSize: '11px', fontWeight: 'bold' }}>
                              ℹ ONVIF DEVICE INFO
                            </button>
                          </>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            </Popup>
          </Marker>
        ))}
      </MapContainer>

      <div style={{ position: 'absolute', right: 16, top: 16, zIndex: 1200, display: 'flex', gap: 8 }}>
        <button onClick={() => setPendingSlot(nextSlot)} style={{ padding: '6px 10px', border: '1px solid #444', background: '#111', color: '#00f0ff', fontFamily: 'monospace', cursor: 'pointer' }}>
          Активный слот: {pendingSlot !== null ? pendingSlot + 1 : `auto (${nextSlot + 1})`}
        </button>
        <button onClick={stopAll} style={{ padding: '6px 10px', border: '1px solid #663333', background: '#220b0b', color: '#ff9a9a', fontFamily: 'monospace', cursor: 'pointer' }}>
          STOP ALL
        </button>
      </div>

      <MultiStreamGrid
        gridSize={gridSize}
        setGridSize={setGridSize}
        slots={slots}
        onStopSlot={stopSlot}
        onPickSlot={(idx) => setPendingSlot(idx)}
      />
    </div>
  );
}
