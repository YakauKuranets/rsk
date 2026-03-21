use serde::{Deserialize, Serialize};
use tauri::State;

use crate::capability_adapter::{
    self, CapabilityName, CapabilityResult, CapabilityResultData, CapabilityRequest, ProbeStreamInput,
};
use crate::core_types::WorkflowMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerInput {
    pub target_id: String,
    pub mode: WorkflowMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedAction {
    pub capability: CapabilityName,
    pub mode: WorkflowMode,
    pub rationale: String,
    pub confidence: f32,
    pub probe_stream: Option<ProbeStreamInput>,
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
pub struct ReporterOutput {
    pub summary: String,
    pub evidence_refs: Vec<String>,
    pub capability_result: Option<CapabilityResult>,
    pub review: ReviewDecision,
}

fn planner(input: PlannerInput) -> PlannerOutput {
    PlannerOutput {
        actions: vec![PlannedAction {
            capability: CapabilityName::ProbeStream,
            mode: input.mode,
            rationale: "Probe stream availability for initial liveness signal".to_string(),
            confidence: 0.8,
            probe_stream: Some(ProbeStreamInput {
                target_id: input.target_id,
            }),
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

    if !input.permit_probe_stream {
        reasons.push("probe_stream not permitted by reviewer policy".to_string());
    }

    if !matches!(action.capability, CapabilityName::ProbeStream) {
        reasons.push("Only probe_stream capability is allowed in minimal adapter".to_string());
    }

    if action.probe_stream.is_none() {
        reasons.push("probeStream input is required".to_string());
    }

    let approved = reasons.is_empty();
    ReviewDecision {
        approved,
        reasons,
        action: approved.then_some(action),
    }
}

fn reporter(review: ReviewDecision, result: Option<CapabilityResult>) -> ReporterOutput {
    let mut evidence_refs = Vec::new();
    let summary = if let Some(res) = &result {
        if let Some(CapabilityResultData::ProbeStream(out)) = &res.data {
            evidence_refs.extend(out.evidence_refs.clone());
            format!(
                "Planner->Reviewer->Execute completed for target {} (alive={})",
                out.target_id, out.alive
            )
        } else {
            "Planner->Reviewer approved, but capability returned no probe_stream payload".to_string()
        }
    } else {
        "Planner->Reviewer rejected action; execution skipped".to_string()
    };

    ReporterOutput {
        summary,
        evidence_refs,
        capability_result: result,
        review,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMinimalRequest {
    pub planner: PlannerInput,
    pub permit_probe_stream: bool,
}

#[tauri::command]
pub async fn run_agent_minimal(
    req: AgentMinimalRequest,
    stream_state: State<'_, crate::StreamState>,
    log_state: State<'_, crate::LogState>,
) -> Result<ReporterOutput, String> {
    crate::push_runtime_log(
        &log_state,
        format!(
            "AGENT_MINIMAL|target={} mode={:?}",
            req.planner.target_id, req.planner.mode
        ),
    );

    let planned = planner(req.planner);
    let review = reviewer(ReviewerInput {
        actions: planned.actions,
        permit_probe_stream: req.permit_probe_stream,
    });

    if !review.approved {
        return Ok(reporter(review, None));
    }

    let action = review
        .action
        .clone()
        .ok_or_else(|| "review approved but action missing".to_string())?;

    let capability_req = CapabilityRequest {
        capability: action.capability,
        mode: action.mode,
        probe_stream: action.probe_stream,
        search_archive_records: None,
        verify_session_cookie_flags: None,
    };

    let result = capability_adapter::execute_capability(capability_req, stream_state, log_state).await?;
    Ok(reporter(review, Some(result)))
}
