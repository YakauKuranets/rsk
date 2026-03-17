import React, { useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

function normalizeTargets(targets) {
  return (targets || []).map((t) => {
    const openPorts = Array.isArray(t?.openPorts)
      ? t.openPorts
      : Array.isArray(t?.open_ports)
        ? t.open_ports
        : [];

    const vulnerabilities = Array.isArray(t?.vulnerabilities)
      ? t.vulnerabilities.map((v) => ({
        cveId: v?.cveId || v?.cve_id || 'unknown_vuln',
        cvssScore: Number(v?.cvssScore ?? v?.cvss_score ?? 0),
      }))
      : [];

    const credentials = t?.credentials || null;

    return {
      ip: t?.host || t?.ip || 'unknown',
      openPorts: openPorts.map((p) => Number(p?.port ?? p)).filter((p) => !Number.isNaN(p)),
      vulnerabilities,
      credentials,
    };
  });
}

export default function AttackGraph({ targets }) {
  const [graph, setGraph] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const campaignTargets = useMemo(() => normalizeTargets(targets), [targets]);

  const runBuild = async () => {
    setLoading(true);
    setError('');
    try {
      const result = await invoke('generate_attack_graph', {
        targetsJson: JSON.stringify(campaignTargets),
      });
      setGraph(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const positions = useMemo(() => {
    if (!graph?.nodes?.length) return null;

    const width = 700;
    const height = 420;
    const centerX = width / 2;
    const centerY = height / 2;
    const radius = Math.max(80, Math.min(170, graph.nodes.length * 8));

    const nodes = graph.nodes.map((n, i) => {
      const angle = (i / Math.max(1, graph.nodes.length)) * Math.PI * 2;
      return {
        ...n,
        x: centerX + Math.cos(angle) * radius,
        y: centerY + Math.sin(angle) * radius,
      };
    });

    const byId = Object.fromEntries(nodes.map((n) => [n.id, n]));
    const links = (graph.edges || []).map((e) => ({ source: byId[e.from], target: byId[e.to] })).filter((l) => l.source && l.target);

    return { nodes, links };
  }, [graph]);

  const colorByType = (type) => {
    if (type === 'host') return '#00f0ff';
    if (type === 'service') return '#55ff88';
    if (type === 'vulnerability') return '#ff4d6d';
    if (type === 'credential') return '#ffd166';
    return '#ccc';
  };

  return (
    <div style={{ border: '1px solid #a96bff', padding: 10, backgroundColor: '#170026', marginBottom: 20 }}>
      <h3 style={{ color: '#d2a8ff', marginTop: 0, fontSize: '0.9rem' }}>🕸️ Attack Graph Generator</h3>
      <div style={{ color: '#b79ddb', fontSize: 11, marginBottom: 8 }}>
        Targets in model: {campaignTargets.length}
      </div>
      <button
        onClick={runBuild}
        disabled={loading}
        style={{ backgroundColor: '#a96bff', color: '#140022', border: 'none', padding: '8px 10px', fontWeight: 'bold', cursor: 'pointer' }}
      >
        {loading ? 'Строим граф...' : 'Построить граф атаки'}
      </button>

      {error && <div style={{ color: '#ff8aa1', marginTop: 8, fontSize: 12 }}>{error}</div>}

      {graph && (
        <div style={{ marginTop: 10 }}>
          <div style={{ fontSize: 11, color: '#dfc8ff', marginBottom: 8 }}>
            Nodes: {graph.nodes.length} · Edges: {graph.edges.length} · Paths: {graph.attackPaths.length} · Risk: {Number(graph.riskScore || 0).toFixed(2)}
          </div>

          {positions && (
            <svg width="700" height="420" style={{ background: '#0c0015', border: '1px solid #3c1f58' }}>
              {positions.links.map((l, idx) => (
                <line
                  key={`l_${idx}`}
                  x1={l.source.x}
                  y1={l.source.y}
                  x2={l.target.x}
                  y2={l.target.y}
                  stroke="#5c3b7a"
                  strokeWidth="1.2"
                />
              ))}
              {positions.nodes.map((n) => (
                <g key={n.id}>
                  <circle cx={n.x} cy={n.y} r="8" fill={colorByType(n.nodeType)} />
                  <text x={n.x + 10} y={n.y + 4} fill="#d8c6f0" fontSize="10">{n.label}</text>
                </g>
              ))}
            </svg>
          )}
        </div>
      )}
    </div>
  );
}
