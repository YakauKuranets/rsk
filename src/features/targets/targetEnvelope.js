import { deriveCardKind } from './cardKindAdapter';

const TARGET_ENVELOPE_VERSION = 1;
const ALLOWED_VERSIONS = new Set([1]);
const ALLOWED_KINDS = new Set(['discovery', 'verified', 'promoted', 'legacy']);

function isPlainObject(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function normalizeKind(kind) {
  const value = String(kind || '').trim().toLowerCase();
  return ALLOWED_KINDS.has(value) ? value : null;
}

export function validateTargetEnvelope(raw) {
  if (!isPlainObject(raw)) {
    return { valid: false, reason: 'not_object' };
  }

  if (!ALLOWED_VERSIONS.has(Number(raw.version))) {
    return { valid: false, reason: 'invalid_version' };
  }

  const normalizedKind = normalizeKind(raw.kind);
  if (!normalizedKind) {
    return { valid: false, reason: 'invalid_kind' };
  }

  if (!isPlainObject(raw.payload)) {
    return { valid: false, reason: 'missing_payload' };
  }

  return { valid: true, reason: 'ok', kind: normalizedKind };
}

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
  const validation = validateTargetEnvelope(target);
  if (validation.valid) {
    return {
      ...target,
      kind: validation.kind,
      metadata: {
        ...(isPlainObject(target.metadata) ? target.metadata : {}),
        source: source || target?.metadata?.source || 'unknown',
        writtenAt: new Date().toISOString(),
      },
    };
  }

  const raw = isPlainObject(target) ? target : {};
  const kind = inferWriteKind(raw);
  const nowIso = new Date().toISOString();
  const payload = { ...raw };

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
  const validation = validateTargetEnvelope(raw);
  if (validation.valid) {
    return {
      target: {
        ...raw.payload,
        __kind: validation.kind,
        __envelopeVersion: Number(raw.version),
        __envelopeMetadata: raw.metadata || null,
      },
      envelope: raw,
      isEnvelope: true,
    };
  }

  if (isPlainObject(raw) && raw.__envelopeVersion && raw.__kind) {
    return { target: raw, envelope: null, isEnvelope: false };
  }

  if (!isPlainObject(raw)) {
    return { target: {}, envelope: null, isEnvelope: false };
  }

  return { target: raw, envelope: null, isEnvelope: false };
}
