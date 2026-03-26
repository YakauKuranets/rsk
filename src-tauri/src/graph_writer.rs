use crate::capability_adapter::{
    CapabilityName, CapabilityRequest, CapabilityResult, CapabilityResultData, ProbeStreamInput,
};
use crate::core_types::WorkflowMode;
use crate::{push_runtime_log, LogState};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tokio::process::Command;

#[derive(Debug, Clone)]
struct GraphShadowConfig {
    enabled: bool,
    bolt_url: String,
    user: String,
    password: String,
    database: String,
}

#[derive(Debug, Clone, Serialize)]
struct DualWriteProjection {
    run_id: String,
    capability_key: String,
    mode_key: String,
    target_ref: String,
    finding_key: String,
    finding_severity: String,
    finding_summary: String,
    evidence_refs_hashed: Vec<String>,
    observed_profile_key: Option<String>,
    service_key: Option<String>,
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn escape_cypher(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', " ")
        .replace('\r', " ")
}

fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn graph_shadow_config() -> GraphShadowConfig {
    let mode = env::var("KV_SHADOW_MODE")
        .unwrap_or_default()
        .to_lowercase();
    let enabled_flag = env::var("KV_DUAL_WRITE_ENABLED")
        .unwrap_or_default()
        .to_lowercase();
    let enabled = enabled_flag == "true" || mode == "true";

    let user = env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = env::var("NEO4J_PASSWORD").unwrap_or_default();
    let bolt_port = env::var("NEO4J_BOLT_PORT").unwrap_or_else(|_| "7687".to_string());
    let bolt_url =
        env::var("NEO4J_BOLT_URL").unwrap_or_else(|_| format!("bolt://localhost:{}", bolt_port));
    let database = env::var("NEO4J_DATABASE").unwrap_or_else(|_| "neo4j".to_string());

    GraphShadowConfig {
        enabled,
        bolt_url,
        user,
        password,
        database,
    }
}

fn extract_target_ref(req: &CapabilityRequest) -> String {
    if let Some(x) = &req.probe_stream {
        return x.target_id.trim().to_string();
    }
    if let Some(x) = &req.verify_session_cookie_flags {
        return x.ip_or_url.trim().to_string();
    }
    if let Some(x) = &req.search_archive_records {
        return x.camera_ip.trim().to_string();
    }
    "unknown_target".to_string()
}

fn projection_from(req: &CapabilityRequest, result: &CapabilityResult) -> DualWriteProjection {
    let run_id = format!("kv_run_{}", now_millis());
    let capability_key = format!("{:?}", req.capability).to_lowercase();
    let mode_key = format!("{:?}", req.mode).to_lowercase();
    let target_ref = extract_target_ref(req);

    match &result.data {
        Some(CapabilityResultData::ProbeStream(out)) => {
            let finding_key = if out.alive {
                format!("finding_stream_alive_{}", now_millis())
            } else {
                format!("finding_stream_unavailable_{}", now_millis())
            };
            let severity = if out.alive { "low" } else { "high" }.to_string();
            let summary = if out.alive {
                "stream probe reports alive".to_string()
            } else {
                "stream probe reports unavailable".to_string()
            };
            let evidence_refs_hashed = out
                .evidence_refs
                .iter()
                .map(|x| format!("sha256:{}", sha256_hex(x)))
                .collect::<Vec<_>>();

            DualWriteProjection {
                run_id,
                capability_key,
                mode_key,
                target_ref,
                finding_key,
                finding_severity: severity,
                finding_summary: summary,
                evidence_refs_hashed,
                observed_profile_key: None,
                service_key: Some("service:stream_probe".to_string()),
            }
        }
        Some(CapabilityResultData::VerifySessionCookieFlags(out)) => {
            let finding_key = if out.secure {
                format!("finding_session_secure_{}", now_millis())
            } else {
                format!("finding_session_insecure_{}", now_millis())
            };
            let severity = if out.secure { "low" } else { "high" }.to_string();
            let summary = if out.secure {
                "session cookie flags look secure".to_string()
            } else {
                format!("session cookie issues count={}", out.issues.len())
            };
            let evidence_refs_hashed = out
                .evidence_refs
                .iter()
                .map(|x| format!("sha256:{}", sha256_hex(x)))
                .collect::<Vec<_>>();

            DualWriteProjection {
                run_id,
                capability_key,
                mode_key,
                target_ref,
                finding_key,
                finding_severity: severity,
                finding_summary: summary,
                evidence_refs_hashed,
                observed_profile_key: Some("credential_profile:session_cookie_flags".to_string()),
                service_key: Some("service:session_cookie_flags".to_string()),
            }
        }
        Some(CapabilityResultData::SearchArchiveRecords(out)) => {
            let finding_key = format!("finding_archive_records_{}", now_millis());
            let severity = if out.records_found > 0 {
                "medium"
            } else {
                "low"
            }
            .to_string();
            let summary = format!(
                "archive records_found={} protocol={}",
                out.records_found, out.protocol_used
            );
            let evidence_refs_hashed = out
                .evidence_refs
                .iter()
                .map(|x| format!("sha256:{}", sha256_hex(x)))
                .collect::<Vec<_>>();

            DualWriteProjection {
                run_id,
                capability_key,
                mode_key,
                target_ref,
                finding_key,
                finding_severity: severity,
                finding_summary: summary,
                evidence_refs_hashed,
                observed_profile_key: None,
                service_key: Some(format!(
                    "service:archive:{}",
                    out.protocol_used.to_lowercase()
                )),
            }
        }
        None => DualWriteProjection {
            run_id,
            capability_key,
            mode_key,
            target_ref,
            finding_key: format!("finding_capability_error_{}", now_millis()),
            finding_severity: "medium".to_string(),
            finding_summary: result
                .error
                .as_ref()
                .map(|e| format!("capability error code={}", e.code))
                .unwrap_or_else(|| "capability error".to_string()),
            evidence_refs_hashed: Vec::new(),
            observed_profile_key: None,
            service_key: None,
        },
    }
}

async fn run_cypher(config: &GraphShadowConfig, query: &str) -> Result<String, String> {
    let output = Command::new("cypher-shell")
        .arg("-a")
        .arg(&config.bolt_url)
        .arg("-u")
        .arg(&config.user)
        .arg("-p")
        .arg(&config.password)
        .arg("-d")
        .arg(&config.database)
        .arg(query)
        .output()
        .await
        .map_err(|e| format!("cypher-shell spawn error: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("cypher-shell failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn write_projection(
    config: &GraphShadowConfig,
    p: &DualWriteProjection,
) -> Result<(), String> {
    let run_id = escape_cypher(&p.run_id);
    let cap = escape_cypher(&p.capability_key);
    let mode = escape_cypher(&p.mode_key);
    let target = escape_cypher(&p.target_ref);
    let finding = escape_cypher(&p.finding_key);
    let severity = escape_cypher(&p.finding_severity);
    let summary = escape_cypher(&p.finding_summary);

    let base = format!(
        "MERGE (d:Device {{device_id:'{target}'}}) \
         MERGE (r:Run {{run_id:'{run_id}'}}) \
         SET r.created_at=timestamp(), r.shadow_mode=true \
         MERGE (c:Capability {{capability_key:'{cap}'}}) \
         MERGE (vp:ValidationPath {{path_key:'{mode}'}}) \
         MERGE (f:Finding {{finding_id:'{finding}'}}) \
         SET f.severity='{severity}', f.summary='{summary}', f.shadow_mode=true \
         MERGE (r)-[:USED_CAPABILITY]->(c) \
         MERGE (r)-[:USED_PATH]->(vp) \
         MERGE (r)-[:PRODUCED_FINDING]->(f)"
    );
    run_cypher(config, &base).await?;

    if let Some(service_key) = &p.service_key {
        let q = format!(
            "MERGE (d:Device {{device_id:'{}'}}) MERGE (s:Service {{service_key:'{}'}}) MERGE (d)-[:HAS_SERVICE]->(s)",
            target,
            escape_cypher(service_key)
        );
        run_cypher(config, &q).await?;
    }

    if let Some(profile_key) = &p.observed_profile_key {
        let q = format!(
            "MERGE (r:Run {{run_id:'{}'}}) MERGE (cp:CredentialProfile {{profile_key:'{}'}}) MERGE (r)-[:OBSERVED_CREDENTIAL_PROFILE]->(cp)",
            run_id,
            escape_cypher(profile_key)
        );
        run_cypher(config, &q).await?;
    }

    for (idx, ev) in p.evidence_refs_hashed.iter().enumerate() {
        let ev_ref = escape_cypher(&format!("{}:{}", p.run_id, idx));
        let ev_hash = escape_cypher(ev);
        let q = format!(
            "MERGE (f:Finding {{finding_id:'{}'}}) \
             MERGE (e:Evidence {{evidence_ref:'{}'}}) \
             SET e.source='runtime_ref', e.hash='{}' \
             MERGE (f)-[:SUPPORTED_BY]->(e)",
            finding, ev_ref, ev_hash
        );
        run_cypher(config, &q).await?;
    }

    Ok(())
}

pub fn enqueue_capability_dual_write(
    req: &CapabilityRequest,
    result: &CapabilityResult,
    log_state: &State<'_, LogState>,
) {
    let cfg = graph_shadow_config();
    if !cfg.enabled {
        push_runtime_log(log_state, "KV_DUAL_WRITE_V1|status=skipped|reason=disabled");
        return;
    }
    if cfg.password.trim().is_empty() {
        push_runtime_log(
            log_state,
            "KV_DUAL_WRITE_V1|status=skipped|reason=missing_password",
        );
        return;
    }

    let projection = projection_from(req, result);
    let cfg_clone = cfg.clone();
    push_runtime_log(
        log_state,
        format!(
            "KV_DUAL_WRITE_V1|status=queued|runId={}|capability={}",
            projection.run_id, projection.capability_key
        ),
    );

    tokio::spawn(async move {
        if let Err(e) = write_projection(&cfg_clone, &projection).await {
            eprintln!(
                "KV_DUAL_WRITE_V1|status=error|runId={}|capability={}|reason={}",
                projection.run_id,
                projection.capability_key,
                e.replace('|', "/")
            );
        }
    });
}

#[tauri::command]
pub async fn kv_dual_write_diagnostic(log_state: State<'_, LogState>) -> Result<String, String> {
    let req = CapabilityRequest {
        capability: CapabilityName::ProbeStream,
        mode: WorkflowMode::DiscoveryMode,
        probe_stream: Some(ProbeStreamInput {
            target_id: "kv_diag_target".to_string(),
        }),
        search_archive_records: None,
        verify_session_cookie_flags: None,
    };

    let result = CapabilityResult {
        ok: true,
        capability: CapabilityName::ProbeStream,
        mode: WorkflowMode::DiscoveryMode,
        data: Some(CapabilityResultData::ProbeStream(
            crate::capability_adapter::ProbeStreamOutput {
                target_id: "kv_diag_target".to_string(),
                alive: false,
                allowed_modes: vec![WorkflowMode::DiscoveryMode],
                evidence_refs: vec!["diag:stream_state:kv_diag_target".to_string()],
            },
        )),
        error: None,
    };

    enqueue_capability_dual_write(&req, &result, &log_state);
    Ok("KV_DUAL_WRITE_V1|status=queued|diagnostic=true".to_string())
}
