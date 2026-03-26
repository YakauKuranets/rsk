function unique(items = []) {
  return [...new Set(items.filter(Boolean))];
}

function toLowerText(value) {
  return String(value || '').trim().toLowerCase();
}

const VENDOR_KEYWORD_MAP = [
  { match: /hik|isapi/, hint: 'vendor:hikvision' },
  { match: /dahua/, hint: 'vendor:dahua' },
  { match: /xiongmai|xmeye|xm/, hint: 'vendor:xiongmai' },
  { match: /onvif/, hint: 'vendor:onvif_compatible' },
  { match: /axis/, hint: 'vendor:axis' },
];

const MODEL_KEYWORD_MAP = [
  { match: /ds-2cd|ds-2de|hikvision/, hint: 'model_family:hikvision_ds_series' },
  { match: /ipc-hfw|ipc-hdbw|dahua/, hint: 'model_family:dahua_ipc_series' },
  { match: /nvr|dvr/, hint: 'model_family:nvr_dvr_class' },
];

export function deriveSpiderFingerprintEnrichmentV1(surfaceResult = {}, raw = {}) {
  const vendorHintsBase = Array.isArray(surfaceResult?.vendor_hints) ? surfaceResult.vendor_hints : [];
  const services = Array.isArray(surfaceResult?.services) ? surfaceResult.services : [];
  const endpoints = Array.isArray(surfaceResult?.web_endpoints) ? surfaceResult.web_endpoints : [];

  const vendorText = [
    ...vendorHintsBase,
    raw?.targetCard?.vendorGuess,
    raw?.targetCard?.apiGuess,
    ...(Array.isArray(raw?.techStack) ? raw.techStack : []),
    ...services.map((s) => s?.service || s?.name || s),
    ...endpoints,
    ...(Array.isArray(raw?.pages) ? raw.pages.map((p) => p?.title || p?.url || '') : []),
  ]
    .map((x) => String(x || '').trim())
    .filter(Boolean)
    .join(' | ')
    .toLowerCase();

  const vendorHints = unique(
    VENDOR_KEYWORD_MAP.filter((x) => x.match.test(vendorText)).map((x) => `fp_${x.hint}`),
  );

  const modelHints = unique(
    MODEL_KEYWORD_MAP.filter((x) => x.match.test(vendorText)).map((x) => `fp_${x.hint}`),
  );

  const openPorts = Array.isArray(services)
    ? services.map((s) => Number(s?.port)).filter((n) => Number.isFinite(n))
    : [];
  const comboHints = [];
  if (openPorts.includes(554) && (openPorts.includes(80) || openPorts.includes(443))) {
    comboHints.push('fp_combo:rtsp_plus_web');
  }
  if (openPorts.includes(8000) || openPorts.includes(37777)) {
    comboHints.push('fp_combo:common_nvr_management_port');
  }
  if (openPorts.includes(23)) {
    comboHints.push('fp_combo:telnet_exposed');
  }

  const bannerText = [
    raw?.targetCard?.apiGuess,
    ...(Array.isArray(raw?.pages) ? raw.pages.map((p) => p?.title || '') : []),
    ...services.map((s) => s?.service || s?.name || ''),
  ]
    .map((x) => toLowerText(x))
    .filter(Boolean)
    .join(' | ');

  const bannerHints = unique([
    /webs|goahead|boa/.test(bannerText) ? 'fp_banner:embedded_web_stack' : null,
    /onvif/.test(bannerText) ? 'fp_banner:onvif_surface' : null,
    /isapi/.test(bannerText) ? 'fp_banner:isapi_surface' : null,
  ]);

  const totalHintCount = vendorHints.length + modelHints.length + comboHints.length + bannerHints.length;
  const confidenceDelta = Math.min(0.12, totalHintCount * 0.02);

  return {
    vendorHints,
    modelHints,
    serviceCombinationHints: comboHints,
    bannerCorrelationHints: bannerHints,
    confidenceDelta,
    evidenceRefs: [
      `fingerprint_enrichment_v1:hints=${totalHintCount}`,
      `fingerprint_enrichment_v1:vendor=${vendorHints.length}`,
      `fingerprint_enrichment_v1:model=${modelHints.length}`,
      `fingerprint_enrichment_v1:combo=${comboHints.length}`,
      `fingerprint_enrichment_v1:banner=${bannerHints.length}`,
    ],
  };
}

function clampConfidence(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) return 0;
  return Math.max(0, Math.min(1, Number(n.toFixed(4))));
}

export function applySpiderFingerprintEnrichmentV1(surfaceResult = {}, raw = {}) {
  const enrichment = deriveSpiderFingerprintEnrichmentV1(surfaceResult, raw);

  const mergedVendorHints = unique([
    ...(Array.isArray(surfaceResult?.vendor_hints) ? surfaceResult.vendor_hints : []),
    ...enrichment.vendorHints,
    ...enrichment.modelHints,
    ...enrichment.serviceCombinationHints,
    ...enrichment.bannerCorrelationHints,
  ]);

  const mergedEvidenceRefs = unique([
    ...(Array.isArray(surfaceResult?.evidenceRefs) ? surfaceResult.evidenceRefs : []),
    ...enrichment.evidenceRefs,
  ]);

  return {
    surfaceResult: {
      ...surfaceResult,
      vendor_hints: mergedVendorHints,
      evidenceRefs: mergedEvidenceRefs,
      confidence: clampConfidence(Number(surfaceResult?.confidence || 0) + enrichment.confidenceDelta),
    },
    enrichment,
  };
}

export function formatSpiderFingerprintEnrichmentV1Marker(enrichment = {}, targetId = 'n/a') {
  const total =
    Number(enrichment?.vendorHints?.length || 0) +
    Number(enrichment?.modelHints?.length || 0) +
    Number(enrichment?.serviceCombinationHints?.length || 0) +
    Number(enrichment?.bannerCorrelationHints?.length || 0);

  return [
    'SPIDER_FINGERPRINT_ENRICHMENT_V1',
    `target=${targetId || 'n/a'}`,
    `hints=${total}`,
    `vendorHints=${Number(enrichment?.vendorHints?.length || 0)}`,
    `modelHints=${Number(enrichment?.modelHints?.length || 0)}`,
    `comboHints=${Number(enrichment?.serviceCombinationHints?.length || 0)}`,
    `bannerHints=${Number(enrichment?.bannerCorrelationHints?.length || 0)}`,
    `confidenceDelta=${Number(enrichment?.confidenceDelta || 0).toFixed(4)}`,
  ].join('|');
}
