import { useMemo, useState } from 'react';
import {
  canRunArchiveExport,
  canRunDiscoveryActions,
  canRunPromotionActions,
  canRunStreamVerification,
  canRunVerifiedActions,
  deriveCardKind,
  isCardKindGatingEnabled,
} from '../targets/cardKindAdapter';

const T = {
  bg0: '#07070f',
  bg1: '#0c0c1a',
  line: '#1e1e35',
  dim: '#444466',
  muted: '#6666aa',
  text: '#c0c0e0',
  red: '#ff3355',
  cyan: '#00ccff',
  grn: '#00dd88',
  amb: '#ffaa00',
  purp: '#9966ff',
  blue: '#4488ff',
};

const btn = (color) => ({
  width: '100%',
  padding: '5px',
  background: `${color}18`,
  color,
  border: `1px solid ${color}40`,
  borderRadius: '3px',
  fontSize: '10px',
  fontWeight: 600,
  cursor: 'pointer',
  marginBottom: '3px',
  textAlign: 'left',
  fontFamily: 'inherit',
});

function normalizeChannels(target) {
  if (Array.isArray(target?.channels) && target.channels.length > 0) return target.channels;
  const count = Number(target?.channelCount || target?.cameraCount || 0);
  return Array.from({ length: count }, (_, i) => ({ id: i + 1, name: `Channel ${i + 1}` }));
}

