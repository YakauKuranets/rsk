use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use tauri::State;

use crate::agents::handoff::{AgentId, HandoffPacket, HandoffStatus};

pub struct ScopeGuard {
    pub authorized_ranges: RwLock<Vec<IpNetwork>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeGuardUpdate {
    pub authorized_ranges: Vec<String>,
}

impl ScopeGuard {
    pub fn check(&self, ip: &str) -> Result<(), String> {
        let addr: std::net::IpAddr = ip.parse().map_err(|_| format!("Неверный IP: {}", ip))?;
        let ranges = self
            .authorized_ranges
            .read()
            .map_err(|_| "ScopeGuard lock poisoned".to_string())?;

        let authorized = ranges.iter().any(|range| range.contains(addr));
        if !authorized {
            return Err(format!(
                "SCOPE VIOLATION: {} не входит в авторизованный диапазон!",
                ip
            ));
        }
        Ok(())
    }
}

#[tauri::command]
pub fn set_scope_authorized_ranges(
    update: ScopeGuardUpdate,
    scope_guard: State<'_, ScopeGuard>,
) -> Result<usize, String> {
    let parsed = update
        .authorized_ranges
        .iter()
        .map(|item| {
            item.parse::<IpNetwork>()
                .map_err(|_| format!("Неверный CIDR: {}", item))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let len = parsed.len();
    let mut ranges = scope_guard
        .authorized_ranges
        .write()
        .map_err(|_| "ScopeGuard lock poisoned".to_string())?;
    *ranges = parsed;
    Ok(len)
}

#[tauri::command]
pub async fn run_scan_agent(
    mut packet: HandoffPacket,
    scope_guard: State<'_, ScopeGuard>,
    log_state: State<'_, crate::LogState>,
) -> Result<HandoffPacket, String> {
    for finding in &packet.findings {
        scope_guard.check(&finding.host).map_err(|e| {
            crate::push_runtime_log(&log_state, format!("SCOPE_BLOCK|{}", e));
            e
        })?;
    }

    crate::push_runtime_log(
        &log_state,
        format!(
            "SCAN_START|pipeline={}|targets={}",
            packet.pipeline_id,
            packet.findings.len()
        ),
    );

    packet.from_agent = AgentId::ScanAgent;
    packet.status = HandoffStatus::Success;
    packet
        .risk_indicators
        .push("scopeGuardValidated".to_string());
    Ok(packet)
}
