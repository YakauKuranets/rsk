import React from 'react';

export default function PlaybookEditor({ value, onChange }) {
  return (
    <textarea
      value={value}
      onChange={(e) => onChange?.(e.target.value)}
      rows={14}
      style={{ width: '100%', fontFamily: 'monospace', background: '#090909', color: '#d0faff' }}
    />
  );
}
