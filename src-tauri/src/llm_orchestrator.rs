// src-tauri/src/llm_orchestrator.rs
// Local LLM decision engine via Ollama HTTP API
// Ollama runs locally: curl https://ollama.ai/install.sh | sh && ollama pull llama3
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    pub ollama_url: String,    // default: "http://localhost:11434"
    pub model: String,         // "llama3" | "mistral" | "phi3"
    pub temperature: f32,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmDecision {
    pub action: String,
    pub parameters: serde_json::Value,
    pub reasoning: String,
    pub confidence: f32,
}


#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}


#[derive(Serialize)]
struct OllamaOptions { temperature: f32, num_predict: i32 }


#[derive(Deserialize)]
struct OllamaResponse { response: String }


async fn query_ollama(config: &LlmConfig, prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build().map_err(|e| e.to_string())?;


    let url = format!("{}/api/generate", config.ollama_url.trim_end_matches('/'));
    let body = OllamaRequest {
        model: config.model.clone(),
        prompt: prompt.to_string(),
        stream: false,
        options: OllamaOptions {
            temperature: config.temperature,
            num_predict: 512,
        },
    };


    let resp: OllamaResponse = client.post(&url)
        .json(&body)
        .send().await.map_err(|e| format!("Ollama not running: {}. Install: curl https://ollama.ai/install.sh | sh && ollama serve", e))?
        .json().await.map_err(|e| e.to_string())?;


    Ok(resp.response)
}


#[tauri::command]
pub async fn llm_analyze_findings(
    findings_json: String,
    config: LlmConfig,
    log_state: State<'_, crate::LogState>,
) -> Result<LlmDecision, String> {
    let findings: Vec<serde_json::Value> = serde_json::from_str(&findings_json)
        .map_err(|e| e.to_string())?;


    let critical_hosts: Vec<String> = findings.iter()
        .filter(|f| f["severity"].as_str() == Some("Critical"))
        .filter_map(|f| f["host"].as_str().map(|s| s.to_string()))
        .collect::<std::collections::HashSet<_>>().into_iter().collect();


    let prompt = format!(r#"You are a cybersecurity AI assistant for penetration testing.
Analyze these security findings and decide the next best action.


Findings summary:
- Total findings: {}
- Critical hosts: {:?}
- Top CVEs: {}


Respond with a JSON object only, no other text:
{{"action": "run_exploit|run_bas|gather_more_info|report", "target": "host_or_scope",
"reasoning": "brief explanation", "confidence": 0.0-1.0}}


Consider: if critical findings exist, prioritize exploit verification. If no findings, gather more info."#,
        findings.len(),
        critical_hosts,
        findings.iter().filter_map(|f| f["cve"].as_str()).take(5).collect::<Vec<_>>().join(", "),
    );


    crate::push_runtime_log(&log_state, format!("LLM_ANALYZE|model={}", config.model));
    let raw = query_ollama(&config, &prompt).await?;


    // Extract JSON from response
    let clean = raw.trim()
        .trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```").trim();


    let json: serde_json::Value = serde_json::from_str(clean)
        .unwrap_or_else(|_| serde_json::json!({
            "action": "gather_more_info",
            "target": "scope",
            "reasoning": raw.chars().take(200).collect::<String>(),
            "confidence": 0.5
        }));


    Ok(LlmDecision {
        action: json["action"].as_str().unwrap_or("gather_more_info").to_string(),
        parameters: json.clone(),
        reasoning: json["reasoning"].as_str().unwrap_or("").to_string(),
        confidence: json["confidence"].as_f64().unwrap_or(0.5) as f32,
    })
}


#[tauri::command]
pub async fn llm_generate_attack_plan(
    target_profile: String,
    config: LlmConfig,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<String>, String> {
    let prompt = format!(r#"You are a red team AI assistant.
Given this target profile, generate a step-by-step attack plan using MITRE ATT&CK techniques.


Target: {}


Respond with a JSON array of strings, each string being one attack step.
Example: ["1. Recon via Shodan for exposed RTSP ports", "2. Probe default credentials admin:admin", ...]
Limit to 7 steps maximum. Be specific and actionable."#, target_profile);


    crate::push_runtime_log(&log_state, "LLM_ATTACK_PLAN|start".to_string());
    let raw = query_ollama(&config, &prompt).await?;


    let clean = raw.trim()
        .trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```").trim();


    let steps: Vec<String> = serde_json::from_str(clean)
        .unwrap_or_else(|_| raw.lines().map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty()).take(7).collect());


    Ok(steps)
}


