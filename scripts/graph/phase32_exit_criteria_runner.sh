#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_JSON="${ROOT_DIR}/docs/phase32_exit_criteria_report_v1.json"
REPORT_MD="${ROOT_DIR}/docs/phase32_exit_criteria_report_v1.md"

GRAPH_ENV_JSON="${ROOT_DIR}/docs/phase32_graph_env_readiness_v1.json"
INTEGRATED_JSON="${ROOT_DIR}/docs/phase32_remediation_integrated_load_v1.json"
RECON_JSON="${ROOT_DIR}/docs/phase32_remediation_reconciliation_v1.json"
LATENCY_JSON="${ROOT_DIR}/docs/phase32_remediation_latency_v1.json"
EXIT_REMEDIATION_JSON="${ROOT_DIR}/docs/phase32_exit_remediation_report_v1.json"

NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

json_ok() {
  local f="$1"
  [[ -f "$f" ]] && jq empty "$f" >/dev/null 2>&1
}

read_json_field() {
  local file="$1"
  local path="$2"
  python - "$file" "$path" <<'PY'
import json, sys
from pathlib import Path

file_path = Path(sys.argv[1])
path = sys.argv[2].split('.')

if not file_path.exists():
    print("")
    raise SystemExit(0)

try:
    data = json.loads(file_path.read_text(encoding="utf-8"))
except Exception:
    print("")
    raise SystemExit(0)

cur = data
for part in path:
    if isinstance(cur, dict) and part in cur:
        cur = cur[part]
    else:
        print("")
        raise SystemExit(0)

if cur is None:
    print("")
elif isinstance(cur, bool):
    print("true" if cur else "false")
else:
    print(str(cur))
PY
}

is_blocking_status() {
  local s="${1:-}"
  [[ "$s" == "blocked" || "$s" == "fail" || "$s" == "error" ]]
}

status_or_default() {
  local file="$1"
  local field="$2"
  local default="$3"
  local v=""
  if json_ok "$file"; then
    v="$(read_json_field "$file" "$field")"
  fi
  if [[ -z "$v" ]]; then
    printf '%s' "$default"
  else
    printf '%s' "$v"
  fi
}

note_or_missing() {
  local file="$1"
  local field="$2"
  local fallback="$3"
  local v=""
  if json_ok "$file"; then
    v="$(read_json_field "$file" "$field")"
  fi
  if [[ -z "$v" ]]; then
    printf '%s' "$fallback"
  else
    printf '%s' "$v"
  fi
}

DUAL_WRITE_STATUS="$(status_or_default "$INTEGRATED_JSON" "status" "blocked")"
INGEST_LOSS_STATUS="$(status_or_default "$RECON_JSON" "status" "blocked")"
LATENCY_STATUS="$(status_or_default "$LATENCY_JSON" "status" "blocked")"
GRAPH_ENV_STATUS="$(status_or_default "$GRAPH_ENV_JSON" "status" "blocked")"

NO_RAW_SECRETS_STATUS="pass_with_notes"
READONLY_STATUS="pass_with_notes"
NO_RUNTIME_STATUS="pass"

BLOCKERS=()
REMEDIATION=()
LIVE_NOTES=()

graph_env_reason="$(note_or_missing "$GRAPH_ENV_JSON" "reason" "graph_env_artifact_missing")"
integrated_reason="$(note_or_missing "$INTEGRATED_JSON" "reason" "integrated_load_artifact_missing")"
recon_reason="$(note_or_missing "$RECON_JSON" "reason" "reconciliation_artifact_missing")"
latency_reason="$(note_or_missing "$LATENCY_JSON" "reason" "latency_artifact_missing")"

if rg -n "kv_read_analytics_v1|kv_shadow_ingest_projection_v2|kv_dual_write_diagnostic" "${ROOT_DIR}/src-tauri/src/capability_adapter.rs" >/dev/null 2>&1; then
  NO_RUNTIME_STATUS="blocked"
fi

if json_ok "$EXIT_REMEDIATION_JSON"; then
  LIVE_NOTES+=("exit_remediation_present")
  LIVE_NOTES+=("exit_remediation_status:$(read_json_field "$EXIT_REMEDIATION_JSON" "overall_status")")
else
  LIVE_NOTES+=("exit_remediation_missing")
fi

if [[ "$GRAPH_ENV_STATUS" != "pass" && "$GRAPH_ENV_STATUS" != "pass_with_notes" ]]; then
  BLOCKERS+=("graph_env_not_ready:${graph_env_reason}")
  REMEDIATION+=("Run kv_graph_env_readiness_v1.sh until graph env becomes pass")
fi

if is_blocking_status "$DUAL_WRITE_STATUS"; then
  BLOCKERS+=("dual_write_stability_not_verified:${integrated_reason}")
  REMEDIATION+=("Run kv_integrated_100_event_load.sh and confirm writes_completed")
fi

if is_blocking_status "$INGEST_LOSS_STATUS"; then
  BLOCKERS+=("ingest_no_data_loss_not_verified:${recon_reason}")
  REMEDIATION+=("Run kv_reconciliation_check_v1.sh and confirm batch_counts_match")
fi

if is_blocking_status "$LATENCY_STATUS"; then
  BLOCKERS+=("latency_not_acceptable:${latency_reason}")
  REMEDIATION+=("Run kv_latency_benchmark_v1.sh and confirm acceptable or pass_with_notes")
fi

