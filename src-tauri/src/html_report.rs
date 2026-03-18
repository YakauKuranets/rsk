// src-tauri/src/html_report.rs
// Professional HTML report generation — executive + technical views
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportConfig {
    pub title: String,
    pub client_name: String,
    pub operator_name: String,
    pub include_executive: bool,
    pub include_technical: bool,
    pub include_mitre_heatmap: bool,
    pub classification: String, // "CONFIDENTIAL" | "RESTRICTED" | "INTERNAL"
}

fn severity_color(s: &str) -> &'static str {
    match s {
        "Critical" => "#E74C3C",
        "High" => "#E67E22",
        "Medium" => "#F1C40F",
        "Low" => "#27AE60",
        "Info" => "#3498DB",
        _ => "#95A5A6",
    }
}

fn render_mitre_heatmap(findings: &[serde_json::Value]) -> String {
    let mut tactic_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let tactics = [
        "Initial Access",
        "Execution",
        "Persistence",
        "Privilege Escalation",
        "Defense Evasion",
        "Credential Access",
        "Discovery",
        "Lateral Movement",
        "Collection",
        "Exfiltration",
        "Impact",
    ];

    for f in findings {
        if let Some(tactic) = f["mitreTactic"].as_str() {
            *tactic_counts.entry(tactic).or_insert(0) += 1;
        }
    }

    let cells = tactics
        .iter()
        .map(|&tac| {
            let count = tactic_counts.get(tac).copied().unwrap_or(0);
            let intensity = match count {
                0 => "#1a1a2e",
                1 => "#16213e",
                2..=3 => "#0f3460",
                4..=6 => "#533483",
                _ => "#e94560",
            };
            format!(
                "<div style=\"background:{};padding:8px 12px;border-radius:4px;font-size:11px;color:#fff;text-align:center;\"><div style=\"font-weight:500;font-size:13px;\">{}</div><div>{} findings</div></div>",
                intensity, tac, count
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        "<div class=\"mitre_heatmap\" style=\"display:grid;grid-template-columns:repeat(4,1fr);gap:8px;\">{}</div>",
        cells
    )
}

#[tauri::command]
pub async fn generate_html_report(
    findings_json: String,
    nlp_report_json: Option<String>,
    config: ReportConfig,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    let findings: Vec<serde_json::Value> =
        serde_json::from_str(&findings_json).map_err(|e| e.to_string())?;

    let critical = findings
        .iter()
        .filter(|f| f["severity"].as_str() == Some("Critical"))
        .count();
    let high = findings
        .iter()
        .filter(|f| f["severity"].as_str() == Some("High"))
        .count();
    let date = chrono::Utc::now().format("%d %B %Y").to_string();

    let nlp: Option<serde_json::Value> = nlp_report_json.and_then(|j| serde_json::from_str(&j).ok());

    let exec_summary = nlp
        .as_ref()
        .and_then(|n| n["executiveSummary"].as_str())
        .unwrap_or("Assessment completed. Review findings below.");

    let findings_html: String = findings
        .iter()
        .enumerate()
        .map(|(_, f)| {
            let sev = f["severity"].as_str().unwrap_or("Info");
            let color = severity_color(sev);
            format!(
                "<tr style=\"border-bottom:1px solid #eee\"><td style=\"padding:8px 12px;font-size:13px;font-weight:500;color:{};\">{}</td><td style=\"padding:8px 12px;font-family:monospace;font-size:12px;\">{}</td><td style=\"padding:8px 12px;font-size:12px;font-weight:500;\">{}</td><td style=\"padding:8px 12px;font-size:12px;color:#666;\">{}</td></tr>",
                color,
                sev,
                f["host"].as_str().unwrap_or("-"),
                f["cve"].as_str().unwrap_or("-"),
                f["description"].as_str().unwrap_or("-")
            )
        })
        .collect();

    let mitre_section = if config.include_mitre_heatmap {
        format!("<h2>MITRE ATT&CK Coverage</h2>{}", render_mitre_heatmap(&findings))
    } else {
        String::new()
    };

    let remediation_html = nlp
        .as_ref()
        .and_then(|n| n["remediationPlan"].as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    format!(
                        "<li style=\"margin-bottom:8px\">{}</li>",
                        item.as_str().unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .map(|li| format!("<h2>Remediation Plan</h2><ol>{}</ol>", li))
        .unwrap_or_default();

    let exec_section = if config.include_executive {
        format!(
            "<h2>Executive Summary</h2><div class=\"exec-box\">{}</div>",
            exec_summary
        )
    } else {
        String::new()
    };

    let technical_note = if config.include_technical {
        "<p style=\"color:#666;font-size:13px;margin:0 0 16px;\">Technical appendix included in the findings and remediation sections below.</p>"
    } else {
        ""
    };

    let html = format!(
        r#"<!DOCTYPE html><html lang="en">
<head><meta charset="UTF-8">
<title>Security Assessment — {title}</title>
<style>
  body{{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
       margin:0;padding:0;background:#f5f5f5;color:#1a1a1a}}
  .cover{{background:linear-gradient(135deg,#1a1a2e 0%,#16213e 60%,#0f3460 100%);
          color:#fff;padding:60px 80px;min-height:200px}}
  .cover h1{{font-size:36px;margin:0 0 12px;font-weight:700}}
  .badge{{display:inline-block;background:rgba(233,69,96,0.9);color:#fff;
          padding:4px 16px;border-radius:20px;font-size:12px;font-weight:700;
          text-transform:uppercase;letter-spacing:1px;margin-bottom:20px}}
  .metrics{{display:grid;grid-template-columns:repeat(4,1fr);gap:16px;
            margin:24px 0}}
  .metric{{background:#fff;border-radius:8px;padding:16px;text-align:center;
           box-shadow:0 2px 8px rgba(0,0,0,0.08)}}
  .metric-n{{font-size:32px;font-weight:700;line-height:1}}
  .metric-l{{font-size:12px;color:#666;margin-top:4px}}
  .content{{max-width:1100px;margin:32px auto;padding:0 32px}}
  h2{{font-size:20px;font-weight:600;margin:32px 0 16px;color:#1a1a2e}}
  .exec-box{{background:#fff;border-left:4px solid #0f3460;padding:16px 20px;
             border-radius:0 8px 8px 0;margin-bottom:24px;line-height:1.7}}
  table{{width:100%;border-collapse:collapse;background:#fff;
         border-radius:8px;overflow:hidden;box-shadow:0 2px 8px rgba(0,0,0,0.06)}}
  thead{{background:#f8f9fa}} th{{padding:10px 12px;text-align:left;
         font-size:12px;font-weight:600;color:#666;text-transform:uppercase}}
  .footer{{text-align:center;padding:32px;color:#999;font-size:12px}}
</style></head>
<body>
<div class="cover">
  <div class="badge">{classification}</div>
  <h1>{title}</h1>
  <p style="opacity:0.8;margin:0">Client: {client} &nbsp;|&nbsp; Operator: {operator} &nbsp;|&nbsp; {date}</p>
</div>
<div class="content">
  <div class="metrics">
    <div class="metric"><div class="metric-n" style="color:#E74C3C">{crit}</div>
      <div class="metric-l">Critical</div></div>
    <div class="metric"><div class="metric-n" style="color:#E67E22">{high}</div>
      <div class="metric-l">High</div></div>
    <div class="metric"><div class="metric-n" style="color:#3498DB">{total}</div>
      <div class="metric-l">Total Findings</div></div>
    <div class="metric"><div class="metric-n" style="color:#27AE60">PTES</div>
      <div class="metric-l">Standard</div></div>
  </div>
  {exec_section}
  {technical_note}
  {mitre}
  <h2>Findings</h2>
  <table><thead><tr>
    <th>Severity</th><th>Host</th><th>CVE</th><th>Description</th>
  </tr></thead><tbody>{findings}</tbody></table>
  {remediation}
</div>
<div class="footer">Generated by Hyperion PTES &nbsp;|&nbsp; {date}</div>
</body></html>"#,
        title = config.title,
        classification = config.classification,
        client = config.client_name,
        operator = config.operator_name,
        date = date,
        crit = critical,
        high = high,
        total = findings.len(),
        exec_section = exec_section,
        technical_note = technical_note,
        mitre = mitre_section,
        findings = findings_html,
        remediation = remediation_html,
    );

    let fname = format!("report_{}.html", chrono::Utc::now().timestamp());
    let path = crate::get_vault_path().join("reports").join(&fname);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, &html).map_err(|e| e.to_string())?;

    crate::push_runtime_log(&log_state, format!("HTML_REPORT|file={}", fname));
    Ok(path.display().to_string())
}