export default function TargetCard({
  target: t,
  selected = false,
  compatibilityProfile = null,
  onSelect,
  onNemesis,
  onMemoryRequest,
  onIsapiInfo,
  onIsapiSearch,
  onOnvifInfo,
  onOnvifRecordings,
  onArchiveEndpoints,
  onOpenHubArchive,
  onDelete,
}) {
  const [open, setOpen] = useState(false);

  const channels = useMemo(() => normalizeChannels(t || {}), [t]);
  const cameraCount = channels.length || Number(t?.cameraCount || t?.channelCount || 0);
  const coords = t?.lat != null && t?.lng != null ? `${Number(t.lat).toFixed(4)}, ${Number(t.lng).toFixed(4)}` : 'n/a';
  const type = String(t?.type || (cameraCount > 0 ? 'camera-hub' : 'host')).toLowerCase();
  const icon = type.includes('cam') || type.includes('nvr') || type.includes('dvr') ? '📹' : '🖥️';
  const danger = t?.riskScore >= 80 || t?.severity === 'critical';
  const isHub = type === 'hub';
  const canDiscovery = canRunDiscoveryActions(t);
  const canVerified = canRunVerifiedActions(t);
  const canStreamCheck = canRunStreamVerification(t);
  const canArchive = canRunArchiveExport(t);
  const canPromotion = canRunPromotionActions(t);

  return (
    <div style={{
      background: `linear-gradient(180deg, ${T.bg1}, ${T.bg0})`,
      border: `1px solid ${danger ? T.red : T.line}`,
      borderRadius: '6px',
      padding: '8px',
      color: T.text,
      boxShadow: selected
        ? `0 0 0 1px ${T.cyan}55 inset`
        : open
          ? `0 0 0 1px ${T.cyan}22 inset`
          : 'none',
    }}>
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '24px 1fr auto',
          gap: '8px',
          alignItems: 'center',
          width: '100%',
          marginBottom: '6px',
        }}
      >
        <div style={{ fontSize: '18px' }}>{icon}</div>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: '12px', fontWeight: 700, color: '#e5e7ff', overflow: 'hidden', textOverflow: 'ellipsis' }}>
            {t?.name || t?.host || 'Неизвестная цель'}
          </div>
          <div style={{ fontSize: '10px', color: T.muted }}>{t?.host || t?.ip || 'без IP'}</div>
          {compatibilityProfile?.label && (
            <div style={{ marginTop: '3px', display: 'inline-flex', alignItems: 'center', gap: '4px', fontSize: '9px', color: '#9db7ca' }}>
              <span style={{ padding: '1px 5px', borderRadius: '999px', border: '1px solid #34506a', background: '#0b1a2b' }}>
                {compatibilityProfile.label}
              </span>
              {selected && <span style={{ color: '#88a8bf' }}>· {compatibilityProfile.note || 'без сигнала'}</span>}
            </div>
          )}
        </div>
        <div style={{ textAlign: 'right' }}>
          <div style={{ fontSize: '10px', color: T.cyan, fontWeight: 700 }}>{cameraCount}</div>
          <div style={{ fontSize: '9px', color: T.dim }}>камер</div>
        </div>
      </div>
      <div style={{display:'flex',gap:'4px'}}>
        <button
          type="button"
          style={{
            ...btn(selected ? T.grn : T.cyan),
            flex: 1,
            marginBottom: 0,
            textAlign: 'center',
            padding: '4px',
          }}
          onClick={() => onSelect?.(t)}
        >
          {selected ? '✓ Выбрана' : 'Выбрать'}
        </button>
        <button
          type="button"
          style={{
            ...btn(T.dim),
            width: '72px',
            marginBottom: 0,
            textAlign: 'center',
            padding: '4px',
          }}
          onClick={() => setOpen((v) => !v)}
        >
          {open ? 'Свернуть' : 'Детали'}
        </button>
      </div>

      {open && (
        <div style={{ marginTop: '8px', borderTop: `1px solid ${T.line}`, paddingTop: '8px' }}>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: '6px', marginBottom: '8px', fontSize: '10px' }}>
            <div style={{ background: '#090916', padding: '6px', border: `1px solid ${T.line}` }}>
              <div style={{ color: T.dim }}>Координаты</div>
              <div style={{ color: T.text }}>{coords}</div>
            </div>
            <div style={{ background: '#090916', padding: '6px', border: `1px solid ${T.line}` }}>
              <div style={{ color: T.dim }}>Каналы</div>
              <div style={{ color: T.text }}>{cameraCount || 'n/a'}</div>
            </div>
          </div>
          {compatibilityProfile?.label && (
            <div style={{ marginBottom: '8px', padding: '6px', border: `1px solid ${T.line}`, background: '#091321', borderRadius: '4px' }}>
              <div style={{ fontSize: '10px', color: T.muted, marginBottom: '3px' }}>Профиль совместимости</div>
              <div style={{ fontSize: '10px', color: T.text, marginBottom: '2px' }}>
                <b>{compatibilityProfile.label}</b> · поток: <span style={{ color: '#9ec58f' }}>{compatibilityProfile.stream}</span>
              </div>
              <div style={{ fontSize: '10px', color: T.dim }}>
                web-info: <span style={{ color: '#9ec58f' }}>{compatibilityProfile.web}</span> · архив: <span style={{ color: '#9ec58f' }}>{compatibilityProfile.archive}</span>
              </div>
            </div>
          )}

          <div style={{ marginBottom: '8px' }}>
            <div style={{ fontSize: '10px', color: T.dim, marginBottom: '4px' }}>Доступные каналы</div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px' }}>
              {channels.length > 0 ? channels.map((channel, idx) => (
                <span key={channel.id || idx} style={{
                  padding: '2px 6px',
                  borderRadius: '999px',
                  border: `1px solid ${T.blue}44`,
                  background: `${T.blue}12`,
                  color: T.blue,
                  fontSize: '9px',
                }}>
                  {channel.name || `Ch ${channel.id || idx + 1}`}
                </span>
              )) : <span style={{ fontSize: '10px', color: T.dim }}>Каналы не заданы</span>}
            </div>
          </div>

          {isHub ? (
            <button type="button" style={{ ...btn(T.cyan), marginBottom: 0 }} onClick={() => onOpenHubArchive?.(t)}>📁 Архив HUB</button>
          ) : (
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: '4px' }}>
              {canVerified && <button type="button" style={btn(T.red)} onClick={() => onNemesis?.(t)}>☠ Nemesis</button>}
              {canDiscovery && <button type="button" style={btn(T.purp)} onClick={() => onMemoryRequest?.(t)}>🧠 Память</button>}
              {canStreamCheck && <button type="button" style={btn(T.cyan)} onClick={() => onIsapiInfo?.(t)}>ℹ ISAPI инфо</button>}
              {canVerified && <button type="button" style={btn(T.amb)} onClick={() => onIsapiSearch?.(t)}>🔎 ISAPI поиск</button>}
              {canStreamCheck && <button type="button" style={btn(T.grn)} onClick={() => onOnvifInfo?.(t)}>📡 ONVIF инфо</button>}
              {canVerified && <button type="button" style={btn(T.blue)} onClick={() => onOnvifRecordings?.(t)}>🎞 Записи</button>}
              {canArchive && <button type="button" style={btn(T.purp)} onClick={() => onArchiveEndpoints?.(t)}>🗄 Архив</button>}
              {isCardKindGatingEnabled() && canPromotion && (
                <span style={{ ...btn(T.dim), display: 'block' }}>↗ Доступно повышение ({deriveCardKind(t)})</span>
              )}
            </div>
          )}

          <button
            type="button"
            style={{ ...btn(T.red), marginTop: '6px', marginBottom: 0, textAlign: 'center' }}
            onClick={() => onDelete?.(t)}
          >
            Удалить цель
          </button>
        </div>
      )}
    </div>
  );
}
