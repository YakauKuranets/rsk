#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

read_json_field() {
  local file="$1"
  local path="$2"
  python - <<PY
import json
from pathlib import Path
p = Path(${file@Q})
if not p.exists():
    print("")
    raise SystemExit(0)
try:
    obj = json.loads(p.read_text())
except Exception:
    print("invalid_json")
    raise SystemExit(0)
cur = obj
for k in ${path@Q}.split("."):
    if isinstance(cur, dict) and k in cur:
        cur = cur[k]
    else:
        print("")
        raise SystemExit(0)
if isinstance(cur, bool):
    print("true" if cur else "false")
elif cur is None:
    print("")
else:
    print(str(cur))
PY
}

check_artifact() {
  local label="$1"
  local file="$2"
  local status_path="$3"
  local reason_path="$4"
  if [[ ! -f "${file}" ]]; then
    echo "${label}: MISSING"
    return
  fi
  local s r
  s="$(read_json_field "${file}" "${status_path}")"
  r="$(read_json_field "${file}" "${reason_path}")"
  echo "${label}: status=${s:-unknown} reason=${r:-none}"
}

check_artifact "phase32_graph_env_readiness" "${ROOT_DIR}/docs/phase32_graph_env_readiness_v1.json" "status" "reason"
check_artifact "phase32_exit_remediation" "${ROOT_DIR}/docs/phase32_exit_remediation_report_v1.json" "overall_status" "graph_env_readiness.reason"
check_artifact "phase33_shadow_validation" "${ROOT_DIR}/docs/phase33_shadow_validation_v1.json" "status" "reason"
check_artifact "phase33_shadow_batch_field_audit" "${ROOT_DIR}/docs/phase33_shadow_batch_field_audit_v1.json" "status" "reason"
check_artifact "phase33_legacy_drift_governance" "${ROOT_DIR}/docs/phase33_legacy_drift_governance_v1.json" "status" "reason"
check_artifact "phase33_operator_readiness" "${ROOT_DIR}/docs/phase33_operator_readiness_v1.json" "status" "reason"
check_artifact "phase33_operator_policy" "${ROOT_DIR}/docs/phase33_operator_policy_v1.json" "status" "reason"
check_artifact "phase33_handoff_pack" "${ROOT_DIR}/docs/phase33_handoff_pack_v1.json" "status" "reason"
check_artifact "phase33_baseline_freeze" "${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json" "baseline_status" "reason"
check_artifact "phase34_operator_backlog_triage" "${ROOT_DIR}/docs/phase34_operator_backlog_triage_v1.json" "status" "reason"
