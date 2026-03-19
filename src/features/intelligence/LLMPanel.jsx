import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

const box = { width: '100%', boxSizing: 'border-box', padding: '7px 8px', background: '#0d1017', color: '#d6faff', border: '1px solid #29414f', borderRadius: '4px' };

export default function LLMPanel() {
  const [prompt, setPrompt] = useState('Summarize target posture and recommend the next safest verification step.');
  const [response, setResponse] = useState('');
  const [health, setHealth] = useState('Unknown');

  const checkHealth = async () => {
    try {
      const ok = await invoke('llm_health_check', { ollamaUrl: 'http://localhost:11434' });
      setHealth(ok ? 'Ollama reachable' : 'Ollama not reachable');
    } catch (error) {
      setHealth(`Health check failed: ${error}`);
    }
  };

  const generatePlan = async () => {
    try {
      const res = await invoke('llm_generate_attack_plan', {
        targetProfile: prompt,
        config: { ollamaUrl: 'http://localhost:11434', model: 'llama3', temperature: 0.3 },
      });
      setResponse(Array.isArray(res) ? res.join('\n') : String(res));
    } catch (error) {
      setResponse(`LLM unavailable: ${error}`);
    }
  };

  return (
    <section style={{ border: '1px solid #24404e', borderRadius: '8px', padding: '12px', background: '#071217', color: '#d6faff' }}>
      <h3 style={{ margin: '0 0 10px', fontSize: '13px', color: '#79e4ff', textTransform: 'uppercase', letterSpacing: '0.08em' }}>LLM Panel</h3>
      <textarea rows={5} style={{ ...box, resize: 'vertical' }} value={prompt} onChange={(e) => setPrompt(e.target.value)} />
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px', marginTop: '8px' }}>
        <button type="button" style={{ ...box, cursor: 'pointer', fontWeight: 700, background: '#11303a', color: '#79e4ff' }} onClick={checkHealth}>🩺 Health check</button>
        <button type="button" style={{ ...box, cursor: 'pointer', fontWeight: 700, background: '#132935', color: '#79e4ff' }} onClick={generatePlan}>🧠 Generate plan</button>
      </div>
      <div style={{ marginTop: '10px', fontSize: '11px', color: '#9fc6d5' }}>{health}</div>
      <pre style={{ marginTop: '10px', ...box, whiteSpace: 'pre-wrap', fontFamily: 'monospace', fontSize: '11px' }}>{response || 'No LLM output yet.'}</pre>
    </section>
  );
}
