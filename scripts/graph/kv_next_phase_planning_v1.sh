#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BASELINE_JSON="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.json"
BASELINE_MD="${ROOT_DIR}/docs/phase33_baseline_freeze_v1.md"
OP_POLICY_JSON="${ROOT_DIR}/docs/phase33_operator_policy_v1.json"
HANDOFF_JSON="${ROOT_DIR}/docs/phase33_handoff_pack_v1.json"
OUT_JSON="${ROOT_DIR}/docs/phase34_next_phase_planning_v1.json"
OUT_MD="${ROOT_DIR}/docs/phase34_next_phase_planning_v1.md"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

artifact_present() { [[ -f "$1" ]] && echo "true" || echo "false"; }

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
obj = json.loads(p.read_text())
cur = obj
for k in ${path@Q}.split('.'):
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

read_json_array_lines() {
  local file="$1"
  local path="$2"
  python - <<PY
import json
from pathlib import Path
p = Path(${file@Q})
if not p.exists():
    raise SystemExit(0)
obj = json.loads(p.read_text())
cur = obj
for k in ${path@Q}.split('.'):
    if isinstance(cur, dict) and k in cur:
        cur = cur[k]
    else:
        raise SystemExit(0)
if isinstance(cur, list):
    for item in cur:
        print(str(item))
PY
}

baseline_json_present="$(artifact_present "${BASELINE_JSON}")"
baseline_md_present="$(artifact_present "${BASELINE_MD}")"
op_policy_present="$(artifact_present "${OP_POLICY_JSON}")"
handoff_present="$(artifact_present "${HANDOFF_JSON}")"

baseline_status="$(read_json_field "${BASELINE_JSON}" "baseline_status")"
baseline_reason="$(read_json_field "${BASELINE_JSON}" "reason")"
baseline_marker="$(read_json_field "${BASELINE_JSON}" "marker")"
op_policy_status="$(read_json_field "${OP_POLICY_JSON}" "status")"
op_policy_reason="$(read_json_field "${OP_POLICY_JSON}" "reason")"
handoff_status="$(read_json_field "${HANDOFF_JSON}" "status")"
handoff_reason="$(read_json_field "${HANDOFF_JSON}" "reason")"

planning_status="planning_ready"
reason="baseline_reference_ready"

if [[ "${baseline_json_present}" != "true" || "${baseline_md_present}" != "true" || -z "${baseline_status}" ]]; then
  planning_status="planning_blocked"
  reason="baseline_reference_missing"
elif [[ "${baseline_status}" != "baseline_frozen" ]]; then
  planning_status="planning_ready_with_notes"
  reason="baseline_reference_ready_with_notes"
fi

marker="KV_NEXT_PHASE_PLANNING_V1|status=${planning_status}|reason=${reason}"

carry_note_1="Preserve frozen baseline continuity from phase33 closure artifacts."
carry_note_2="Respect operator policy chain as primary operational decision channel."
carry_note_3="Any unresolved baseline notes remain active until the next explicit freeze cycle."

