import { useMemo, useState } from 'react';

const COLORS = {
  violet: '#9b6bff',
  cyan: '#00cfff',
  red: '#ff4d6d',
  amber: '#ffb020',
  green: '#19d38a',
  blue: '#4488ff',
  pink: '#ff66cc',
  gray: '#8b90b3',
};

const ICONS = ['🏷️', '🎯', '⚠️', '🔐', '📍', '📡', '🛰️', '🎥', '🧠', '🧪', '🛡️', '🔥'];
const PRIORITIES = [1, 2, 3, 4, 5];
const COLOR_KEYS = Object.keys(COLORS);

const baseInput = {
  width: '100%',
  padding: '6px 8px',
  borderRadius: '4px',
  background: '#0a0d18',
  color: '#d9def8',
  border: '1px solid #2a314f',
  fontSize: '12px',
  boxSizing: 'border-box',
};

function pillStyle(color) {
  return {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '6px',
    padding: '4px 8px',
    borderRadius: '999px',
    background: `${color}18`,
    border: `1px solid ${color}40`,
    color,
    fontSize: '10px',
    fontWeight: 700,
  };
}

export default function LabelPanel({ value = [], onChange, title = 'Label system' }) {
  const [draft, setDraft] = useState({ text: '', icon: ICONS[0], color: 'violet', priority: 3 });

  const labels = useMemo(() => Array.isArray(value) ? value : [], [value]);

  const emit = (next) => onChange?.(next);
  const addLabel = () => {
    if (!draft.text.trim()) return;
    emit([
      ...labels,
      {
        id: `${Date.now()}-${draft.text.trim()}`,
        text: draft.text.trim(),
        icon: draft.icon,
        color: draft.color,
        priority: draft.priority,
      },
    ]);
    setDraft((prev) => ({ ...prev, text: '' }));
  };

  const removeLabel = (id) => emit(labels.filter((label) => label.id !== id));

  return (
    <section style={{ border: '1px solid #212744', borderRadius: '8px', padding: '12px', background: '#070b14', color: '#d9def8' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '10px' }}>
        <h3 style={{ margin: 0, fontSize: '13px', letterSpacing: '0.08em', textTransform: 'uppercase', color: '#8fb7ff' }}>{title}</h3>
        <span style={{ fontSize: '10px', color: '#68729d' }}>{labels.length} labels</span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr 1fr auto', gap: '8px', marginBottom: '10px' }}>
        <input style={baseInput} value={draft.text} placeholder="Label name" onChange={(e) => setDraft((prev) => ({ ...prev, text: e.target.value }))} />
        <select style={baseInput} value={draft.icon} onChange={(e) => setDraft((prev) => ({ ...prev, icon: e.target.value }))}>
          {ICONS.map((icon) => <option key={icon} value={icon}>{icon}</option>)}
        </select>
        <select style={baseInput} value={draft.color} onChange={(e) => setDraft((prev) => ({ ...prev, color: e.target.value }))}>
          {COLOR_KEYS.map((key) => <option key={key} value={key}>{key}</option>)}
        </select>
        <select style={baseInput} value={draft.priority} onChange={(e) => setDraft((prev) => ({ ...prev, priority: Number(e.target.value) }))}>
          {PRIORITIES.map((priority) => <option key={priority} value={priority}>P{priority}</option>)}
        </select>
        <button type="button" onClick={addLabel} style={{ ...baseInput, width: 'auto', cursor: 'pointer', background: '#10203c', color: '#7fd2ff', fontWeight: 700 }}>
          Add
        </button>
      </div>

      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', marginBottom: '12px' }}>
        {labels.length > 0 ? labels.map((label) => {
          const color = COLORS[label.color] || COLORS.violet;
          return (
            <div key={label.id} style={pillStyle(color)}>
              <span>{label.icon}</span>
              <span>{label.text}</span>
              <span style={{ opacity: 0.75 }}>P{label.priority}</span>
              <button type="button" onClick={() => removeLabel(label.id)} style={{ all: 'unset', cursor: 'pointer', fontWeight: 800 }}>
                ×
              </button>
            </div>
          );
        }) : <div style={{ fontSize: '11px', color: '#67708e' }}>Create labels with 12 icons, 8 colors, and 5 priorities.</div>}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, minmax(0, 1fr))', gap: '8px' }}>
        {COLOR_KEYS.map((key) => (
          <div key={key} style={{ border: `1px solid ${COLORS[key]}40`, background: `${COLORS[key]}12`, borderRadius: '6px', padding: '8px' }}>
            <div style={{ fontSize: '11px', color: COLORS[key], fontWeight: 700, marginBottom: '4px' }}>{key}</div>
            <div style={{ fontSize: '10px', color: '#a9b0d0' }}>Preview style for operational labels.</div>
          </div>
        ))}
      </div>
    </section>
  );
}
