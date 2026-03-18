use chrono::Utc;
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tauri::State;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};

use crate::agents::handoff::{
    AgentId, Finding, FindingType, HandoffPacket, HandoffStatus, Severity,
};

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn make_scope_finding(scope: &str) -> Finding {
    let finding_type = if scope.parse::<std::net::IpAddr>().is_ok() || scope.contains('/') {
        FindingType::Asset
    } else {
        FindingType::Intelligence
    };

    Finding {
        host: scope.to_string(),
        finding_type,
        severity: Severity::Info,
        cve: None,
        cvss_score: None,
        description: format!("Scope принят в пайплайн recon: {}", scope),
        evidence: None,
        confidence_score: 1.0,
    }
}

fn shodan_to_findings(payload: &Value) -> Vec<Finding> {
    payload["matches"]
        .as_array()
        .into_iter()
        .flatten()
        .take(10)
        .map(|item| Finding {
            host: item["ip_str"].as_str().unwrap_or_default().to_string(),
            finding_type: FindingType::Exposure,
            severity: Severity::Medium,
            cve: None,
            cvss_score: None,
            description: format!(
                "Shodan exposure: port {} / org {}",
                item["port"].as_u64().unwrap_or_default(),
                item["org"].as_str().unwrap_or("unknown")
            ),
            evidence: item["data"].as_str().map(|v| v.chars().take(240).collect()),
            confidence_score: 0.75,
        })
        .collect()
}

async fn query_shodan(scope: &str, api_key: &str) -> Result<Value, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Hyperion-PTES/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let query = if scope.parse::<std::net::IpAddr>().is_ok() {
        scope.to_string()
    } else {
        format!("hostname:{}", scope)
    };

    let url = format!(
        "https://api.shodan.io/shodan/host/search?key={}&query={}",
        api_key,
        urlencoding::encode(&query)
    );

    timeout(Duration::from_secs(20), async {
        let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
        response.json::<Value>().await.map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "Shodan timeout".to_string())?
}

#[tauri::command]
pub async fn run_recon_agent(
    scope: String,
    shodan_key: Option<String>,
    pipeline_id: String,
    log_state: State<'_, crate::LogState>,
) -> Result<HandoffPacket, String> {
    let scope = scope.trim().to_string();
    if scope.is_empty() {
        return Err("scope пустой".into());
    }

    info!(pipeline_id = %pipeline_id, scope = %scope, "Recon agent started");
    crate::push_runtime_log(
        &log_state,
        format!("RECON_START|pipeline={}|scope={}", pipeline_id, scope),
    );

    let mut findings = vec![make_scope_finding(&scope)];
    let mut risk_indicators = Vec::new();
    let mut status = HandoffStatus::Success;

    let shodan_enabled = shodan_key
        .as_ref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    if let Some(key) = shodan_key.as_deref().filter(|v| !v.trim().is_empty()) {
        match query_shodan(&scope, key).await {
            Ok(payload) => {
                let shodan_findings = shodan_to_findings(&payload);
                if !shodan_findings.is_empty() {
                    risk_indicators.push("externalExposureDetected".to_string());
                    findings.extend(shodan_findings);
                }
            }
            Err(err) => {
                warn!(pipeline_id = %pipeline_id, error = %err, "Recon agent Shodan lookup failed");
                risk_indicators.push("shodanLookupFailed".to_string());
                status = HandoffStatus::Partial { reason: err };
            }
        }
    }

    crate::push_runtime_log(
        &log_state,
        format!(
            "RECON_DONE|pipeline={}|found={}",
            pipeline_id,
            findings.len()
        ),
    );

    Ok(HandoffPacket {
        pipeline_id,
        from_agent: AgentId::ReconAgent,
        timestamp_utc: Utc::now().to_rfc3339(),
        scope_hash: sha256_hex(&scope),
        permit_number: None,
        status,
        findings,
        context_carry: json!({
            "scope": scope,
            "shodanEnabled": shodan_enabled,
        }),
        operator_notes: None,
        risk_indicators,
    })
}
