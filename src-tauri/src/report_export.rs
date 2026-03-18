use serde::Serialize;
use std::time::Duration;
use tauri::State;
use tokio::net::UdpSocket;
use tokio::time::{sleep, timeout};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportSummary {
    findings: usize,
    generated_at: String,
}

fn ensure_reports_dir() -> Result<std::path::PathBuf, String> {
    let path = crate::get_vault_path().join("reports");
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

#[tauri::command]
pub async fn export_report_json(
    campaign_data: String,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    crate::push_runtime_log(&log_state, "[REPORT] export json".to_string());
    let value: serde_json::Value =
        serde_json::from_str(&campaign_data).map_err(|e| e.to_string())?;
    let pretty = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;

    let dir = ensure_reports_dir()?;
    let file = dir.join(format!(
        "report_{}.json",
        chrono::Utc::now().timestamp_millis()
    ));
    tokio::fs::write(&file, pretty)
        .await
        .map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(120)).await;
    Ok(file.display().to_string())
}

#[tauri::command]
pub async fn export_report_csv(
    campaign_data: String,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    crate::push_runtime_log(&log_state, "[REPORT] export csv".to_string());
    let value: serde_json::Value =
        serde_json::from_str(&campaign_data).map_err(|e| e.to_string())?;

    let mut lines = vec!["id,type,severity,target,description".to_string()];
    if let Some(items) = value.as_array() {
        for (idx, item) in items.iter().enumerate() {
            let id = item["id"]
                .as_str()
                .map(|x| x.to_string())
                .unwrap_or_else(|| format!("finding_{}", idx));
            let t = item["type"].as_str().unwrap_or("unknown");
            let s = item["severity"].as_str().unwrap_or("n/a");
            let target = item["target"].as_str().unwrap_or("n/a");
            let desc = item["description"]
                .as_str()
                .unwrap_or("")
                .replace('"', "''");
            lines.push(format!(
                "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
                id, t, s, target, desc
            ));
        }
    }

    let dir = ensure_reports_dir()?;
    let file = dir.join(format!(
        "report_{}.csv",
        chrono::Utc::now().timestamp_millis()
    ));
    tokio::fs::write(&file, lines.join("\n"))
        .await
        .map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(120)).await;
    Ok(file.display().to_string())
}

