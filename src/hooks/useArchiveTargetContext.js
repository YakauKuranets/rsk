import { useCallback, useState } from 'react';
import {
  canRunArchiveExport,
  canRunDiscoveryActions,
  canRunStreamVerification,
  canRunVerifiedActions,
  deriveCardKind,
} from '../features/targets/cardKindAdapter';

function buildContextShape(target, source) {
  const snapshot = target && typeof target === 'object' ? target : {};
  return {
    source,
    sourceTargetId: snapshot.id || snapshot.targetId || null,
    targetSnapshot: snapshot,
    cardKind: deriveCardKind(snapshot),
    eligibility: {
      discovery: canRunDiscoveryActions(snapshot),
      verified: canRunVerifiedActions(snapshot),
      streamVerification: canRunStreamVerification(snapshot),
      archiveExport: canRunArchiveExport(snapshot),
    },
  };
}

export function useArchiveTargetContext({ targets, streamTerminal, onDenied }) {
  const [archiveTargetContext, setArchiveTargetContext] = useState(null);

  const findTargetById = useCallback((targetId) => {
    if (!targetId) return null;
    return (targets || []).find((target) => String(target?.id) === String(targetId)) || null;
  }, [targets]);

  const buildArchiveTargetContext = useCallback((target, source) => (
    buildContextShape(target, source)
  ), []);

  const setArchiveContextFromTarget = useCallback((target, source) => {
    const next = buildArchiveTargetContext(target, source);
    setArchiveTargetContext(next);
    return next;
  }, [buildArchiveTargetContext]);

  const resolveArchiveContext = useCallback((source, fallbackTarget = null) => {
    if (fallbackTarget) return setArchiveContextFromTarget(fallbackTarget, source);

    if (archiveTargetContext?.sourceTargetId) {
      const liveTarget = findTargetById(archiveTargetContext.sourceTargetId);
      if (liveTarget) {
        return setArchiveContextFromTarget(liveTarget, `${source}:live_target`);
      }
    }

    if (archiveTargetContext?.targetSnapshot) {
      return buildArchiveTargetContext(
        archiveTargetContext.targetSnapshot,
        `${source}:snapshot_fallback`,
      );
    }

    if (streamTerminal) {
      return setArchiveContextFromTarget(streamTerminal, `${source}:stream_terminal_fallback`);
    }

    return buildArchiveTargetContext({}, `${source}:legacy_unknown`);
  }, [
    archiveTargetContext,
    buildArchiveTargetContext,
    findTargetById,
    setArchiveContextFromTarget,
    streamTerminal,
  ]);

  const ensureArchiveEligibility = useCallback((ctx, key, actionLabel) => {
    if (ctx?.eligibility?.[key]) return true;
    if (typeof onDenied === 'function') {
      onDenied(`${actionLabel} gated for kind=${ctx?.cardKind || 'legacy'}`);
    }
    return false;
  }, [onDenied]);

  return {
    archiveTargetContext,
    buildArchiveTargetContext,
    setArchiveContextFromTarget,
    resolveArchiveContext,
    ensureArchiveEligibility,
  };
}
