import { ARCHIVE_RESULT_CONTRACT_VERSION, normalizeArchiveResultV1 } from './archiveResultContract';
import { AUTH_RESULT_CONTRACT_VERSION, validateAuthResultV1Shape } from './authResultContract';
import { runArchiveBaselinePackV1 } from './archiveBaselinePack';
import { runArchiveEdgeCaseHarnessV1 } from './archiveEdgeCaseHarness';

export const SAFE_ARCHIVE_FUZZ_LIMITS_V1 = {
  max_mutations_per_run: 40,
  max_runtime_ms: 1200,
};

export const DEFAULT_ARCHIVE_FUZZ_SEEDS_V1 = [
  {
    seed: 'seed_good_isapi',
    input: {
      target_id: 'fuzz_seed_target_a',
      archive_path_type: 'isapi_search',
      search_supported: true,
      search_requires_auth: true,
      export_supported: true,
      export_requires_auth: true,
      partial_access_detected: false,
      timeout_detected: false,
      integrity_status: 'ok',
      issues: [],
      evidenceRefs: ['seed:isapi_search'],
      confidence: 0.83,
      resultClass: 'passed',
    },
  },
  {
    seed: 'seed_partial_probe',
    input: {
      target_id: 'fuzz_seed_target_b',
      archive_path_type: 'archive_export_probe',
      search_supported: true,
      search_requires_auth: true,
      export_supported: true,
      export_requires_auth: true,
      partial_access_detected: true,
      timeout_detected: false,
      integrity_status: 'degraded',
      issues: ['partial_access_detected'],
      evidenceRefs: ['seed:partial_probe'],
      confidence: 0.55,
      resultClass: 'inconclusive',
    },
  },
];

const MUTATION_TYPES = [
  'time_range',
  'channel',
  'parameter_presence',
  'malformed_safe_success_like',
  'auth_boundary_adjacent',
];