#[tauri::command]
pub async fn export_report_markdown(
    campaign_data: String,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    crate::push_runtime_log(&log_state, "[REPORT] export markdown".to_string());
    let value: serde_json::Value =
        serde_json::from_str(&campaign_data).map_err(|e| e.to_string())?;

    let findings = value.as_array().map(|a| a.len()).unwrap_or(0);
    let summary = ReportSummary {
        findings,
        generated_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut out = String::new();
    out.push_str("# Hyperion Security Report\n\n");
    out.push_str(&format!("- Generated: {}\n", summary.generated_at));
    out.push_str(&format!("- Findings: {}\n\n", summary.findings));
    out.push_str("## Findings\n\n");

    if let Some(items) = value.as_array() {
        for (idx, item) in items.iter().enumerate() {
            out.push_str(&format!(
                "### {}. {}\n",
                idx + 1,
                item["id"].as_str().unwrap_or("finding")
            ));
            out.push_str(&format!(
                "- Type: {}\n",
                item["type"].as_str().unwrap_or("unknown")
            ));
            out.push_str(&format!(
                "- Severity: {}\n",
                item["severity"].as_str().unwrap_or("n/a")
            ));
            out.push_str(&format!(
                "- Target: {}\n",
                item["target"].as_str().unwrap_or("n/a")
            ));
            out.push_str(&format!(
                "- Description: {}\n\n",
                item["description"].as_str().unwrap_or("")
            ));
        }
    }

    let dir = ensure_reports_dir()?;
    let file = dir.join(format!(
        "report_{}.md",
        chrono::Utc::now().timestamp_millis()
    ));
    tokio::fs::write(&file, out)
        .await
        .map_err(|e| e.to_string())?;
    sleep(Duration::from_millis(120)).await;
    Ok(file.display().to_string())
}

#[tauri::command]
pub async fn send_to_syslog(
    findings: String,
    syslog_host: String,
    syslog_port: u16,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    crate::push_runtime_log(
        &log_state,
        format!("[REPORT] send syslog {}:{}", syslog_host, syslog_port),
    );

    let pri = 14;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let hostname = "hyperion";
    let app = "hyperion-agent";
    let msg = format!(
        "<{}>1 {} {} {} - - {}",
        pri,
        timestamp,
        hostname,
        app,
        findings.replace('\n', " | ")
    );

    let socket = timeout(Duration::from_secs(3), UdpSocket::bind("0.0.0.0:0"))
        .await
        .map_err(|_| "syslog bind timeout".to_string())?
        .map_err(|e| e.to_string())?;

    timeout(
        Duration::from_secs(3),
        socket.send_to(msg.as_bytes(), format!("{}:{}", syslog_host, syslog_port)),
    )
    .await
    .map_err(|_| "syslog send timeout".to_string())?
    .map_err(|e| e.to_string())?;

    sleep(Duration::from_millis(100)).await;
    Ok("syslog sent".into())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiemConfig {
    pub target: String, // "splunk" | "elastic" | "qradar" | "graylog"
    pub host: String,
    pub port: u16,
    pub token: Option<String>, // HEC token for Splunk
    pub index: Option<String>,
}

/// Send findings to Splunk HEC (HTTP Event Collector)
#[tauri::command]
pub async fn send_to_splunk_hec(
    findings_json: String,
    config: SiemConfig,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    let findings: Vec<serde_json::Value> =
        serde_json::from_str(&findings_json).map_err(|e| e.to_string())?;
    let token = config
        .token
        .as_deref()
        .ok_or_else(|| "Splunk HEC token required".to_string())?;
    let url = format!(
        "https://{}:{}/services/collector/event",
        config.host, config.port
    );

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // Splunk often uses self-signed
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let mut sent = 0usize;
    for finding in &findings {
        let event = serde_json::json!({
            "time": chrono::Utc::now().timestamp(),
            "host": "hyperion",
            "source": "hyperion-ptes",
            "sourcetype": "hyperion:finding",
            "index": config.index.as_deref().unwrap_or("main"),
            "event": {
                "host": finding["host"],
                "severity": finding["severity"],
                "cve": finding["cve"],
                "cvss_score": finding["cvssScore"],
                "description": finding["description"],
                "mitre_technique": finding["mitreTechniqueId"],
                "threat_score": finding["threatScore"],
            }
        });
        let resp = client
            .post(&url)
            .header("Authorization", format!("Splunk {}", token))
            .json(&event)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if resp.status().is_success() {
            sent += 1;
        }
    }
    crate::push_runtime_log(&log_state, format!("SPLUNK_HEC|sent={}", sent));
    Ok(format!("Sent {} events to Splunk", sent))
}

/// Send findings in Elastic Common Schema (ECS) format
#[tauri::command]
pub async fn send_to_elastic(
    findings_json: String,
    config: SiemConfig,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    let findings: Vec<serde_json::Value> =
        serde_json::from_str(&findings_json).map_err(|e| e.to_string())?;
    let index = config.index.as_deref().unwrap_or("hyperion-findings");
    let url = format!("https://{}:{}/{}/_bulk", config.host, config.port, index);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    // Elastic bulk format: {"index":{}} newline {"doc"} newline
    let mut bulk_body = String::new();
    for finding in &findings {
        bulk_body.push_str("{\"index\":{}}\n");
        let ecs = serde_json::json!({
            "@timestamp": chrono::Utc::now().to_rfc3339(),
            "event.kind": "alert",
            "event.category": "vulnerability",
            "event.severity": match finding["severity"].as_str().unwrap_or("") {
                "Critical" => 1,
                "High" => 2,
                "Medium" => 3,
                _ => 4,
            },
            "host.ip": [finding["host"]],
            "vulnerability.id": finding["cve"],
            "vulnerability.score.base": finding["cvssScore"],
            "vulnerability.description": finding["description"],
            "threat.technique.id": finding["mitreTechniqueId"],
            "labels.threat_score": finding["threatScore"],
            "observer.product": "Hyperion PTES",
        });
        bulk_body.push_str(&serde_json::to_string(&ecs).map_err(|e| e.to_string())?);
        bulk_body.push('\n');
    }

    let resp = client
        .post(&url)
        .header("Content-Type", "application/x-ndjson")
        .body(bulk_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    crate::push_runtime_log(
        &log_state,
        format!("ELASTIC|status={}|findings={}", resp.status(), findings.len()),
    );
    Ok(format!(
        "Sent {} findings to Elastic ({})",
        findings.len(),
        resp.status()
    ))
}

/// Send in QRadar LEEF format over syslog
#[tauri::command]
pub async fn send_to_qradar(
    findings_json: String,
    config: SiemConfig,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    let findings: Vec<serde_json::Value> =
        serde_json::from_str(&findings_json).map_err(|e| e.to_string())?;
    let sock = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| e.to_string())?;
    let dest = format!("{}:{}", config.host, config.port);
    let mut sent = 0usize;

    for f in &findings {
        // LEEF 2.0 format
        let leef = format!(
            "LEEF:2.0|Hyperion|PTES|1.0|Finding|\tcat=Vulnerability\tsev={}\tsrc={}\tusrName=hyperion\tcve={}\tmsg={}",
            f["severity"].as_str().unwrap_or("Unknown"),
            f["host"].as_str().unwrap_or("0.0.0.0"),
            f["cve"].as_str().unwrap_or("-"),
            f["description"].as_str().unwrap_or("").replace('\t', " "),
        );
        let _ = sock.send_to(leef.as_bytes(), &dest).await;
        sent += 1;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    crate::push_runtime_log(&log_state, format!("QRADAR_LEEF|sent={}", sent));
    Ok(format!("Sent {} LEEF events to QRadar", sent))
}
