use crate::{push_runtime_log, LogState};
use serde::Serialize;
use std::collections::HashMap;
use std::env;
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
#[serde(rename_all = "camelCase")]
pub struct GraphReadAnalyticsV1 {
    pub analytics_version: String,
    pub mode: String,
    pub status: String,
    pub marker: String,
    pub summary: HashMap<String, String>,
    pub query_outputs: HashMap<String, String>,
    pub limitations: Vec<String>,
}

fn graph_shadow_config() -> GraphShadowConfig {
    let mode = env::var("KV_SHADOW_MODE")
        .unwrap_or_default()
        .to_lowercase();
    let enabled_flag = env::var("KV_READ_ANALYTICS_ENABLED")
        .or_else(|_| env::var("KV_DUAL_WRITE_ENABLED"))
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
        .arg("--format")
        .arg("plain")
        .arg(query)
        .output()
        .await
        .map_err(|e| format!("cypher-shell spawn error: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("cypher-shell failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn limited_report(reason: &str) -> GraphReadAnalyticsV1 {
    let marker = format!("KV_READ_ANALYTICS_V1|status=limited|reason={}", reason);
    let mut summary = HashMap::new();
    summary.insert(
        "coverageHints".to_string(),
        "limited_or_empty_graph".to_string(),
    );

    GraphReadAnalyticsV1 {
        analytics_version: "kv_read_analytics_v1".to_string(),
        mode: "read_only_shadow".to_string(),
        status: "limited".to_string(),
        marker,
        summary,
        query_outputs: HashMap::new(),
        limitations: vec![
            "graph is unavailable or sparse".to_string(),
            "analytics are inconclusive without sufficient dual-write data".to_string(),
        ],
    }
}

#[tauri::command]
pub async fn kv_read_analytics_v1(
    log_state: State<'_, LogState>,
) -> Result<GraphReadAnalyticsV1, String> {
    let cfg = graph_shadow_config();
    if !cfg.enabled {
        let report = limited_report("disabled");
        push_runtime_log(&log_state, report.marker.clone());
        return Ok(report);
    }
    if cfg.password.trim().is_empty() {
        let report = limited_report("missing_password");
        push_runtime_log(&log_state, report.marker.clone());
        return Ok(report);
    }
    if Command::new("cypher-shell")
        .arg("--version")
        .output()
        .await
        .is_err()
    {
        let report = limited_report("missing_cypher_shell");
        push_runtime_log(&log_state, report.marker.clone());
        return Ok(report);
    }

    let queries = vec![
        (
            "capability_frequency_by_vendor_device",
            "MATCH (r:Run)-[:USED_CAPABILITY]->(c:Capability) OPTIONAL MATCH (r)-[:TARGET_DEVICE]->(d:Device) OPTIONAL MATCH (d)-[:FROM_VENDOR]->(v:Vendor) RETURN coalesce(v.name,'unknown') AS vendor, coalesce(d.device_id,'unknown') AS device, c.capability_key AS capability, count(*) AS freq ORDER BY freq DESC LIMIT 20",
        ),
        (
            "most_common_finding_types_by_capability",
            "MATCH (r:Run)-[:USED_CAPABILITY]->(c:Capability) MATCH (r)-[:PRODUCED_FINDING]->(f:Finding) RETURN c.capability_key AS capability, f.severity AS severity, count(*) AS freq ORDER BY freq DESC LIMIT 20",
        ),
        (
            "evidence_linkage_density",
            "MATCH (f:Finding) OPTIONAL MATCH (f)-[:SUPPORTED_BY]->(e:Evidence) RETURN count(f) AS findings, count(e) AS evidence, CASE WHEN count(f)=0 THEN 0 ELSE round((toFloat(count(e))/toFloat(count(f)))*100)/100 END AS evidence_per_finding",
        ),
        (
            "repeated_validation_paths",
            "MATCH (r:Run)-[:USED_PATH]->(p:ValidationPath) RETURN p.path_key AS path, count(*) AS uses ORDER BY uses DESC LIMIT 20",
        ),
        (
            "inconclusive_or_weak_clusters",
            "MATCH (r:Run)-[:PRODUCED_FINDING]->(f:Finding) OPTIONAL MATCH (f)-[:SUPPORTED_BY]->(e:Evidence) WITH r, f, count(e) AS evidence_count WHERE f.summary CONTAINS 'issues count=' OR evidence_count=0 RETURN count(DISTINCT r) AS runs, count(f) AS weak_findings",
        ),
        (
            "coverage_hints_mature_subset",
            "MATCH (c:Capability) RETURN collect(DISTINCT c.capability_key) AS seen_capabilities",
        ),
    ];

    let mut outputs = HashMap::new();
    let mut limitations = Vec::new();

    for (name, query) in queries {
        match run_cypher(&cfg, query).await {
            Ok(data) => {
                outputs.insert(name.to_string(), data);
            }
            Err(err) => {
                limitations.push(format!("query_failed:{}", name));
                outputs.insert(
                    name.to_string(),
                    format!("error:{}", err.replace('\n', " ")),
                );
            }
        }
    }

    let status = if limitations.is_empty() {
        "ok"
    } else {
        "limited"
    };
    let marker = format!(
        "KV_READ_ANALYTICS_V1|status={}|queries={}|failed={}",
        status,
        outputs.len(),
        limitations.len()
    );

    let mut summary = HashMap::new();
    summary.insert(
        "strongestSignal".to_string(),
        "capability/finding frequency on mature dual-write subset".to_string(),
    );
    summary.insert(
        "weakestSignal".to_string(),
        "inconclusive/weak clusters when graph sparsity is high".to_string(),
    );
    summary.insert(
        "readOnlyGuarantee".to_string(),
        "true (no writes in analytics command)".to_string(),
    );

    push_runtime_log(&log_state, marker.clone());

    Ok(GraphReadAnalyticsV1 {
        analytics_version: "kv_read_analytics_v1".to_string(),
        mode: "read_only_shadow".to_string(),
        status: status.to_string(),
        marker,
        summary,
        query_outputs: outputs,
        limitations,
    })
}
