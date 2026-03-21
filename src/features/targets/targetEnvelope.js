import { deriveCardKind } from './cardKindAdapter';

const TARGET_ENVELOPE_VERSION = 1;

function inferWriteKind(target) {
  const raw = target || {};
  const explicit = String(raw.kind || '').toLowerCase();
  if (explicit === 'discovery' || explicit === 'verified' || explicit === 'promoted') return explicit;

  const hasPromotion = Boolean(raw.promotionId || raw.promotionStatus || raw.sourceDiscoveryCardId);
  if (hasPromotion) return 'promoted';

  const hasCredentialRef = Boolean(raw.credentialRef);
  const hasPassword = Boolean(String(raw.password || '').trim());
  if (hasCredentialRef || hasPassword) return 'verified';

  if (raw.host || raw.ip || raw.address) return 'discovery';
  return deriveCardKind(raw);
}

export function buildTargetEnvelope(target, source = 'unknown') {
  const kind = inferWriteKind(target);
  const nowIso = new Date().toISOString();
  const payload = { ...(target || {}) };

  return {
    version: TARGET_ENVELOPE_VERSION,
    kind,
    metadata: {
      source,
      writtenAt: nowIso,
      markers: {
        hasCredentials: Boolean(payload.credentialRef || String(payload.password || '').trim()),
        hasPromotion: Boolean(payload.promotionId || payload.promotionStatus || payload.sourceDiscoveryCardId),
      },
    },
    payload,
  };
}

export function unwrapTargetEnvelope(raw) {
  if (
    raw &&
    typeof raw === 'object' &&
    Number.isInteger(raw.version) &&
    raw.version >= 1 &&
    typeof raw.kind === 'string' &&
    raw.payload &&
    typeof raw.payload === 'object'
  ) {
    return {
      target: {
        ...raw.payload,
        __kind: raw.kind,
        __envelopeVersion: raw.version,
        __envelopeMetadata: raw.metadata || null,
      },
      envelope: raw,
      isEnvelope: true,
    };
  }

  return { target: raw, envelope: null, isEnvelope: false };
}

