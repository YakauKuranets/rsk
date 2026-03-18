// src-tauri/src/agents/risk_agent.rs
use crate::agents::handoff::{AgentId, HandoffPacket, HandoffStatus};
use crate::threat_intel::build_threat_report;
use chrono::Utc;
use serde_json::json;
use tauri::State;

#[tauri::command]
pub async fn run_risk_agent(
    packet: HandoffPacket,
    log_state: State<'_, crate::LogState>,
) -> Result<HandoffPacket, String> {
    let report = build_threat_report(&packet);
    let max_score = report
        .profiles
        .iter()
        .map(|p| p.threat_score)
        .fold(0.0_f32, f32::max);

    let mut indicators = packet.risk_indicators.clone();
    indicators.push(format!("maxThreatScore:{:.1}", max_score));
    indicators.push(format!("criticalHosts:{}", report.critical_count));
    indicators.push(format!("kevFindings:{}", report.total_kev));
    indicators.push(format!("topTactic:{}", report.top_tactic));

    let mut ctx = packet.context_carry.clone();
    ctx["risk_report"] = json!({
        "profiles": report.profiles,
        "critical_count": report.critical_count,
        "total_kev": report.total_kev,
        "top_tactic": report.top_tactic,
        "max_score": max_score,
    });

    crate::push_runtime_log(
        &log_state,
        format!(
            "RISK_DONE|pipeline={}|score={:.1}|critical={}",
            packet.pipeline_id, max_score, report.critical_count
        ),
    );

    Ok(HandoffPacket {
        from_agent: AgentId::RiskAgent,
        timestamp_utc: Utc::now().to_rfc3339(),
        status: HandoffStatus::Success,
        risk_indicators: indicators,
        context_carry: ctx,
        ..packet
    })
}
