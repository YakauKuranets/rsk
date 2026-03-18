use tauri::State;

use crate::agents::handoff::HandoffPacket;

#[tauri::command]
pub async fn run_scan_agent(
    packet: HandoffPacket,
    scope_guard: State<'_, crate::scope_guard::ScopeGuard>,
    log_state: State<'_, crate::LogState>,
) -> Result<HandoffPacket, String> {
    crate::scope_guard::run_scan_agent(packet, scope_guard, log_state).await
}
