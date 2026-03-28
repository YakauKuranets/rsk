import { normalizeArchiveAuthResultV1 } from './authResultContract';

export const ARCHIVE_RESULT_CONTRACT_VERSION = 'archive_result_v1';

const DEFAULT_RESULT = {
  contractVersion: ARCHIVE_RESULT_CONTRACT_VERSION,
  target_id: null,
  archive_path_type: 'unknown',
  search_supported: false,
  search_requires_auth: true,
  export_supported: false,
  export_requires_auth: true,
  partial_access_detected: false,
  timeout_detected: false,
  integrity_status: 'unknown',
  issues: [],
  issuesCount: 0,
  evidenceRefs: [],
  confidence: 0,
  resultClass: 'inconclusive',
};

function toIssueArray(issues) {
  if (!Array.isArray(issues)) return [];
  return issues.map((item) => String(item ?? '').trim()).filter(Boolean);
}

function toEvidenceRefs(refs) {
  if (!Array.isArray(refs)) return [];
  return refs.map((item) => String(item ?? '').trim()).filter(Boolean);
}

function clampConfidence(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) return 0;
  return Math.max(0, Math.min(1, Number(n.toFixed(4))));
}

export function normalizeArchiveResultV1(input = {}) {
  const issues = toIssueArray(input.issues);
  const evidenceRefs = toEvidenceRefs(input.evidenceRefs);
  const resultClass = ['passed', 'failed', 'inconclusive'].includes(input.resultClass)
    ? input.resultClass
    : 'inconclusive';

  const normalized = {
    ...DEFAULT_RESULT,
    ...input,
    contractVersion: ARCHIVE_RESULT_CONTRACT_VERSION,
    target_id: input.target_id ? String(input.target_id) : null,
    archive_path_type: input.archive_path_type ? String(input.archive_path_type) : 'unknown',
    search_supported: Boolean(input.search_supported),
    search_requires_auth:
      typeof input.search_requires_auth === 'boolean' ? input.search_requires_auth : true,
    export_supported: Boolean(input.export_supported),
    export_requires_auth:
      typeof input.export_requires_auth === 'boolean' ? input.export_requires_auth : true,
    partial_access_detected: Boolean(input.partial_access_detected),
    timeout_detected: Boolean(input.timeout_detected),
    integrity_status: input.integrity_status ? String(input.integrity_status) : 'unknown',
    issues,
    issuesCount: issues.length,
    evidenceRefs,
    confidence: clampConfidence(input.confidence),
    resultClass,
  };

  return {
    ...normalized,
    authResult: normalizeArchiveAuthResultV1({
      archiveResult: normalized,
    }),
  };
}

export function normalizeArchiveSearchResultV1({
  targetId,
  archivePathType,
  records = [],
  errors = [],
  timeoutDetected = false,
  partialAccessDetected = false,
  evidenceRefs = [],
  confidence,
} = {}) {
  const list = Array.isArray(records) ? records : [];
  const issueList = [...toIssueArray(errors)];
  const hasRecords = list.length > 0;

  if (!hasRecords && issueList.length === 0) {
    issueList.push('archive_search_returned_no_records');
  }

  return normalizeArchiveResultV1({
    target_id: targetId,
    archive_path_type: archivePathType || 'search',
    search_supported: true,
    search_requires_auth: true,
    export_supported: hasRecords,
    export_requires_auth: true,
    partial_access_detected: partialAccessDetected,
    timeout_detected: timeoutDetected,
    integrity_status: hasRecords ? 'unknown' : 'degraded',
    issues: issueList,
    evidenceRefs: [
      `records:${list.length}`,
      ...toEvidenceRefs(evidenceRefs),
    ],
    confidence: confidence ?? (hasRecords ? 0.7 : issueList.length ? 0.45 : 0.3),
    resultClass: timeoutDetected ? 'inconclusive' : hasRecords ? 'passed' : 'failed',
  });
}

export function normalizeArchiveExportResultV1({
  targetId,
  archivePathType,
  ok = false,
  timeoutDetected = false,
  partialAccessDetected = false,
  integrityStatus = 'unknown',
  issues = [],
  evidenceRefs = [],
  confidence,
} = {}) {
  const issueList = toIssueArray(issues);
  if (!ok && issueList.length === 0) issueList.push('archive_export_failed');

  const resultClass = timeoutDetected
    ? 'inconclusive'
    : ok && !partialAccessDetected
      ? 'passed'
      : ok && partialAccessDetected
        ? 'inconclusive'
        : 'failed';

  return normalizeArchiveResultV1({
    target_id: targetId,
    archive_path_type: archivePathType || 'export',
    search_supported: true,
    search_requires_auth: true,
    export_supported: true,
    export_requires_auth: true,
    partial_access_detected: partialAccessDetected,
    timeout_detected: timeoutDetected,
    integrity_status: integrityStatus,
    issues: issueList,
    evidenceRefs,
    confidence: confidence ?? (ok ? 0.75 : timeoutDetected ? 0.4 : 0.2),
    resultClass,
  });
}
