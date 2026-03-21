import React, { useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import StreamPlayer from '../../StreamPlayer';
import { toast } from '../../utils/toast';
import { canRunStreamVerification, deriveCardKind, isCardKindGatingEnabled } from '../targets/cardKindAdapter';

const GRID_LAYOUTS = {
  1: { cols: 1, rows: 1, label: '1×1' },
  4: { cols: 2, rows: 2, label: '2×2' },
  9: { cols: 3, rows: 3, label: '3×3' },
};

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

export default function MultiStreamGrid({ terminalId, targets, hubCookie, onClose, onArchiveContext }) {
  const [gridSize, setGridSize] = useState(4);
  const [slots, setSlots] = useState(Array(9).fill(null));
  const [pickerSlot, setPickerSlot] = useState(null);

  const layout = GRID_LAYOUTS[gridSize] || GRID_LAYOUTS[4];

  const terminalCameras = useMemo(() => {
    const terminal = (targets || []).find((t) => String(t.id) === String(terminalId));
    if (!terminal) return [];
    return (terminal.channels || []).map((channel) => ({
      id: `${terminal.id}_${channel.id}`,
      label: `${terminal.name} :: ${channel.name}`,
      terminal,
      channel,
    }));
  }, [targets, terminalId]);

  const stopSlot = async (slotIndex) => {
    const slot = slots[slotIndex];
    if (!slot?.targetId) return;
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

  const startInSlot = async (slotIndex, terminal, channel) => {
    try {
      const targetId = `ms_${slotIndex}_${terminal.id}_${channel.id}`;
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
        if (!canRunStreamVerification(terminal)) {
          toast(`Stream verification action is gated for kind=${deriveCardKind(terminal)}`);
          return;
        }
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

      setPickerSlot(null);
    } catch (err) {
      toast(`Ошибка запуска потока: ${err}`);
    }
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '10px 12px', borderBottom: '1px solid #222' }}>
        <strong style={{ fontSize: 12, color: '#00f0ff' }}>MULTI-STREAM · terminal={terminalId}</strong>
        <button onClick={onClose} style={{ background: '#300', color: '#ffd1d1', border: '1px solid #733', padding: '4px 10px', cursor: 'pointer' }}>
          Закрыть панель (X)
        </button>
      </div>

      <div style={{ display: 'flex', gap: 8, padding: '10px 12px' }}>
        {Object.entries(GRID_LAYOUTS).map(([size, cfg]) => (
          <button
            key={size}
            onClick={() => setGridSize(Number(size))}
            style={{
              padding: '4px 12px',
              background: gridSize === Number(size) ? '#00f0ff' : '#222',
              color: gridSize === Number(size) ? '#000' : '#888',
              border: 'none',
              borderRadius: 4,
              cursor: 'pointer',
              fontFamily: 'monospace',
              fontSize: 12,
              fontWeight: 700,
            }}
          >
            {cfg.label}
          </button>
        ))}
      </div>

      <div
        style={{
          flex: 1,
          margin: '0 12px 12px',
          display: 'grid',
          gridTemplateColumns: `repeat(${layout.cols}, 1fr)`,
          gridTemplateRows: `repeat(${layout.rows}, 1fr)`,
          gap: 3,
          backgroundColor: 'rgba(0,0,0,0.55)',
          border: '1px solid #202020',
          minHeight: 0,
        }}
      >
        {Array.from({ length: gridSize }, (_, i) => {
          const slot = slots[i];
          if (slot) {
            return (
              <div key={i} style={{ position: 'relative', overflow: 'hidden', backgroundColor: '#0a0a0c' }}>
                <StreamPlayer
                  streamUrl={slot.wsUrl}
                  cameraName={slot.cameraName}
                  terminal={slot.terminal}
                  channel={slot.channel}
                  hubCookie={slot.hubCookie}
                  onArchiveContext={onArchiveContext}
                  onClose={() => stopSlot(i)}
                />
              </div>
            );
          }

          return (
            <div
              key={i}
              onClick={() => setPickerSlot(i)}
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                backgroundColor: '#0a0a0c',
                border: '1px dashed #222',
                color: '#333',
                fontSize: 11,
                fontFamily: 'monospace',
                cursor: 'pointer',
                position: 'relative',
              }}
            >
              <div style={{ fontSize: 24, marginBottom: 4 }}>+</div>
              <div>Слот {i + 1}</div>
              {pickerSlot === i && (
                <div style={{ position: 'absolute', inset: 10, background: '#111', border: '1px solid #333', padding: 8 }}>
                  <div style={{ fontSize: 11, color: '#ccc', marginBottom: 6 }}>Выберите камеру терминала</div>
                  <select
                    style={{ width: '100%', background: '#000', color: '#00f0ff', border: '1px solid #00f0ff' }}
                    defaultValue=""
                    onChange={(e) => {
                      const selected = terminalCameras.find((cam) => cam.id === e.target.value);
                      if (selected) {
                        startInSlot(i, selected.terminal, selected.channel);
                      }
                    }}
                  >
                    <option value="" disabled>+ (Слот {i + 1})</option>
                    {terminalCameras.map((cam) => {
                      const disabled = cam.terminal?.type !== 'hub' && !canRunStreamVerification(cam.terminal);
                      const kindLabel = isCardKindGatingEnabled() ? ` [${deriveCardKind(cam.terminal)}]` : '';
                      return (
                        <option key={cam.id} value={cam.id} disabled={disabled}>
                          {cam.label}{kindLabel}{disabled ? ' (gated)' : ''}
                        </option>
                      );
                    })}
                  </select>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
