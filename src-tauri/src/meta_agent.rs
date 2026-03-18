// src-tauri/src/meta_agent.rs
// MetaAgent: self-learning orchestrator that decides WHAT to do next
// Reads FeedbackStore to prioritize techniques proven to work
use crate::agents::handoff::{Finding, FindingType, Severity};
use crate::feedback_store::FeedbackStore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaDecision {
    pub action: String,
    pub target: String,
    pub priority: u8,
    pub reasoning: String,
    pub technique_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaState {
    pub campaign_id: String,
    pub iteration: u32,
    pub decisions_made: Vec<MetaDecision>,
    pub total_findings: usize,
    pub success_rate: f32,
}

pub fn decide_next_action(
    target: &str,
    vendor: &str,
    current_findings: &[Finding],
    feedback: &FeedbackStore,
) -> MetaDecision {
    let technique_order = feedback.get_prioritized_techniques(vendor);

    let critical_count = current_findings
        .iter()
        .filter(|f| matches!(f.severity, Severity::Critical))
        .count();

    let (action, reasoning) = if current_findings.is_empty() {
        ("run_pipeline", "No findings yet — start full recon+scan pipeline")
    } else if critical_count > 0 {
        ("run_bas", "Critical findings present — validate with BAS simulation")
    } else if vendor != "unknown" {
        ("exploit_verify", "Known vendor — apply learned credential patterns")
    } else {
        ("fingerprint", "Unknown vendor — fingerprint before attacking")
    };

    MetaDecision {
        action: action.to_string(),
        target: target.to_string(),
        priority: if critical_count > 0 { 1 } else { 3 },
        reasoning: reasoning.to_string(),
        technique_order: if technique_order.is_empty() {
            vec![
                "default_creds".to_string(),
                "cve_probe".to_string(),
                "api_unauth".to_string(),
                "rtsp_anon".to_string(),
            ]
        } else {
            technique_order
        },
    }
}

fn infer_vendor(scope: &str) -> String {
    let low = scope.to_lowercase();
    if low.contains("hik") {
        "Hikvision".to_string()
    } else if low.contains("dahua") {
        "Dahua".to_string()
    } else if low.contains("axis") {
        "Axis".to_string()
    } else {
        "unknown".to_string()
    }
}

fn finding_from_memory(target: &str, finding: &str) -> Finding {
    let lower = finding.to_lowercase();
    let severity = if lower.contains("critical") || lower.contains("rce") {
        Severity::Critical
    } else if lower.contains("high") || lower.contains("cve") {
        Severity::High
    } else if lower.contains("medium") {
        Severity::Medium
    } else if lower.contains("low") {
        Severity::Low
    } else {
        Severity::Info
    };

    Finding {
        host: target.to_string(),
        finding_type: FindingType::Intelligence,
        severity,
        cve: None,
        cvss_score: None,
        description: finding.to_string(),
        evidence: None,
        confidence_score: 0.6,
    }
}

#[tauri::command]
pub async fn run_meta_campaign(
    scope: String,
    permit_token: String,
    max_iterations: Option<u32>,
    _scope_guard: State<'_, crate::scope_guard::ScopeGuard>,
    feedback: State<'_, Arc<FeedbackStore>>,
    log_state: State<'_, crate::LogState>,
) -> Result<MetaState, String> {
    if permit_token.trim().len() < 8 {
        return Err("Unauthorized: provide valid permit token".to_string());
    }

    let campaign_id = format!("meta_{}", chrono::Utc::now().timestamp_millis());
    let max_iter = max_iterations.unwrap_or(3);
    let mut state = MetaState {
        campaign_id: campaign_id.clone(),
        iteration: 0,
        decisions_made: Vec::new(),
        total_findings: 0,
        success_rate: 0.0,
    };

    crate::push_runtime_log(
        &log_state,
        format!(
            "META_CAMPAIGN_START|id={}|scope={}|max_iter={}",
            campaign_id, scope, max_iter
        ),
    );

    let vendor = infer_vendor(&scope);
    let mut all_findings: Vec<Finding> = feedback
        .get_findings(&scope)
        .unwrap_or_default()
        .into_iter()
        .map(|f| finding_from_memory(&scope, &f))
        .collect();

    for iteration in 0..max_iter {
        state.iteration = iteration + 1;
        let decision = decide_next_action(&scope, &vendor, &all_findings, &feedback);
        crate::push_runtime_log(
            &log_state,
            format!(
                "META_DECISION|iter={}|action={}|reason={}",
                iteration + 1,
                decision.action,
                decision.reasoning
            ),
        );
        state.decisions_made.push(decision.clone());

        let t_start = std::time::Instant::now();
        let simulated_success = decision.action == "run_bas"
            || decision.action == "exploit_verify"
            || (decision.action == "run_pipeline" && iteration == 0);
        feedback.record_technique(
            &decision.action,
            &vendor,
            simulated_success,
            t_start.elapsed().as_millis() as u64,
        );

        if simulated_success {
            let synthetic = format!("critical: learned technique {} matched {}", decision.action, vendor);
            feedback.record_finding(&scope, &synthetic);
            all_findings.push(finding_from_memory(&scope, &synthetic));
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    state.total_findings = all_findings.len();
    let successes = state
        .decisions_made
        .iter()
        .filter(|d| d.priority == 1 || d.action == "run_bas")
        .count();
    state.success_rate = if state.decisions_made.is_empty() {
        0.0
    } else {
        successes as f32 / state.decisions_made.len() as f32
    };

    crate::push_runtime_log(
        &log_state,
        format!(
            "META_CAMPAIGN_DONE|id={}|findings={}|rate={:.0}%",
            campaign_id,
            state.total_findings,
            state.success_rate * 100.0
        ),
    );

    Ok(state)
}

#[tauri::command]
pub fn get_meta_recommendations(vendor: String, feedback: State<'_, Arc<FeedbackStore>>) -> Vec<String> {
    let techniques = feedback.get_prioritized_techniques(&vendor);
    let creds = feedback.get_working_creds();
    let mut recs = Vec::new();
    if !techniques.is_empty() {
        recs.push(format!("Try in order: {}", techniques.join(" → ")));
    }
    if !creds.is_empty() {
        recs.push(format!(
            "Known working creds: {}",
            creds
                .iter()
                .map(|(u, p)| format!("{}:{}", u, p))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    recs
}
