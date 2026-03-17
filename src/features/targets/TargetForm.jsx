import React from 'react';

export default function TargetForm({ form, setForm, onSave, onGeocode }) {
  return (
    <div style={{ border: '1px solid #2a2a2e', padding: 12, background: '#0d0d11' }}>
      <h3 style={{ marginTop: 0, color: '#00f0ff' }}>Target Form</h3>
      <div style={{ display: 'grid', gap: 6 }}>
        <input value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="Название" />
        <input value={form.host} onChange={(e) => setForm({ ...form, host: e.target.value })} placeholder="Host/IP" />
        <input value={form.login} onChange={(e) => setForm({ ...form, login: e.target.value })} placeholder="Login" />
        <input value={form.password} onChange={(e) => setForm({ ...form, password: e.target.value })} placeholder="Password" />
        <div style={{ display: 'flex', gap: 8 }}>
          <button onClick={onSave}>Сохранить</button>
          <button onClick={onGeocode}>Геокодировать</button>
        </div>
      </div>
    </div>
  );
}