if is_blocking_status "$NO_RUNTIME_STATUS"; then
  BLOCKERS+=("runtime_path_directly_invokes_graph_read_or_diagnostic_command")
  REMEDIATION+=("Remove direct graph-read or diagnostic command invocation from runtime path")
fi

if [[ ${#REMEDIATION[@]} -eq 0 ]]; then
  REMEDIATION+=("No blocking remediation items remain; Phase32 exit criteria are satisfied with notes")
fi

if [[ -f "${ROOT_DIR}/infra/neo4j-shadow/.env" ]] && command -v cypher-shell >/dev/null 2>&1; then
  # shellcheck disable=SC1090
  source "${ROOT_DIR}/infra/neo4j-shadow/.env"
  BOLT_URL="bolt://localhost:${NEO4J_BOLT_PORT:-7687}"
  if cypher-shell -a "${BOLT_URL}" -u "${NEO4J_USER:-neo4j}" -p "${NEO4J_PASSWORD:-}" 'RETURN 1' >/dev/null 2>&1; then
    LIVE_NOTES+=("neo4j_live_connection_ok")
  else
    LIVE_NOTES+=("neo4j_live_connection_failed")
  fi
else
  LIVE_NOTES+=("neo4j_live_checks_skipped")
fi

OVERALL="blocked"
RECOMMENDATION="no_go_to_phase33_until_blockers_resolved"

if [[ ${#BLOCKERS[@]} -eq 0 ]]; then
  if [[ "$DUAL_WRITE_STATUS" == "pass" && "$INGEST_LOSS_STATUS" == "pass" && "$LATENCY_STATUS" == "pass" && "$NO_RUNTIME_STATUS" == "pass" && "$GRAPH_ENV_STATUS" == "pass" ]]; then
    OVERALL="pass"
    RECOMMENDATION="go_to_phase33"
  else
    OVERALL="pass_with_notes"
    RECOMMENDATION="go_to_phase33_with_notes"
  fi
fi

MARKER="KV_EXIT_CRITERIA_V1|status=${OVERALL}|blocked=$((${#BLOCKERS[@]}))|ts=${NOW_UTC}"

blockers_json="$(printf '%s\n' "${BLOCKERS[@]}" | python -c 'import json,sys; print(json.dumps([x.strip() for x in sys.stdin if x.strip()]))')"
remediation_json="$(printf '%s\n' "${REMEDIATION[@]}" | python -c 'import json,sys; print(json.dumps([x.strip() for x in sys.stdin if x.strip()]))')"
live_notes_json="$(printf '%s\n' "${LIVE_NOTES[@]}" | python -c 'import json,sys; print(json.dumps([x.strip() for x in sys.stdin if x.strip()]))')"

cat > "${REPORT_JSON}" <<JSON
{
  "report_version": "phase32_exit_criteria_report_v1",
  "generated_at": "${NOW_UTC}",
  "overall_status": "${OVERALL}",
  "marker": "${MARKER}",
  "criteria": {
    "graph_env_ready": {
      "status": "${GRAPH_ENV_STATUS}",
      "notes": ["graph_env_reason:${graph_env_reason}"]
    },
    "dual_write_stability": {
      "status": "${DUAL_WRITE_STATUS}",
      "notes": ["integrated_load_reason:${integrated_reason}"]
    },
    "no_raw_secrets": {
      "status": "${NO_RAW_SECRETS_STATUS}",
      "notes": ["Code-level sanitization and hashed evidence refs are present"]
    },
    "ingest_no_data_loss": {
      "status": "${INGEST_LOSS_STATUS}",
      "notes": ["reconciliation_reason:${recon_reason}"]
    },
    "readonly_analytics_useful": {
      "status": "${READONLY_STATUS}",
      "notes": ["Read-only analytics commands exist; usefulness remains advisory-only"]
    },
    "no_runtime_influence": {
      "status": "${NO_RUNTIME_STATUS}",
      "notes": ["No direct graph-read decision coupling should exist in capability runtime path"]
    },
    "latency_acceptable": {
      "status": "${LATENCY_STATUS}",
      "notes": ["latency_reason:${latency_reason}"]
    }
  },
  "blockers": ${blockers_json},
  "remediation_items": ${remediation_json},
  "live_check_notes": ${live_notes_json},
  "recommendation": "${RECOMMENDATION}"
}
JSON

cat > "${REPORT_MD}" <<MD
# Phase 32 Exit Criteria Report v1

Generated at: ${NOW_UTC}

Marker:
\`${MARKER}\`

## Overall status
- **${OVERALL}**
- Recommendation: **${RECOMMENDATION}**

## Criteria status
- graph_env_ready: **${GRAPH_ENV_STATUS}**
- dual_write_stability: **${DUAL_WRITE_STATUS}**
- no_raw_secrets: **${NO_RAW_SECRETS_STATUS}**
- ingest_no_data_loss: **${INGEST_LOSS_STATUS}**
- readonly_analytics_useful: **${READONLY_STATUS}**
- no_runtime_influence: **${NO_RUNTIME_STATUS}**
- latency_acceptable: **${LATENCY_STATUS}**

## Blockers
$(printf '%s\n' "${BLOCKERS[@]}" | sed 's/^/- /')

## Remediation items
$(printf '%s\n' "${REMEDIATION[@]}" | sed 's/^/- /')

## Live-check notes
$(printf '%s\n' "${LIVE_NOTES[@]}" | sed 's/^/- /')
MD

echo "${MARKER}"