mapfile -t baseline_notes < <(read_json_array_lines "${BASELINE_JSON}" "notes_to_carry_forward") || true
if [[ ${#baseline_notes[@]} -gt 0 ]]; then
  carry_note_1="${baseline_notes[0]}"
fi
if [[ ${#baseline_notes[@]} -gt 1 ]]; then
  carry_note_2="${baseline_notes[1]}"
fi
if [[ ${#baseline_notes[@]} -gt 2 ]]; then
  carry_note_3="${baseline_notes[2]}"
fi

cons_1="read-only only"
cons_2="no graph writes"
cons_3="no backfill"
cons_4="no reruns"
cons_5="no UI/runtime/ValidationAgent changes"
cons_6="no feature expansion"

mapfile -t baseline_limits < <(read_json_array_lines "${BASELINE_JSON}" "accepted_limitations") || true
if [[ ${#baseline_limits[@]} -gt 0 ]]; then
  cons_2="${baseline_limits[0]}"
fi
if [[ ${#baseline_limits[@]} -gt 1 ]]; then
  cons_3="${baseline_limits[1]}"
fi
if [[ ${#baseline_limits[@]} -gt 2 ]]; then
  cons_4="${baseline_limits[2]}"
fi
if [[ ${#baseline_limits[@]} -gt 3 ]]; then
  cons_5="${baseline_limits[3]}"
fi

track_1="operator hardening"
track_2="validation agent planning"
track_3="production packaging"
track_4="reporting/analytics hardening"

forbid_1="ломать batch_id canonical path"
forbid_2="возвращать legacy drift"
forbid_3="ломать reconciliation"
forbid_4="ломать handoff / policy chain"
forbid_5="обходить frozen baseline без нового freeze"

entry_1="baseline reference artifacts must exist and be internally consistent."
entry_2="preserved constraints must remain intact during planning and execution."
entry_3="operator policy chain must remain observable and non-bypassed."
entry_4="any baseline notes must be tracked in the next phase plan as explicit work items."

mapfile -t baseline_entries < <(read_json_array_lines "${BASELINE_JSON}" "next_phase_entry_conditions") || true
if [[ ${#baseline_entries[@]} -gt 0 ]]; then
  entry_1="${baseline_entries[0]}"
fi
if [[ ${#baseline_entries[@]} -gt 1 ]]; then
  entry_2="${baseline_entries[1]}"
fi
if [[ ${#baseline_entries[@]} -gt 2 ]]; then
  entry_3="${baseline_entries[2]}"
fi
if [[ ${#baseline_entries[@]} -gt 3 ]]; then
  entry_4="${baseline_entries[3]}"
fi

recommended_first_step="Draft next major track kickoff checklist from this planning artifact and validate constraints with operator policy owner."
if [[ "${planning_status}" == "planning_blocked" ]]; then
  recommended_first_step="Restore baseline reference artifacts first, then rerun planning generation before any next-phase kickoff."
elif [[ "${planning_status}" == "planning_ready_with_notes" ]]; then
  recommended_first_step="Start with note-closure planning: convert carry-forward notes into explicit prioritized tasks before expansion."
fi

cat > "${OUT_JSON}" <<JSON
{
  "version": "phase34_next_phase_planning_v1",
  "generated_at": "${NOW_UTC}",
  "status": "${planning_status}",
  "reason": "${reason}",
  "marker": "${marker}",
  "baseline_reference": {
    "phase33_baseline_freeze_json": {"path": "docs/phase33_baseline_freeze_v1.json", "present": ${baseline_json_present}},
    "phase33_baseline_freeze_md": {"path": "docs/phase33_baseline_freeze_v1.md", "present": ${baseline_md_present}},
    "baseline_status": "${baseline_status}",
    "baseline_reason": "${baseline_reason}",
    "baseline_marker": "${baseline_marker}"
  },
  "carry_forward_notes": [
    "${carry_note_1}",
    "${carry_note_2}",
    "${carry_note_3}"
  ],
  "preserved_constraints": [
    "${cons_1}",
    "${cons_2}",
    "${cons_3}",
    "${cons_4}",
    "${cons_5}",
    "${cons_6}"
  ],
  "allowed_next_tracks": [
    "${track_1}",
    "${track_2}",
    "${track_3}",
    "${track_4}"
  ],
  "forbidden_regressions": [
    "${forbid_1}",
    "${forbid_2}",
    "${forbid_3}",
    "${forbid_4}",
    "${forbid_5}"
  ],
  "entry_requirements": [
    "${entry_1}",
    "${entry_2}",
    "${entry_3}",
    "${entry_4}"
  ],
  "recommended_first_step": "${recommended_first_step}",
  "context_snapshot": {
    "operator_policy_artifact_present": ${op_policy_present},
    "operator_policy_status": "${op_policy_status}",
    "operator_policy_reason": "${op_policy_reason}",
    "handoff_artifact_present": ${handoff_present},
    "handoff_status": "${handoff_status}",
    "handoff_reason": "${handoff_reason}"
  }
}
JSON

cat > "${OUT_MD}" <<MD
# Phase 34 Next Phase Planning v1

Generated at: ${NOW_UTC}

Marker: \`${marker}\`

- status: **${planning_status}**
- reason: **${reason}**

## baseline_reference
- phase33_baseline_freeze_json: ${baseline_json_present}
- phase33_baseline_freeze_md: ${baseline_md_present}
- baseline_status: ${baseline_status}
- baseline_reason: ${baseline_reason}
- baseline_marker: ${baseline_marker}

## carry_forward_notes
- ${carry_note_1}
- ${carry_note_2}
- ${carry_note_3}

## preserved_constraints
- ${cons_1}
- ${cons_2}
- ${cons_3}
- ${cons_4}
- ${cons_5}
- ${cons_6}

## allowed_next_tracks
- ${track_1}
- ${track_2}
- ${track_3}
- ${track_4}

## forbidden_regressions
- ${forbid_1}
- ${forbid_2}
- ${forbid_3}
- ${forbid_4}
- ${forbid_5}

## entry_requirements
- ${entry_1}
- ${entry_2}
- ${entry_3}
- ${entry_4}

## recommended_first_step
${recommended_first_step}
MD

echo "${marker}"
