import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function ToolExecutorPanel() {
  const [tool, setTool] = useState('nmap');
  const [args, setArgs] = useState('-sV 127.0.0.1');
  const [result, setResult] = useState('');

  const executeTool = async () => {
    try {
      const res = await invoke('execute_tool', { tool, args });
      setResult(typeof res === 'string' ? res : JSON.stringify(res, null, 2));
    } catch (error) {
      setResult(`Tool executor unavailable: ${error}`);
    }
  };

  return (
    <section style={{ border: '1px solid #27472b', borderRadius: '8px', padding: '12px', background: '#09150b', color: '#dfffe4' }}>
      <h3 style={{ margin: '0 0 10px', fontSize: '13px', color: '#77e18b', textTransform: 'uppercase', letterSpacing: '0.08em' }}>Tool Executor</h3>
      <input value={tool} onChange={(e) => setTool(e.target.value)} placeholder="Tool name" style={{ width: '100%', boxSizing: 'border-box', padding: '7px 8px', borderRadius: '4px', border: '1px solid #3d7244', background: '#102014', color: '#dfffe4', marginBottom: '8px' }} />
      <input value={args} onChange={(e) => setArgs(e.target.value)} placeholder="Arguments" style={{ width: '100%', boxSizing: 'border-box', padding: '7px 8px', borderRadius: '4px', border: '1px solid #3d7244', background: '#102014', color: '#dfffe4' }} />
      <button type="button" onClick={executeTool} style={{ width: '100%', marginTop: '8px', padding: '8px', borderRadius: '4px', border: '1px solid #5bb76d', background: '#17311c', color: '#77e18b', fontWeight: 700, cursor: 'pointer' }}>🛠 Run tool</button>
      <pre style={{ marginTop: '10px', padding: '8px', borderRadius: '6px', border: '1px solid #3d7244', background: '#081009', color: '#b7f5c2', whiteSpace: 'pre-wrap', fontSize: '11px', fontFamily: 'monospace' }}>{result || 'No tool output yet.'}</pre>
    </section>
  );
}
