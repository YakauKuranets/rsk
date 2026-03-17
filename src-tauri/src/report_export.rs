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
