// Phase 32.1 — Knowledge Vault v1 shadow bootstrap schema
// Shadow-only foundation: no runtime decision coupling.

CREATE CONSTRAINT device_id_unique IF NOT EXISTS
FOR (d:Device)
REQUIRE d.device_id IS UNIQUE;

CREATE CONSTRAINT service_key_unique IF NOT EXISTS
FOR (s:Service)
REQUIRE s.service_key IS UNIQUE;

CREATE CONSTRAINT vendor_name_unique IF NOT EXISTS
FOR (v:Vendor)
REQUIRE v.name IS UNIQUE;

CREATE CONSTRAINT firmware_key_unique IF NOT EXISTS
FOR (f:Firmware)
REQUIRE f.firmware_key IS UNIQUE;

CREATE CONSTRAINT credential_profile_key_unique IF NOT EXISTS
FOR (c:CredentialProfile)
REQUIRE c.profile_key IS UNIQUE;

CREATE CONSTRAINT run_id_unique IF NOT EXISTS
FOR (r:Run)
REQUIRE r.run_id IS UNIQUE;

CREATE CONSTRAINT capability_key_unique IF NOT EXISTS
FOR (c:Capability)
REQUIRE c.capability_key IS UNIQUE;

CREATE CONSTRAINT finding_id_unique IF NOT EXISTS
FOR (f:Finding)
REQUIRE f.finding_id IS UNIQUE;

CREATE CONSTRAINT evidence_ref_unique IF NOT EXISTS
FOR (e:Evidence)
REQUIRE e.evidence_ref IS UNIQUE;

CREATE CONSTRAINT profile_pack_id_unique IF NOT EXISTS
FOR (p:ProfilePack)
REQUIRE p.pack_id IS UNIQUE;

CREATE CONSTRAINT environment_key_unique IF NOT EXISTS
FOR (e:Environment)
REQUIRE e.environment_key IS UNIQUE;

CREATE CONSTRAINT review_decision_id_unique IF NOT EXISTS
FOR (r:ReviewDecision)
REQUIRE r.decision_id IS UNIQUE;

CREATE CONSTRAINT validation_path_key_unique IF NOT EXISTS
FOR (v:ValidationPath)
REQUIRE v.path_key IS UNIQUE;

CREATE INDEX finding_severity_idx IF NOT EXISTS
FOR (f:Finding)
ON (f.severity);

CREATE INDEX run_created_at_idx IF NOT EXISTS
FOR (r:Run)
ON (r.created_at);

CREATE INDEX evidence_source_idx IF NOT EXISTS
FOR (e:Evidence)
ON (e.source);

// Required relationship semantics (documented, ready for Phase 32.2 dual-write):
// (Device)-[:HAS_SERVICE]->(Service)
// (Device)-[:FROM_VENDOR]->(Vendor)
// (Device)-[:RUNS_FIRMWARE]->(Firmware)
// (Device)-[:HAS_CREDENTIAL_PROFILE]->(CredentialProfile)
// (Run)-[:USED_CAPABILITY]->(Capability)
// (Run)-[:PRODUCED_FINDING]->(Finding)
// (Finding)-[:SUPPORTED_BY]->(Evidence)
// (ProfilePack)-[:CONTAINS_CASE]->(Finding)
// (Run)-[:IN_ENVIRONMENT]->(Environment)
// (Finding)-[:REVIEW_DECISION]->(ReviewDecision)
// (Run)-[:USED_PATH]->(ValidationPath)
// (Run)-[:OBSERVED_CREDENTIAL_PROFILE]->(CredentialProfile)
