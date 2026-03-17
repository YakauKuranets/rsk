import React, { useEffect, useState } from 'react';

export default function ToastHost() {
  const [items, setItems] = useState([]);

  useEffect(() => {
    const onToast = (e) => {
      const id = `${Date.now()}_${Math.random().toString(16).slice(2)}`;
      const next = { id, message: e.detail?.message || '...' };
      setItems((prev) => [next, ...prev].slice(0, 5));
      setTimeout(() => {
        setItems((prev) => prev.filter((x) => x.id !== id));
      }, 4200);
    };
    window.addEventListener('hyperion:toast', onToast);
    return () => window.removeEventListener('hyperion:toast', onToast);
  }, []);

  return (
    <div style={{ position: 'fixed', right: 16, bottom: 16, zIndex: 10000, display: 'flex', flexDirection: 'column', gap: 8 }}>
      {items.map((item) => (
        <div key={item.id} style={{ maxWidth: 460, whiteSpace: 'pre-wrap', background: 'rgba(15,15,18,0.92)', color: '#d7f8ff', border: '1px solid #2d6b77', borderRadius: 6, padding: '8px 10px', fontFamily: 'monospace', fontSize: 12 }}>
          {item.message}
        </div>
      ))}
    </div>
  );
}
