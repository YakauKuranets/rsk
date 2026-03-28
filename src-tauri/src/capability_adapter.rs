use serde::{Deserialize, Serialize};
use tauri::State;

use crate::core_types::WorkflowMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityName {
    ProbeStream,
    SearchArchiveRecords,
    VerifySessionCookieFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeStreamInput {
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchArchiveRecordsInput {
    pub camera_ip: String,
    pub login: String,
    pub password: String,
    pub date_from: String,
    pub date_to: String,
    pub channel: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifySessionCookieFlagsInput {
    pub ip_or_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRequest {
    pub capability: CapabilityName,
    pub mode: WorkflowMode,
    pub probe_stream: Option<ProbeStreamInput>,
    pub search_archive_records: Option<SearchArchiveRecordsInput>,
    pub verify_session_cookie_flags: Option<VerifySessionCookieFlagsInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeStreamOutput {
    pub target_id: String,
    pub alive: bool,
    pub allowed_modes: Vec<WorkflowMode>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchArchiveRecordsOutput {
    pub camera_ip: String,
    pub protocol_used: String,
    pub records_found: usize,
    pub total_duration_secs: u64,
    pub allowed_modes: Vec<WorkflowMode>,
    pub errors: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifySessionCookieFlagsOutput {
    pub ip_or_url: String,
    pub secure: bool,
    pub issues: Vec<String>,
    pub allowed_modes: Vec<WorkflowMode>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CapabilityResultData {
    ProbeStream(ProbeStreamOutput),
    SearchArchiveRecords(SearchArchiveRecordsOutput),
    VerifySessionCookieFlags(VerifySessionCookieFlagsOutput),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityResult {
    pub ok: bool,
    pub capability: CapabilityName,
    pub mode: WorkflowMode,
    pub data: Option<CapabilityResultData>,
    pub error: Option<CapabilityError>,
}

fn allowed_modes(capability: &CapabilityName) -> Vec<WorkflowMode> {
    match capability {
        CapabilityName::ProbeStream => {
            vec![WorkflowMode::DiscoveryMode, WorkflowMode::VerifiedMode]
        }
        CapabilityName::SearchArchiveRecords => {
            vec![WorkflowMode::VerifiedMode, WorkflowMode::AnalysisMode]
        }
        CapabilityName::VerifySessionCookieFlags => vec![
            WorkflowMode::DiscoveryMode,
            WorkflowMode::VerifiedMode,
            WorkflowMode::AnalysisMode,
        ],
    }
}

fn validate_mode(capability: &CapabilityName, mode: &WorkflowMode) -> Result<(), CapabilityError> {
    let allowed = allowed_modes(capability);
    if allowed.contains(mode) {
        Ok(())
    } else {
        Err(CapabilityError {
            code: "mode_not_allowed".to_string(),
            message: format!("Mode is not allowed for capability: {:?}", capability),
        })
    }
}

fn fail(
    capability: CapabilityName,
    mode: WorkflowMode,
    code: &str,
    message: String,
) -> CapabilityResult {
    CapabilityResult {
        ok: false,
        capability,
        mode,
        data: None,
        error: Some(CapabilityError {
            code: code.to_string(),
            message,
        }),
    }
}

#[tauri::command]
pub async fn execute_capability(
    req: CapabilityRequest,
    stream_state: State<'_, crate::StreamState>,
    log_state: State<'_, crate::LogState>,
) -> Result<CapabilityResult, String> {
    if let Err(err) = validate_mode(&req.capability, &req.mode) {
        return Ok(CapabilityResult {
            ok: false,
            capability: req.capability,
            mode: req.mode,
            data: None,
            error: Some(err),
        });
    }

    match req.capability.clone() {
        CapabilityName::ProbeStream => {
            let Some(input) = req.probe_stream.as_ref() else {
                return Ok(fail(
                    req.capability,
                    req.mode,
                    "validation_error",
                    "probeStream input is required".to_string(),
                ));
            };
            if input.target_id.trim().is_empty() {
                return Ok(fail(
                    req.capability,
                    req.mode,
                    "validation_error",
                    "targetId is empty".to_string(),
                ));
            }
            let alive = crate::streaming::check_stream_alive(input.target_id.clone(), stream_state)
                .map_err(|e| format!("probe_stream failed: {}", e))?;

            let out = ProbeStreamOutput {
                target_id: input.target_id.clone(),
                alive,
                allowed_modes: allowed_modes(&CapabilityName::ProbeStream),
                evidence_refs: vec![format!("stream_state:{}", input.target_id)],
            };

            let response = CapabilityResult {
                ok: true,
                capability: req.capability.clone(),
                mode: req.mode.clone(),
                data: Some(CapabilityResultData::ProbeStream(out)),
                error: None,
            };
            crate::graph_writer::enqueue_capability_dual_write(&req, &response, &log_state);
            Ok(response)
        }
        CapabilityName::SearchArchiveRecords => {
            let Some(input) = req.search_archive_records.as_ref() else {
                return Ok(fail(
                    req.capability,
                    req.mode,
                    "validation_error",
                    "searchArchiveRecords input is required".to_string(),
                ));
            };
            if input.camera_ip.trim().is_empty()
                || input.login.trim().is_empty()
                || input.password.trim().is_empty()
            {
                return Ok(fail(
                    req.capability,
                    req.mode,
                    "validation_error",
                    "cameraIp/login/password are required".to_string(),
                ));
            }

            let result = crate::unified_archive::search_archive_unified(
                input.camera_ip.clone(),
                input.login.clone(),
                input.password.clone(),
                input.date_from.clone(),
                input.date_to.clone(),
                input.channel,
                log_state.clone(),
            )
            .await
            .map_err(|e| format!("search_archive_records failed: {}", e))?;

            let out = SearchArchiveRecordsOutput {
                camera_ip: result.camera_ip.clone(),
                protocol_used: result.protocol_used.clone(),
                records_found: result.records.len(),
                total_duration_secs: result.total_duration_secs,
                allowed_modes: allowed_modes(&CapabilityName::SearchArchiveRecords),
                errors: result.errors.clone(),
                evidence_refs: vec![
                    format!("archive_protocol:{}", result.protocol_used),
                    format!("archive_records:{}", result.records.len()),
                ],
            };

            let response = CapabilityResult {
                ok: true,
                capability: req.capability.clone(),
                mode: req.mode.clone(),
                data: Some(CapabilityResultData::SearchArchiveRecords(out)),
                error: None,
            };
            crate::graph_writer::enqueue_capability_dual_write(&req, &response, &log_state);
            Ok(response)
        }
        CapabilityName::VerifySessionCookieFlags => {
            let Some(input) = req.verify_session_cookie_flags.as_ref() else {
                return Ok(fail(
                    req.capability,
                    req.mode,
                    "validation_error",
                    "verifySessionCookieFlags input is required".to_string(),
                ));
            };
            if input.ip_or_url.trim().is_empty() {
                return Ok(fail(
                    req.capability,
                    req.mode,
                    "validation_error",
                    "ipOrUrl is empty".to_string(),
                ));
            }

            let audit = crate::session_checker::check_session_security(input.ip_or_url.clone())
                .await
                .map_err(|e| format!("verify_session_cookie_flags failed: {}", e))?;

            let issues = if audit.contains("выглядят безопасно") {
                Vec::new()
            } else {
                audit
                    .replace("[SESSION_AUDIT] ", "")
                    .split(" | ")
                    .map(|s| s.to_string())
                    .collect()
            };

            let out = VerifySessionCookieFlagsOutput {
                ip_or_url: input.ip_or_url.clone(),
                secure: issues.is_empty(),
                issues,
                allowed_modes: allowed_modes(&CapabilityName::VerifySessionCookieFlags),
                evidence_refs: vec![format!("session_audit:{}", input.ip_or_url)],
            };

            let response = CapabilityResult {
                ok: true,
                capability: req.capability.clone(),
                mode: req.mode.clone(),
                data: Some(CapabilityResultData::VerifySessionCookieFlags(out)),
                error: None,
            };
            crate::graph_writer::enqueue_capability_dual_write(&req, &response, &log_state);
            Ok(response)
        }
    }
}
