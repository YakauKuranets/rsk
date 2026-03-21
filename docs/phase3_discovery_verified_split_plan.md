# Phase 3 — Discovery / Verified Split (safe migration plan)

## 1) Scope and constraints

- No big-bang rewrite.
- No agent-layer changes in this phase.
- Reuse capability foundation that already encodes workflow modes (`discovery_mode`, `verified_mode`, `analysis_mode`).

## 2) What is currently in place

### Backend

- Core workflow enums and card structs already exist in `core_types.rs`:
  - `WorkflowMode` with `DiscoveryMode` and `VerifiedMode`.
  - `DiscoveryCard`, `VerifiedCard`, `PromotedCard`.
  - `DiscoveryStatus`, `VerificationStatus`.
- Current target persistence layer stores an opaque JSON payload (`save_target(target_id, payload)`), without entity-level validation for discovery vs verified cards.
- Capability adapter already constrains capabilities by mode:
  - `ProbeStream`: discovery + verified.
  - `SearchArchiveRecords`: verified + analysis.

### Frontend

- Target create form always captures `host` + `login` + `password`, so discovery-only targets and verified targets are entered through one form and one list.
- Main orchestration (`App.jsx`) uses a shared `targets` array for both stream inference and credential-dependent archive actions.
- Stream start path for local targets probes RTSP path using credentials, then starts stream in the same flow.
- Archive actions (ISAPI/ONVIF/export/download) are also invoked from same target card context.

## 3) Where discovery and verified are mixed

1. **Entity input layer**: one `TargetForm` for both credential-less and credentialed entities.
2. **Entity storage layer**: one opaque `payload` JSON in vault for all card kinds.
3. **Execution layer**:
   - stream inference and verified stream checks share same per-target UX context;
   - archive search/export actions are reachable from same card list as discovery cards.
4. **Status layer**: UI mostly relies on ad-hoc runtime labels and task statuses (`running/done/error`) rather than explicit discovery/verified/promotion state machine per card.

## 4) Target card model (minimal)

### DiscoveryCard (target identity, no creds)

**Required fields**
- `cardId`
- `ipOrHost`
- `createdAt`
- `discoveryStatus`

**Optional fields**
- `address`
- `siteLabel`
- `suspectedVendor`
- `streamCapability` (`unknown|inferred|confirmed|not_supported`)
- `archiveCapability` (`unknown|inferred|confirmed|not_supported`)
- `evidenceRefs[]`

**Allowed actions**
- probe stream capability (shadow/diagnostic + explicit)
- passive profiling / geocode / labeling
- create promotion candidate

**Expected outcomes**
- refined capability inference
- explicit `auth_required` signal
- promotion candidate creation

### VerifiedCard (identity + validated creds)

**Required fields**
- `cardId`
- `ipOrHost`
- `credentialRef` (vault reference id, not raw password in UI state)
- `verifiedStatus`
- `verifiedAt` (nullable until first success)

**Optional fields**
- `vendorHint`
- `streamAuthMode`
- `archiveAuthMode`
- `evidenceRefs[]`

**Allowed actions**
- verified stream start/restart
- archive search (ISAPI/ONVIF/unified)
- archive export/download

**Expected outcomes**
- credential validity verdict
- protocol/auth mode resolution
- downloadable archive artifacts

### PromotedCard (bridge object)

**Required fields**
- `promotionId`
- `sourceDiscoveryCardId`
- `targetVerifiedCardId` (nullable until created)
- `promotionStatus`
- `promotionReason`
- `confidence`

**Optional fields**
- `requiredFields[]`
- `blockedBy[]`
- `approvedBy` / `approvedAt`

**Allowed actions**
- request credentials
- map credentials to source discovery card
- create verified card

**Expected outcomes**
- deterministic handoff from discovery to verified
- audit trail for why promotion happened

## 5) Target status model split

### Discovery statuses
- `new`
- `profiling`
- `auth_required`
- `promoted`
- `completed`
- `failed`

### Verified statuses
- `pending`
- `in_progress`
- `verified`
- `inconclusive`
- `failed`

### Promotion statuses
- `candidate`
- `awaiting_credentials`
- `ready_to_promote`
- `promoted`
- `rejected`

## 6) Migration table (safe, incremental)

| Current entity/field | Target entity/field | Adapt now (no big-bang) | Keep legacy for now |
|---|---|---|---|
| `targets[]` mixed objects in frontend | `discoveryCards[]`, `verifiedCards[]`, `promotions[]` view-model slices | Add adapter selectors that derive split views from existing `targets[]` | Keep single persisted payload format in vault initially |
| `TargetForm` (`host/login/password` always visible) | discovery-first form + optional promotion credentials step | Add mode toggle: `Discovery` (host only) / `Verified` (host+creds) | Keep current save API (`save_target`) |
| `save_target(targetId, payload)` opaque JSON | explicit card envelope `{kind, version, data}` | Start writing new records with envelope; read old records via fallback adapter | Continue reading legacy payloads unchanged |
| Stream actions from one card list | discovery actions vs verified actions | Gate action buttons by derived card kind/status | Keep command handlers unchanged |
| Archive search/export reachable from any local target | verified-only action set | UI guard: show archive actions only for `VerifiedCard` | Keep backend commands unchanged |
| Ad-hoc task/runtime statuses | per-card workflow status fields | Add local status projection layer in frontend | Keep existing runtime log/task statuses as operational telemetry |

## 7) What to rewrite first (minimal practical sequence)

1. Add **frontend card-kind adapter** (`deriveCardKind(target)` + selectors) with zero backend changes.
2. Gate action buttons in `TargetList` and stream/archive panels by card kind + status.
3. Introduce payload envelope versioning for newly saved targets; keep legacy read path.
4. Add promotion UI micro-flow (create promoted candidate, then verified card creation).

## 8) What not to touch yet

- Agent layer (recon/scan/risk/auto pipeline).
- Capability command internals (except mode-aware gating already present).
- Archive backend command implementations.
- Streaming backend process manager.

## 9) Risks and mitigations

- **Risk**: UI regression from hidden actions.  
  **Mitigation**: feature-flag split gating and keep legacy fallback actions behind debug toggle.
- **Risk**: legacy target payload heterogeneity.  
  **Mitigation**: tolerant adapter with schema-version detection.
- **Risk**: credential leakage in client state.  
  **Mitigation**: move toward `credentialRef` envelope while keeping existing encrypted vault storage path.

## 10) Dependencies

- Existing capability foundation (`execute_capability` mode checks).
- Existing vault persistence APIs (`save_target/read_target/get_all_targets`).
- Existing stream/archive commands; no behavior rewrite required for step 1.

## 11) Definition of done for this planning PR

- Discovery/Verified/Promotion model documented.
- Explicit status split and action matrix documented.
- Migration table with “adapt now vs legacy” documented.
- First practical migration step identified and scoped to frontend adapter + UI gating.
