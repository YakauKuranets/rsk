import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

function FindingsTable({ findings = [] }) {
  return (
    <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12 }}>
      <thead>
        <tr style={{ color: '#00f0ff', textAlign: 'left' }}>
          <th style={{ padding: '8px 6px' }}>Host</th>
          <th style={{ padding: '8px 6px' }}>Type</th>
          <th style={{ padding: '8px 6px' }}>Severity</th>
          <th style={{ padding: '8px 6px' }}>Description</th>
        </tr>
      </thead>
      <tbody>
        {findings.map((finding, index) => (
          <tr key={`${finding.host}-${index}`} style={{ borderTop: '1px solid #23232a' }}>
            <td style={{ padding: '8px 6px', verticalAlign: 'top' }}>{finding.host}</td>
            <td style={{ padding: '8px 6px', verticalAlign: 'top' }}>{finding.findingType}</td>
            <td style={{ padding: '8px 6px', verticalAlign: 'top' }}>{finding.severity}</td>
            <td style={{ padding: '8px 6px', verticalAlign: 'top' }}>{finding.description}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function RiskIndicators({ indicators = [] }) {
  if (!indicators.length) {
    return <div style={{ color: '#7f8a99', fontSize: 12 }}>Risk indicators отсутствуют.</div>;
  }

  return (
    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
      {indicators.map((indicator) => (
        <span
          key={indicator}
          style={{
            padding: '4px 8px',
            border: '1px solid #ff003c66',
            borderRadius: 999,
            fontSize: 11,
            color: '#ff7d95',
          }}
        >
          {indicator}
        </span>
      ))}
    </div>
  );
}

function HandoffModal({
  nextAgent,
  needsPermit,
  permit,
  setPermit,
  notes,
  setNotes,
  scopeConfirmed,
  setScopeConfirmed,
  onConfirm,
  onCancel,
  busy,
}) {
  return (
    <div style={{ position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.7)', display: 'grid', placeItems: 'center', zIndex: 9999 }}>
      <div style={{ width: 420, background: '#111115', border: '1px solid #2c2c34', padding: 20, display: 'grid', gap: 12 }}>
        <h3 style={{ margin: 0, color: '#ff003c' }}>Подтвердить handoff → {nextAgent}</h3>
        {needsPermit && (
          <input value={permit} onChange={(e) => setPermit(e.target.value)} placeholder="PT-2026-001" />
        )}
        <textarea
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          placeholder="Operator notes"
          rows={4}
          style={{ resize: 'vertical' }}
        />
        <label style={{ display: 'flex', gap: 8, alignItems: 'center', fontSize: 12 }}>
          <input type="checkbox" checked={scopeConfirmed} onChange={(e) => setScopeConfirmed(e.target.checked)} />
          Scope и findings подтверждены оператором
        </label>
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
          <button onClick={onCancel} disabled={busy}>Отмена</button>
          <button onClick={onConfirm} disabled={busy || !scopeConfirmed}>
            {busy ? 'Проверка...' : 'Подтвердить передачу'}
          </button>
        </div>
      </div>
    </div>
  );
}

export function AgentReport({ packet, nextAgent, onHandoff }) {
  const [scrolled, setScrolled] = useState(false);
  const [showModal, setShowModal] = useState(false);
  const [notes, setNotes] = useState('');
  const [permit, setPermit] = useState('');
  const [scopeConfirmed, setScopeConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const reportRef = useRef(null);

  useEffect(() => {
    setScrolled(false);
    setShowModal(false);
    setNotes('');
    setPermit('');
    setScopeConfirmed(false);
    requestAnimationFrame(() => {
      const el = reportRef.current;
      if (el) {
        el.scrollTop = 0;
        if (el.scrollHeight <= el.clientHeight + 40) {
          setScrolled(true);
        }
      }
    });
  }, [packet?.pipelineId]);

  const handleScroll = useCallback(() => {
    const el = reportRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop <= el.clientHeight + 40;
    if (atBottom) setScrolled(true);
  }, []);

  const needsPermit = nextAgent === 'ExploitVerifyAgent';
  const permitDate = useMemo(() => {
    const match = permit.match(/^PT-(\d{4})-(\d{3,})$/);
    if (!match) return '';
    return `${match[1]}-12-31`;
  }, [permit]);

  async function handleConfirmHandoff() {
    if (!scopeConfirmed || busy) return;
    setBusy(true);
    try {
      if (needsPermit) {
        await invoke('validate_exploit_authorization', {
          request: {
            targetIps: [...new Set((packet?.findings || []).map((finding) => finding.host).filter(Boolean))],
            permitNumber: permit,
            permitDate,
            operatorId: 'operator',
          },
        });
      }

      onHandoff({ ...packet, operatorNotes: notes || null });
      setShowModal(false);
    } catch (error) {
      alert(`Ошибка авторизации: ${error}`);
    } finally {
      setBusy(false);
    }
  }

  if (!packet) return null;

  return (
    <div style={{ border: '1px solid #2a2a2e', background: '#0d0d11', padding: 12, display: 'grid', gap: 12 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: 12, alignItems: 'center' }}>
        <div>
          <div style={{ color: '#ff003c', fontWeight: 700 }}>Agent Report</div>
          <div style={{ fontSize: 12, color: '#7f8a99' }}>
            pipeline={packet.pipelineId} • status={typeof packet.status === 'string' ? packet.status : packet.status?.kind || 'partial'}
          </div>
        </div>
        <div style={{ fontSize: 12, color: '#7f8a99' }}>{packet.timestampUtc}</div>
      </div>

      <div ref={reportRef} onScroll={handleScroll} style={{ maxHeight: 320, overflowY: 'auto', border: '1px solid #23232a', padding: 8 }}>
        <FindingsTable findings={packet.findings} />
        <div style={{ marginTop: 12, display: 'grid', gap: 6 }}>
          <strong style={{ color: '#00f0ff', fontSize: 12 }}>Risk indicators</strong>
          <RiskIndicators indicators={packet.riskIndicators} />
        </div>
      </div>

      <button disabled={!scrolled} onClick={() => setShowModal(true)} style={{ opacity: scrolled ? 1 : 0.45 }}>
        {scrolled ? `↓ Передать в ${nextAgent}` : 'Прокрутите отчёт до конца'}
      </button>

      {showModal && (
        <HandoffModal
          nextAgent={nextAgent}
          needsPermit={needsPermit}
          permit={permit}
          setPermit={setPermit}
          notes={notes}
          setNotes={setNotes}
          scopeConfirmed={scopeConfirmed}
          setScopeConfirmed={setScopeConfirmed}
          onConfirm={handleConfirmHandoff}
          onCancel={() => setShowModal(false)}
          busy={busy}
        />
      )}
    </div>
  );
}

export default AgentReport;
