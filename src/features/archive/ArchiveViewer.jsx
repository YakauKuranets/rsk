import React from 'react';
import { useAppStore } from '../../store/appStore';

export default function ArchiveViewer({ fetchFtpRoot, goBackFtp, handleDownloadFtp }) {
  const ftpBrowserOpen = useAppStore((s) => s.ftpBrowserOpen);
  const activeServerAlias = useAppStore((s) => s.activeServerAlias);
  const setFtpBrowserOpen = useAppStore((s) => s.setFtpBrowserOpen);
  const ftpPath = useAppStore((s) => s.ftpPath);
  const ftpItems = useAppStore((s) => s.ftpItems);

  if (!ftpBrowserOpen) return null;

  return (
    <div style={{ position: 'fixed', top: '5%', left: '5%', width: '90%', height: '90%', backgroundColor: '#05050a', border: '2px solid #00f0ff', zIndex: 10000, padding: '20px', display: 'flex', flexDirection: 'column', boxSizing: 'border-box' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '15px' }}>
        <h2 style={{ color: '#00f0ff', margin: 0 }}>📁 СЕРВЕР АРХИВОВ NVR ({activeServerAlias.toUpperCase()})</h2>
        <button onClick={() => setFtpBrowserOpen(false)} style={{ background: 'none', border: '1px solid #ff003c', color: '#ff003c', cursor: 'pointer', fontWeight: 'bold', padding: '5px 15px' }}>ЗАКРЫТЬ [X]</button>
      </div>

      <div style={{ display: 'flex', gap: '10px', marginBottom: '20px' }}>
        <button onClick={() => fetchFtpRoot('video1')} style={{ background: activeServerAlias === 'video1' ? '#1a4a4a' : '#111', color: '#00f0ff', border: '1px solid #00f0ff', padding: '5px 15px', cursor: 'pointer' }}>SERVER 1 (video1)</button>
        <button onClick={() => fetchFtpRoot('video2')} style={{ background: activeServerAlias === 'video2' ? '#4a1a4a' : '#111', color: '#ff00ff', border: '1px solid #ff00ff', padding: '5px 15px', cursor: 'pointer' }}>SERVER 2 (video2)</button>
        <div style={{ flex: 1, background: '#000', color: '#fff', border: '1px solid #555', padding: '8px', fontSize: '14px' }}><span style={{ color: '#888' }}>ПУТЬ:</span> {ftpPath}</div>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', border: '1px solid #333', background: '#000', padding: '10px' }}>
        {ftpPath !== '/' && <div onClick={goBackFtp} style={{ padding: '10px', borderBottom: '1px dashed #444', cursor: 'pointer', color: '#ffcc00', fontWeight: 'bold' }}>⬅ НАЗАД</div>}
        {ftpItems.map((item, index) => (
          <div key={index} onClick={() => { if (!item.isFile) fetchFtpRoot(activeServerAlias, item.path); }} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '10px', borderBottom: '1px solid #111', cursor: item.isFile ? 'default' : 'pointer', background: item.isFile ? 'transparent' : '#0a1515' }}>
            <span style={{ color: item.isFile ? '#00f0ff' : '#7dff9c', fontSize: '14px', fontWeight: item.isFile ? 'normal' : 'bold' }}>{item.isFile ? '📄' : '📁'} {item.name}</span>
            {item.isFile && <button onClick={(e) => { e.stopPropagation(); handleDownloadFtp(activeServerAlias, ftpPath, item.name); }} style={{ background: '#1a4a4a', color: '#00f0ff', border: '1px solid #00f0ff', cursor: 'pointer', padding: '5px 15px', fontWeight: 'bold' }}>СКАЧАТЬ ФАЙЛ</button>}
          </div>
        ))}
      </div>
    </div>
  );
}
