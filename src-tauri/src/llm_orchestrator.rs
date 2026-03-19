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



// ДОБАВИТЬ в конец src-tauri/src/llm_orchestrator.rs
// (не заменять весь файл — добавить эти структуры и функции)


// ═══════ НОВЫЕ СТРУКТУРЫ ═══════════════════════════════════════


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackHypothesis {
    pub technique: String,
    pub description: String,
    pub expected_probability: f32,  // 0.0..1.0
    pub stealth_level: u8,          // 1..5
    pub required_conditions: Vec<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HypothesisRequest {
    pub vendor: String,
    pub firmware: String,
    pub open_ports: Vec<u16>,
    pub already_failed: Vec<String>,
    pub config: LlmConfig,
}


// ═══ НОВАЯ TAURI КОМАНДА ════════════════════════════════════


#[tauri::command]
pub async fn llm_generate_hypotheses(
    req: HypothesisRequest,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<AttackHypothesis>, String> {
    let prompt = format!(
r#"You are a cybersecurity red team AI specializing in IoT/CCTV devices.


Target profile:
- Vendor: {}
- Firmware: {}
- Open ports: {:?}
- Already failed techniques: {}


Generate exactly 5 attack hypotheses. Each must be specific and actionable.
Respond ONLY with valid JSON array, no other text:
[
  {{
    "technique": "short_snake_case_name",
    "description": "what to do specifically",
    "expected_probability": 0.0,
    "stealth_level": 1,
    "required_conditions": ["condition1", "condition2"]
  }}
]
"#,
        req.vendor, req.firmware, req.open_ports,
        req.already_failed.join(", ")
    );


    crate::push_runtime_log(&log_state,
        format!("LLM_HYPOTHESES|vendor={}|failed={}", req.vendor, req.already_failed.len()));


    let raw = match query_ollama(&req.config, &prompt).await {
        Ok(v) => v,
        Err(_) => return Ok(default_hypotheses(&req.vendor)),
    };


    let clean = raw.trim()
        .trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```").trim();


    // Парсим JSON — если LLM ответил неправильно, возвращаем дефолтные
    let hypotheses: Vec<AttackHypothesis> = serde_json::from_str(clean)
        .unwrap_or_else(|_| default_hypotheses(&req.vendor));


    Ok(hypotheses)
}


// Дефолтные гипотезы если LLM недоступен
fn default_hypotheses(vendor: &str) -> Vec<AttackHypothesis> {
    let base = match vendor {
        "Hikvision" => vec![
            ("cve_2021_36260", "RCE через /SDK/webLanguage без авторизации", 0.7, 2),
            ("isapi_unauth", "ISAPI /ISAPI/Security/users без токена", 0.6, 3),
            ("rtsp_default", "RTSP rtsp://admin:12345@host/Streaming/Channels/101", 0.5, 4),
        ],
        "Dahua" => vec![
            ("cve_2021_33045", "Auth bypass через /cgi-bin/snapshot.cgi", 0.65, 2),
            ("xm_probe", "XM protocol port 37777 enumeration", 0.5, 3),
            ("rtsp_default", "RTSP rtsp://admin:admin@host/cam/realmonitor", 0.55, 4),
        ],
        _ => vec![
            ("default_creds", "admin:admin / admin:12345 / admin:", 0.4, 4),
            ("rtsp_anon", "Anonymous RTSP stream access", 0.35, 5),
            ("onvif_enum", "ONVIF device enumeration without auth", 0.4, 4),
        ],
    };
    base.into_iter().map(|(t, d, p, s)| AttackHypothesis {
        technique: t.to_string(), description: d.to_string(),
        expected_probability: p, stealth_level: s,
        required_conditions: vec![],
    }).collect()
}
