const KIND_DISCOVERY = 'discovery';
const KIND_VERIFIED = 'verified';
const KIND_PROMOTED = 'promoted';
const KIND_LEGACY = 'legacy';

function normalizeKind(rawKind) {
  const value = String(rawKind || '').trim().toLowerCase();
  if (value === KIND_DISCOVERY) return KIND_DISCOVERY;
  if (value === KIND_VERIFIED) return KIND_VERIFIED;
  if (value === KIND_PROMOTED) return KIND_PROMOTED;
  return null;
}

function hasAnyField(target, fields) {
  if (!target || typeof target !== 'object') return false;
  return fields.some((field) => target[field] !== undefined && target[field] !== null && target[field] !== '');
}

function getEnvelopeKind(target) {
  if (!target || typeof target !== 'object') return null;
  if (target.kind && target.data && typeof target.data === 'object') {
    return normalizeKind(target.kind);
  }
  if (target.card && typeof target.card === 'object' && target.card.kind) {
    return normalizeKind(target.card.kind);
  }
  if (target.envelope && typeof target.envelope === 'object' && target.envelope.kind) {
    return normalizeKind(target.envelope.kind);
  }
  return null;
}

function hasPromotionMetadata(target) {
  return hasAnyField(target, [
    'promotionId',
    'promotionStatus',
    'promotionReason',
    'sourceDiscoveryCardId',
    'sourceDiscoveryIp',
    'targetVerifiedCardId',
    'confidence',
  ]);
}

function hasCredentialMetadata(target) {
  return hasAnyField(target, [
    'credentialRef',
    'verificationStatus',
    'verifiedStatus',
    'verifiedAt',
    'streamAuthMode',
    'archiveAuthMode',
    'login',
    'password',
  ]);
}

function hasDiscoveryMetadata(target) {
  return hasAnyField(target, [
    'discoveryStatus',
    'authRequired',
    'streamCapability',
    'archiveCapability',
    'suspectedVendor',
    'scanProfile',
    'ip',
    'host',
    'address',
  ]);
}

export function deriveCardKind(target) {
  const explicitKind = getEnvelopeKind(target);
  if (explicitKind) return explicitKind;

  if (hasPromotionMetadata(target)) return KIND_PROMOTED;
  if (hasCredentialMetadata(target)) return KIND_VERIFIED;
  if (hasDiscoveryMetadata(target)) return KIND_DISCOVERY;

  return KIND_LEGACY;
}

export function isDiscoveryCard(target) {
  return deriveCardKind(target) === KIND_DISCOVERY;
}

export function isVerifiedCard(target) {
  return deriveCardKind(target) === KIND_VERIFIED;
}

export function isPromotedCard(target) {
  return deriveCardKind(target) === KIND_PROMOTED;
}

export function isCardKindGatingEnabled() {
  return String(import.meta.env.VITE_ENABLE_CARD_KIND_GATING || '').toLowerCase() === 'true';
}

export function canRunDiscoveryActions(target, options = {}) {
  const enabled = options.enabled ?? isCardKindGatingEnabled();
  if (!enabled) return true;
  const kind = deriveCardKind(target);
  return kind === KIND_DISCOVERY || kind === KIND_PROMOTED || kind === KIND_LEGACY;
}

export function canRunVerifiedActions(target, options = {}) {
  const enabled = options.enabled ?? isCardKindGatingEnabled();
  if (!enabled) return true;
  const kind = deriveCardKind(target);
  return kind === KIND_VERIFIED || kind === KIND_PROMOTED || kind === KIND_LEGACY;
}

export function canRunPromotionActions(target, options = {}) {
  const enabled = options.enabled ?? isCardKindGatingEnabled();
  if (!enabled) return true;
  const kind = deriveCardKind(target);
  return kind === KIND_DISCOVERY || kind === KIND_LEGACY;
}

export function canRunArchiveExport(target, options = {}) {
  if (target?.type === 'hub') return false;
  return canRunVerifiedActions(target, options);
}

export function canRunStreamVerification(target, options = {}) {
  if (target?.type === 'hub') return false;
  const enabled = options.enabled ?? isCardKindGatingEnabled();
  if (!enabled) return true;
  const kind = deriveCardKind(target);
  return kind === KIND_VERIFIED || kind === KIND_PROMOTED || kind === KIND_LEGACY;
}
