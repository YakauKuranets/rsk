// src-tauri/src/nlp_reporter.rs
use crate::agents::handoff::HandoffPacket;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NlpReport {
    pub executive_summary: String,
    pub key_findings: Vec<String>,
    pub remediation_plan: Vec<String>,
    pub risk_narrative: String,
}

#[tauri::command]
pub async fn generate_nlp_report(
    packet_json: String,
    api_key: String,
    language: String,
    log_state: State<'_, crate::LogState>,
) -> Result<NlpReport, String> {
    let packet: HandoffPacket = serde_json::from_str(&packet_json).map_err(|e| e.to_string())?;
    let r = &packet.context_carry["risk_report"];
    let lang = if language == "ru" {
        "Respond entirely in Russian. Use professional cybersecurity terminology."
    } else {
        "Respond in English. Use professional cybersecurity terminology."
    };
    let prompt = format!(
        "{} Analyze this security assessment and provide a JSON report:\n\
         - Total findings: {}\n\
         - Critical hosts: {}\n\
         - CISA KEV: {} CVEs actively exploited\n\
         - Max threat score: {}/10\n\
         - Primary MITRE tactic: {}\n\
         - Key indicators: {}\n\n\
         Respond ONLY with valid JSON (no markdown) with keys:\n\
         executive_summary (string), key_findings (array), remediation_plan (array), risk_narrative (string)",
        lang,
        packet.findings.len(),
        r["critical_count"].as_u64().unwrap_or(0),
        r["total_kev"].as_u64().unwrap_or(0),
        r["max_score"]
            .as_f64()
            .map(|s| format!("{:.1}", s))
            .unwrap_or_else(|| "?".into()),
        r["top_tactic"].as_str().unwrap_or("Unknown"),
        packet
            .risk_indicators
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let resp: serde_json::Value = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": "claude-opus-4-6",
            "max_tokens": 1500,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let text = resp["content"][0]["text"]
        .as_str()
        .ok_or_else(|| "Empty API response".to_string())?;
    let clean = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let report: NlpReport = serde_json::from_str(clean)
        .map_err(|e| format!("Parse error: {} | raw: {}", e, &clean[..200.min(clean.len())]))?;

    crate::push_runtime_log(&log_state, format!("NLP_DONE|pipeline={}", packet.pipeline_id));
    Ok(report)
}
