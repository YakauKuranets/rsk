import React from 'react';
import StreamPlayer from '../../StreamPlayer';

const GRID_LAYOUTS = {
  1: { cols: 1, rows: 1, label: '1×1' },
  4: { cols: 2, rows: 2, label: '2×2' },
  9: { cols: 3, rows: 3, label: '3×3' },
  16: { cols: 4, rows: 4, label: '4×4' },
};

export default function MultiStreamGrid({
  gridSize,
  setGridSize,
  slots,
  onStopSlot,
  onPickSlot,
}) {
  const layout = GRID_LAYOUTS[gridSize] || GRID_LAYOUTS[4];

  return (
    <div style={{ position: 'absolute', inset: 12, pointerEvents: 'none' }}>
      <div style={{ display: 'flex', gap: 8, marginBottom: 8, pointerEvents: 'auto' }}>
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
          height: 'calc(100% - 36px)',
          display: 'grid',
          gridTemplateColumns: `repeat(${layout.cols}, 1fr)`,
          gridTemplateRows: `repeat(${layout.rows}, 1fr)`,
          gap: 3,
          backgroundColor: 'rgba(0,0,0,0.55)',
          border: '1px solid #202020',
          pointerEvents: 'auto',
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
                  onClose={() => onStopSlot(i)}
                />
                <div style={{ position: 'absolute', top: 4, left: 4, background: 'rgba(0,0,0,0.7)', padding: '2px 6px', fontSize: 10, color: '#00f0ff', borderRadius: 2 }}>
                  {slot.cameraName}
                </div>
              </div>
            );
          }

          return (
            <div
              key={i}
              onClick={() => onPickSlot(i)}
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
              }}
            >
              <div style={{ fontSize: 24, marginBottom: 4 }}>+</div>
              <div>Слот {i + 1}</div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
