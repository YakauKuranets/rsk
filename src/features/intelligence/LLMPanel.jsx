import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../store/appStore';

const box = { width: '100%', boxSizing: 'border-box', padding: '7px 8px', background: '#0d1017', color: '#d6faff', border: '1px solid #29414f', borderRadius: '4px' };

export default function LLMPanel() {
  const intelligenceTarget = useAppStore((s) => s.intelligenceTarget);
  const ollamaUrl = useAppStore((s) => s.ollamaUrl);
  const ollamaModel = useAppStore((s) => s.ollamaModel);
  const ollamaTemperature = useAppStore((s) => s.ollamaTemperature);
  const [prompt, setPrompt] = useState('Summarize target posture and recommend the next safest verification step.');
  const [response, setResponse] = useState('');
  const [health, setHealth] = useState('Unknown');
  const [deepThinking, setDeepThinking] = useState(false);
  const [showStartupPrompt, setShowStartupPrompt] = useState(false);

  const checkHealth = async () => {
    try {
      const ok = await invoke('llm_health_check', { ollamaUrl });
      setHealth(ok ? 'Ollama reachable' : 'Ollama not reachable');
      setShowStartupPrompt(ok);
    } catch (error) {
      setHealth(`Health check failed: ${error}`);
      setShowStartupPrompt(false);
    }
  };

  const generatePlan = async () => {
    try {
      const finalPrompt = [
        intelligenceTarget,
        prompt,
        deepThinking
          ? 'Use deep, step-by-step strategic reasoning and provide a structured plan with assumptions, risks, and validation checkpoints.'
          : '',
      ]
        .filter(Boolean)
        .join('\n\n');

      const res = await invoke('llm_generate_attack_plan', {
        targetProfile: finalPrompt,
        config: { ollamaUrl, model: ollamaModel, temperature: Number(ollamaTemperature) || 0.3 },
      });
      setResponse(Array.isArray(res) ? res.join('\n') : String(res));
    } catch (error) {
      setResponse(`LLM unavailable: ${error}`);
    }
  };

  return (
    <section style={{ border: '1px solid #24404e', borderRadius: '8px', padding: '12px', background: '#071217', color: '#d6faff' }}>
      <h3 style={{ margin: '0 0 10px', fontSize: '13px', color: '#79e4ff', textTransform: 'uppercase', letterSpacing: '0.08em' }}>LLM Panel</h3>
      <div style={{ marginBottom: '8px', fontSize: '11px', color: '#9fc6d5' }}>Target context: {intelligenceTarget || 'не задан'}</div>
      <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '11px', color: '#a9d8e8', marginBottom: '8px' }}>
        <input type="checkbox" checked={deepThinking} onChange={(e) => setDeepThinking(e.target.checked)} />
        Deep thinking mode (рекомендуется для DeepSeek)
      </label>
      <textarea rows={5} style={{ ...box, resize: 'vertical' }} value={prompt} onChange={(e) => setPrompt(e.target.value)} />
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px', marginTop: '8px' }}>
        <button type="button" style={{ ...box, cursor: 'pointer', fontWeight: 700, background: '#11303a', color: '#79e4ff' }} onClick={checkHealth}>🩺 Health check</button>
        <button type="button" style={{ ...box, cursor: 'pointer', fontWeight: 700, background: '#132935', color: '#79e4ff' }} onClick={generatePlan}>🧠 Generate plan</button>
      </div>
      {showStartupPrompt && (
        <div style={{ marginTop: '10px', padding: '8px 10px', border: '1px solid #2f6d45', borderRadius: '6px', background: '#102017', color: '#8ef5ab', fontSize: '11px' }}>
          ✅ Всё работает. LLM доступен — можно нажимать <strong>Generate plan</strong>.
          <button type="button" onClick={() => setShowStartupPrompt(false)} style={{ marginLeft: '10px', border: '1px solid #356a46', background: '#12301f', color: '#8ef5ab', borderRadius: '4px', cursor: 'pointer', padding: '2px 8px', fontSize: '10px' }}>
            Закрыть
          </button>
        </div>
      )}
      <div style={{ marginTop: '10px', fontSize: '11px', color: '#9fc6d5' }}>{health}</div>
      <pre style={{ marginTop: '10px', ...box, whiteSpace: 'pre-wrap', fontFamily: 'monospace', fontSize: '11px' }}>{response || 'No LLM output yet.'}</pre>
    </section>
  );
}
