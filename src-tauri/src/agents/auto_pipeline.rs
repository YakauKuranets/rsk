// src-tauri/src/agents/auto_pipeline.rs
use crate::agents::handoff::HandoffPacket;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineOptions {
    pub scope: String,
    pub permit_token: String,
    pub shodan_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub language: String,
    pub skip_exploit_verify: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineResult {
    pub pipeline_id: String,
    pub stages_completed: Vec<String>,
    pub final_packet: HandoffPacket,
    pub nlp_report: Option<crate::nlp_reporter::NlpReport>,
    pub duration_ms: u128,
}

#[tauri::command]
pub async fn run_full_pipeline(
    options: PipelineOptions,
    scope_guard: State<'_, crate::scope_guard::ScopeGuard>,
    log_state: State<'_, crate::LogState>,
) -> Result<PipelineResult, String> {
    let t0 = std::time::Instant::now();
    let pid = format!("pipeline_{}", chrono::Utc::now().timestamp_millis());
    let mut stages: Vec<String> = Vec::new();
    crate::push_runtime_log(
        &log_state,
        format!("PIPELINE_START|id={}|scope={}", pid, options.scope),
    );

    // Stage 1 — Recon
    let p1 = crate::agents::recon_agent::run_recon_agent(
        options.scope.clone(),
        options.shodan_key.clone(),
        pid.clone(),
        log_state.clone(),
    )
    .await?;
    stages.push("ReconAgent".to_string());

    // Stage 2 — Scan + scope guard
    let p2 = crate::scope_guard::run_scan_agent(p1, scope_guard, &log_state).await?;
    stages.push("ScanAgent".to_string());

    // Stage 3 — ExploitVerify (optional)
    let p3 = if !options.skip_exploit_verify {
        let p = crate::agents::exploit_verify_agent::run_exploit_verify_agent(
            p2,
            options.permit_token.clone(),
            log_state.clone(),
        )
        .await?;
        stages.push("ExploitVerifyAgent".to_string());
        p
    } else {
        p2
    };

    // Stage 4 — Risk Intelligence
    let p4 = crate::agents::risk_agent::run_risk_agent(p3, log_state.clone()).await?;
    stages.push("RiskAgent".to_string());

    // Stage 5 — NLP Report (optional, requires Anthropic API key)
    let nlp = if let Some(key) = options.anthropic_api_key {
        let pj = serde_json::to_string(&p4).map_err(|e| e.to_string())?;
        match crate::nlp_reporter::generate_nlp_report(pj, key, options.language, log_state).await {
            Ok(r) => {
                stages.push("ReportAgent(NLP)".to_string());
                Some(r)
            }
            Err(e) => {
                crate::push_runtime_log(&log_state, format!("NLP_WARN|{}", e));
                None
            }
        }
    } else {
        None
    };

    crate::push_runtime_log(
        &log_state,
        format!(
            "PIPELINE_DONE|id={}|stages={}|ms={}",
            pid,
            stages.len(),
            t0.elapsed().as_millis()
        ),
    );

    Ok(PipelineResult {
        pipeline_id: pid,
        stages_completed: stages,
        final_packet: p4,
        nlp_report: nlp,
        duration_ms: t0.elapsed().as_millis(),
    })
}