function stableHash(seedText) {
  const text = String(seedText || 'seed');
  let h = 2166136261;
  for (let i = 0; i < text.length; i += 1) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function makePrng(seedText) {
  let state = stableHash(seedText) || 1;
  return () => {
    state = (Math.imul(1664525, state) + 1013904223) >>> 0;
    return state / 0x100000000;
  };
}

function pick(arr, rnd) {
  if (!Array.isArray(arr) || arr.length === 0) return null;
  return arr[Math.floor(rnd() * arr.length)] || arr[0];
}

function mutateInput(base, mutationType, rnd) {
  const next = {
    ...base,
    issues: Array.isArray(base.issues) ? [...base.issues] : [],
    evidenceRefs: Array.isArray(base.evidenceRefs) ? [...base.evidenceRefs] : [],
  };

  if (mutationType === 'time_range') {
    const mode = pick(['empty_range', 'shifted_range', 'inverted_range'], rnd);
    next.issues.push(`time_mutation:${mode}`);
    if (mode === 'inverted_range') {
      next.resultClass = 'failed';
      next.integrity_status = 'failed';
    } else {
      next.resultClass = 'inconclusive';
      next.integrity_status = 'degraded';
    }
    next.evidenceRefs.push(`mutation:time_range:${mode}`);
    return {
      mutated: next,
      expectedBehavior: mode === 'inverted_range' ? 'failed' : 'inconclusive',
      detail: mode,
    };
  }

  if (mutationType === 'channel') {
    const mode = pick(['empty_channel', 'negative_channel', 'high_channel'], rnd);
    next.issues.push(`channel_mutation:${mode}`);
    next.resultClass = mode === 'empty_channel' ? 'failed' : 'inconclusive';
    next.integrity_status = 'degraded';
    next.evidenceRefs.push(`mutation:channel:${mode}`);
    return {
      mutated: next,
      expectedBehavior: mode === 'empty_channel' ? 'failed' : 'inconclusive',
      detail: mode,
    };
  }

  if (mutationType === 'parameter_presence') {
    const mode = pick(['missing_target_id', 'missing_archive_path_type', 'missing_integrity_status'], rnd);
    if (mode === 'missing_target_id') next.target_id = null;
    if (mode === 'missing_archive_path_type') next.archive_path_type = '';
    if (mode === 'missing_integrity_status') next.integrity_status = '';
    next.issues.push(`param_omission:${mode}`);
    next.resultClass = mode === 'missing_target_id' ? 'failed' : 'inconclusive';
    next.evidenceRefs.push(`mutation:param_presence:${mode}`);
    return {
      mutated: next,
      expectedBehavior: mode === 'missing_target_id' ? 'failed' : 'inconclusive',
      detail: mode,
    };
  }

  if (mutationType === 'malformed_safe_success_like') {
    const mode = pick(['success_like_with_failed_integrity', 'success_like_with_issues'], rnd);
    next.resultClass = 'passed';
    next.integrity_status = 'failed';
    next.issues.push(`malformed_success:${mode}`);
    next.evidenceRefs.push(`mutation:malformed_success:${mode}`);
    return {
      mutated: next,
      expectedBehavior: 'failed',
      detail: mode,
    };
  }

  const mode = pick(['auth_flag_inconsistent', 'partial_without_issue'], rnd);
  if (mode === 'auth_flag_inconsistent') {
    next.search_requires_auth = false;
    next.export_requires_auth = false;
    next.issues.push('auth_boundary_inconsistent');
    next.resultClass = 'failed';
  } else {
    next.partial_access_detected = true;
    next.resultClass = 'inconclusive';
    next.issues.push('partial_access_without_auth_proof');
  }
  next.evidenceRefs.push(`mutation:auth_boundary_adjacent:${mode}`);
  return {
    mutated: next,
    expectedBehavior: mode === 'auth_flag_inconsistent' ? 'failed' : 'inconclusive',
    detail: mode,
  };
}

function buildMutationReport({ seed, mutationType, mutationDetail, expectedBehavior, normalized }) {
  const actualBehavior = normalized.resultClass;
  const classMatch = actualBehavior === expectedBehavior;
  const issuesCountMatch = normalized.issuesCount === normalized.issues.length;
  const contractVersionOk = normalized.contractVersion === ARCHIVE_RESULT_CONTRACT_VERSION;
  const authShapeOk = validateAuthResultV1Shape(normalized?.authResult).ok;
  const authTargetMatch = normalized?.authResult?.target_id === normalized?.target_id;
  const authPathTypeMatch =
    normalized?.authResult?.auth_path_type === `archive:${String(normalized?.archive_path_type || 'unknown')}`;

  const status = classMatch && issuesCountMatch && contractVersionOk && authShapeOk && authTargetMatch && authPathTypeMatch
    ? 'passed'
    : !issuesCountMatch || !contractVersionOk || !authShapeOk
      ? 'failed'
      : 'inconclusive';

  return {
    seed,
    mutationType,
    mutationDetail,
    expectedBehavior,
    actualBehavior,
    status,
    checks: {
      classMatch,
      issuesCountMatch,
      contractVersionOk,
      authShapeOk,
      authTargetMatch,
      authPathTypeMatch,
    },
    result: normalized,
  };
}

export function archiveFuzzMetrics(mutationReports = []) {
  const total = mutationReports.length;
  const byStatus = { passed: 0, failed: 0, inconclusive: 0 };
  const byMutationType = {};
  const byActualBehavior = { passed: 0, failed: 0, inconclusive: 0 };

  for (const report of mutationReports) {
    byStatus[report.status] = Number(byStatus[report.status] || 0) + 1;
    byMutationType[report.mutationType] = Number(byMutationType[report.mutationType] || 0) + 1;
    byActualBehavior[report.actualBehavior] = Number(byActualBehavior[report.actualBehavior] || 0) + 1;
  }

  const matchCount = mutationReports.filter((item) => item.checks.classMatch).length;
  return {
    total,
    byStatus,
    byMutationType,
    byActualBehavior,
    classMatchRate: total ? Number((matchCount / total).toFixed(4)) : 0,
    failedCount: byStatus.failed,
    inconclusiveCount: byStatus.inconclusive,
  };
}

export async function runSafeArchiveFuzzLayerV1({
  seeds = DEFAULT_ARCHIVE_FUZZ_SEEDS_V1,
  maxMutationsPerRun = SAFE_ARCHIVE_FUZZ_LIMITS_V1.max_mutations_per_run,
  maxRuntimeMs = SAFE_ARCHIVE_FUZZ_LIMITS_V1.max_runtime_ms,
  includeContinuity = true,
} = {}) {
  const startedAt = Date.now();
  const safeMaxMutations = Math.max(1, Math.min(Number(maxMutationsPerRun || 0), SAFE_ARCHIVE_FUZZ_LIMITS_V1.max_mutations_per_run));
  const safeMaxRuntime = Math.max(100, Math.min(Number(maxRuntimeMs || 0), SAFE_ARCHIVE_FUZZ_LIMITS_V1.max_runtime_ms));
  const replaySeeds = Array.isArray(seeds) ? seeds : [];

  const continuity = includeContinuity
    ? {
        baseline: await runArchiveBaselinePackV1(),
        edgeCase: await runArchiveEdgeCaseHarnessV1({ includeBaseline: false }),
      }
    : null;

  const mutationReports = [];
  for (const seedItem of replaySeeds) {
    const seed = String(seedItem?.seed || `seed_${mutationReports.length + 1}`);
    const baseInput = normalizeArchiveResultV1(seedItem?.input || {});
    const rnd = makePrng(seed);

    for (const mutationType of MUTATION_TYPES) {
      if (mutationReports.length >= safeMaxMutations) break;
      if (Date.now() - startedAt > safeMaxRuntime) break;

      const { mutated, expectedBehavior, detail } = mutateInput(baseInput, mutationType, rnd);
      const normalized = normalizeArchiveResultV1(mutated);
      mutationReports.push(buildMutationReport({
        seed,
        mutationType,
        mutationDetail: detail,
        expectedBehavior,
        normalized,
      }));
    }

    if (mutationReports.length >= safeMaxMutations) break;
    if (Date.now() - startedAt > safeMaxRuntime) break;
  }

  const metrics = archiveFuzzMetrics(mutationReports);

  return {
    layerId: 'safe_archive_fuzz_layer_v1',
    contractVersion: ARCHIVE_RESULT_CONTRACT_VERSION,
    createdAt: new Date().toISOString(),
    safeLimits: {
      max_mutations_per_run: safeMaxMutations,
      max_runtime_ms: safeMaxRuntime,
    },
    replaySeeds: replaySeeds.map((item) => String(item?.seed || 'unknown_seed')),
    continuity,
    mutationReports,
    metrics,
  };
}

export function formatArchiveFuzzCompactSummary(report) {
  const metrics = report?.metrics || {};
  return [
    `layerId=${report?.layerId || 'n/a'}`,
    `contractVersion=${report?.contractVersion || ARCHIVE_RESULT_CONTRACT_VERSION}`,
    `total=${Number(metrics.total || 0)}`,
    `passed=${Number(metrics.byStatus?.passed || 0)}`,
    `failed=${Number(metrics.byStatus?.failed || 0)}`,
    `inconclusive=${Number(metrics.byStatus?.inconclusive || 0)}`,
    `classMatchRate=${Number(metrics.classMatchRate || 0).toFixed(4)}`,
    `mutationTypes=${JSON.stringify(metrics.byMutationType || {})}`,
    `safeLimits=${JSON.stringify(report?.safeLimits || {})}`,
    `authContractVersion=${AUTH_RESULT_CONTRACT_VERSION}`,
    `AUTH_RESULT_V1|present=${(report?.mutationReports || []).filter((x) => x?.checks?.authShapeOk).length}/${(report?.mutationReports || []).length || 0}`,
  ].join(' | ');
}
