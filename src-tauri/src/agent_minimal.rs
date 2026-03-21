use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

use crate::capability_adapter::{
    self, CapabilityName, CapabilityRequest, CapabilityResult, CapabilityResultData, ProbeStreamInput,
    VerifySessionCookieFlagsInput,
};
use crate::core_types::WorkflowMode;

static AGENT_RUN_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerInput {
    pub target_id: String,
    pub mode: WorkflowMode,
    pub preferred_capability: Option<CapabilityName>,
    pub verify_session_cookie_flags: Option<VerifySessionCookieFlagsInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedAction {
    pub capability: CapabilityName,
    pub mode: WorkflowMode,
    pub rationale: String,
    pub confidence: f32,
    pub probe_stream: Option<ProbeStreamInput>,
    pub verify_session_cookie_flags: Option<VerifySessionCookieFlagsInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerOutput {
    pub actions: Vec<PlannedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewerInput {
    pub actions: Vec<PlannedAction>,
    pub permit_probe_stream: bool,
    pub permit_verify_session_cookie_flags: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewDecision {
    pub approved: bool,
    pub reasons: Vec<String>,
    pub action: Option<PlannedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerDecisionSummary {
    pub action_count: usize,
    pub primary_capability: Option<CapabilityName>,
    pub rationale: Option<String>,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewerVerdictSummary {
    pub approved: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityArgsSummary {
    pub target_id: Option<String>,
    pub ip_or_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityResultSummary {
    pub ok: bool,
    pub alive: Option<bool>,
    pub secure: Option<bool>,
    pub issues_count: Option<usize>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MinimalAgentFinalStatus {
    ReviewerRejected,
    CapabilitySucceeded,
    CapabilityFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMinimalTraceEnvelope {
    pub agent_run_id: String,
    pub target_id: String,
    pub mode: WorkflowMode,
    pub planner_decision: PlannerDecisionSummary,
    pub reviewer_verdict: ReviewerVerdictSummary,
    pub capability_invoked: Option<CapabilityName>,
    pub capability_args_summary: Option<CapabilityArgsSummary>,
    pub capability_result_summary: Option<CapabilityResultSummary>,
    pub evidence_refs: Vec<String>,
    pub reporter_summary: String,
    pub final_status: MinimalAgentFinalStatus,
    pub capability_result: Option<CapabilityResult>,
}

fn make_agent_run_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = AGENT_RUN_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("amr_{}_{}", millis, seq)
}

fn planner(input: PlannerInput) -> PlannerOutput {
    let preferred = input.preferred_capability.clone().unwrap_or(CapabilityName::ProbeStream);
    let (capability, rationale, probe_stream, verify_session_cookie_flags) = match preferred {
        CapabilityName::VerifySessionCookieFlags => (
            CapabilityName::VerifySessionCookieFlags,
            "Verify session cookie security flags for target endpoint".to_string(),
            None,
            Some(VerifySessionCookieFlagsInput {
                ip_or_url: input
                    .verify_session_cookie_flags
                    .as_ref()
                    .map(|v| v.ip_or_url.clone())
                    .unwrap_or_else(|| input.target_id.clone()),
            }),
        ),
        CapabilityName::ProbeStream => (
            CapabilityName::ProbeStream,
            "Probe stream availability for initial liveness signal".to_string(),
            Some(ProbeStreamInput {
                target_id: input.target_id,
            }),
            None,
        ),
        _ => (
            CapabilityName::ProbeStream,
            "Unsupported preferred capability; fallback to probe_stream".to_string(),
            Some(ProbeStreamInput {
                target_id: input.target_id,
            }),
            None,
        ),
    };

    PlannerOutput {
        actions: vec![PlannedAction {
            capability,
            mode: input.mode,
            rationale,
            confidence: 0.8,
            probe_stream,
            verify_session_cookie_flags,
        }],
    }
}

fn reviewer(input: ReviewerInput) -> ReviewDecision {
    let mut reasons = Vec::new();

    let action = input.actions.into_iter().next();
    let Some(action) = action else {
        return ReviewDecision {
            approved: false,
            reasons: vec!["No planned action".to_string()],
            action: None,
        };
    };

    match action.capability {
        CapabilityName::ProbeStream => {
            if !input.permit_probe_stream {
                reasons.push("probe_stream not permitted by reviewer policy".to_string());
            }
            if action.probe_stream.is_none() {
                reasons.push("probeStream input is required".to_string());
            }
        }
        CapabilityName::VerifySessionCookieFlags => {
            if !input.permit_verify_session_cookie_flags {
                reasons.push("verify_session_cookie_flags not permitted by reviewer policy".to_string());
            }
            if action.verify_session_cookie_flags.is_none() {
                reasons.push("verifySessionCookieFlags input is required".to_string());
            }
        }
        _ => reasons.push("Only probe_stream and verify_session_cookie_flags are allowed in minimal adapter".to_string()),
    }

    let approved = reasons.is_empty();
    ReviewDecision {
        approved,
        reasons,
        action: approved.then_some(action),
    }
}

fn summarize_planner(planned: &PlannerOutput) -> PlannerDecisionSummary {
    let primary = planned.actions.first();
    PlannerDecisionSummary {
        action_count: planned.actions.len(),
        primary_capability: primary.map(|a| a.capability.clone()),
        rationale: primary.map(|a| a.rationale.clone()),
        confidence: primary.map(|a| a.confidence),
    }
}

fn summarize_reviewer(review: &ReviewDecision) -> ReviewerVerdictSummary {
    ReviewerVerdictSummary {
        approved: review.approved,
        reasons: review.reasons.clone(),
    }
}

fn summarize_capability_result(result: &CapabilityResult) -> CapabilityResultSummary {
    let (alive, secure, issues_count) = match &result.data {
        Some(CapabilityResultData::ProbeStream(out)) => Some(out.alive),
        _ => None
    }
    .map(|v| (Some(v), None, None))
    .unwrap_or_else(|| match &result.data {
        Some(CapabilityResultData::VerifySessionCookieFlags(out)) => {
            (None, Some(out.secure), Some(out.issues.len()))
        }
        _ => (None, None, None),
    });

    CapabilityResultSummary {
        ok: result.ok,
        alive,
        secure,
        issues_count,
        error_code: result.error.as_ref().map(|e| e.code.clone()),
        error_message: result.error.as_ref().map(|e| e.message.clone()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMinimalRequest {
    pub planner: PlannerInput,
    pub permit_probe_stream: bool,
    pub permit_verify_session_cookie_flags: bool,
}

#[tauri::command]
pub async fn run_agent_minimal(
    req: AgentMinimalRequest,
    stream_state: State<'_, crate::StreamState>,
    log_state: State<'_, crate::LogState>,
) -> Result<AgentMinimalTraceEnvelope, String> {
    let agent_run_id = make_agent_run_id();

    crate::push_runtime_log(
        &log_state,
        format!(
            "AGENT_MINIMAL|runId={} target={} mode={:?}",
            agent_run_id, req.planner.target_id, req.planner.mode
        ),
    );

    let target_id = req.planner.target_id.clone();
    let mode = req.planner.mode.clone();
    let planned = planner(req.planner);
    let planner_decision = summarize_planner(&planned);

    let review = reviewer(ReviewerInput {
        actions: planned.actions,
        permit_probe_stream: req.permit_probe_stream,
        permit_verify_session_cookie_flags: req.permit_verify_session_cookie_flags,
    });
    let reviewer_verdict = summarize_reviewer(&review);

    if !review.approved {
        return Ok(AgentMinimalTraceEnvelope {
            agent_run_id,
            target_id,
            mode,
            planner_decision,
            reviewer_verdict,
            capability_invoked: None,
            capability_args_summary: None,
            capability_result_summary: None,
            evidence_refs: vec![],
            reporter_summary: "Planner->Reviewer rejected action; execution skipped".to_string(),
            final_status: MinimalAgentFinalStatus::ReviewerRejected,
            capability_result: None,
        });
    }

    let action = review
        .action
        .clone()
        .ok_or_else(|| "review approved but action missing".to_string())?;

    let capability_req = CapabilityRequest {
        capability: action.capability.clone(),
        mode: action.mode,
        probe_stream: action.probe_stream.clone(),
        search_archive_records: None,
        verify_session_cookie_flags: action.verify_session_cookie_flags.clone(),
    };

    let capability_args_summary = CapabilityArgsSummary {
        target_id: action.probe_stream.as_ref().map(|p| p.target_id.clone()),
        ip_or_url: action
            .verify_session_cookie_flags
            .as_ref()
            .map(|v| v.ip_or_url.clone()),
    };

    let result = capability_adapter::execute_capability(capability_req, stream_state, log_state.clone()).await?;
    let capability_result_summary = summarize_capability_result(&result);

    let (reporter_summary, evidence_refs, final_status) = match &result.data {
        Some(CapabilityResultData::ProbeStream(out)) if result.ok => (
            format!(
                "Planner->Reviewer->Execute completed for target {} (alive={})",
                out.target_id, out.alive
            ),
            out.evidence_refs.clone(),
            MinimalAgentFinalStatus::CapabilitySucceeded,
        ),
        Some(CapabilityResultData::VerifySessionCookieFlags(out)) if result.ok => (
            format!(
                "Planner->Reviewer->Execute completed for endpoint {} (secure={} issues={})",
                out.ip_or_url,
                out.secure,
                out.issues.len()
            ),
            out.evidence_refs.clone(),
            MinimalAgentFinalStatus::CapabilitySucceeded,
        ),
        _ => (
            "Planner->Reviewer approved, but capability execution failed or returned unexpected payload"
                .to_string(),
            vec![],
            MinimalAgentFinalStatus::CapabilityFailed,
        ),
    };

    crate::push_runtime_log(
        &log_state,
        format!(
            "AGENT_MINIMAL|runId={} status={:?} capability={:?} ok={}",
            agent_run_id,
            final_status,
            action.capability,
            result.ok
        ),
    );

    Ok(AgentMinimalTraceEnvelope {
        agent_run_id,
        target_id,
        mode,
        planner_decision,
        reviewer_verdict,
        capability_invoked: Some(action.capability),
        capability_args_summary: Some(capability_args_summary),
        capability_result_summary: Some(capability_result_summary),
        evidence_refs,
        reporter_summary,
        final_status,
        capability_result: Some(result),
    })
}