#[tauri::command]
pub async fn llm_health_check(
    ollama_url: String,
) -> Result<bool, String> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(3)).build().unwrap_or_default();
    let resp = client.get(&format!("{}/api/tags", ollama_url.trim_end_matches('/')))
        .send().await;
    Ok(resp.map(|r| r.status().is_success()).unwrap_or(false))
}



#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackHypothesis {
    pub technique: String,
    pub rationale: String,
    pub confidence: f32,
    pub prerequisites: Vec<String>,
}

fn default_hypotheses(target_profile: &str) -> Vec<AttackHypothesis> {
    let lower = target_profile.to_lowercase();
    let mut out = Vec::new();

    if lower.contains("554") || lower.contains("rtsp") {
        out.push(AttackHypothesis {
            technique: "rtsp_anon".to_string(),
            rationale: "RTSP/554 hints suggest trying anonymous or weakly protected stream access.".to_string(),
            confidence: 0.72,
            prerequisites: vec!["Confirm TCP/554 reachability".to_string()],
        });
    }
    if lower.contains("21") || lower.contains("ftp") {
        out.push(AttackHypothesis {
            technique: "ftp_anon".to_string(),
            rationale: "FTP exposure often correlates with archive export or anonymous access testing.".to_string(),
            confidence: 0.68,
            prerequisites: vec!["Verify TCP/21 open".to_string()],
        });
    }
    if lower.contains("hik") || lower.contains("isapi") || lower.contains("80") {
        out.push(AttackHypothesis {
            technique: "isapi_search".to_string(),
            rationale: "HTTP/ISAPI indicators suggest enumerating archive and device management endpoints.".to_string(),
            confidence: 0.74,
            prerequisites: vec!["Reach HTTP management interface".to_string()],
        });
    }
    if lower.contains("onvif") || lower.contains("8000") || lower.contains("8899") {
        out.push(AttackHypothesis {
            technique: "onvif_probe".to_string(),
            rationale: "ONVIF-related exposure can reveal capabilities, recordings, and media services.".to_string(),
            confidence: 0.66,
            prerequisites: vec!["Probe ONVIF device_service endpoint".to_string()],
        });
    }

    if out.is_empty() {
        out.push(AttackHypothesis {
            technique: "default_creds".to_string(),
            rationale: "Default credential validation is the safest general-purpose starting hypothesis when context is sparse.".to_string(),
            confidence: 0.55,
            prerequisites: vec!["Identify reachable management endpoint".to_string()],
        });
        out.push(AttackHypothesis {
            technique: "cve_probe".to_string(),
            rationale: "Fallback hypothesis: fingerprint exposure and compare against known vendor/firmware vulnerabilities.".to_string(),
            confidence: 0.51,
            prerequisites: vec!["Collect vendor and firmware clues".to_string()],
        });
    }

    out.truncate(6);
    out
}

#[tauri::command]
pub async fn llm_generate_hypotheses(
    target_profile: String,
    config: Option<LlmConfig>,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<AttackHypothesis>, String> {
    crate::push_runtime_log(&log_state, "LLM_HYPOTHESES|start".to_string());

    let Some(config) = config else {
        return Ok(default_hypotheses(&target_profile));
    };

    let prompt = format!(r#"You are a cybersecurity AI assistant.
Generate up to 6 attack or reconnaissance hypotheses for the following target profile.
Respond with JSON array only.
Each item must be:
{{"technique":"short_name","rationale":"why","confidence":0.0-1.0,"prerequisites":["item1"]}}

Target profile:
{}
"#, target_profile);

    let raw = match query_ollama(&config, &prompt).await {
        Ok(v) => v,
        Err(_) => return Ok(default_hypotheses(&target_profile)),
    };

    let clean = raw.trim()
        .trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```").trim();

    let hypotheses: Vec<AttackHypothesis> = serde_json::from_str(clean)
        .unwrap_or_else(|_| default_hypotheses(&target_profile));

    Ok(hypotheses)
}
