use super::*;

#[tauri::command]
pub async fn probe_rtsp_path(host: String, login: String, pass: String) -> Result<String, String> {
    crate::fuzzer::probe_rtsp_path(host, login, pass).await
}

#[tauri::command]
pub async fn start_hub_stream(
    target_id: String,
    user_id: String,
    channel_id: String,
    cookie: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    super::start_hub_stream(target_id, user_id, channel_id, cookie, state, log_state).await
}

#[tauri::command]
pub async fn start_stream(
    target_id: String,
    rtsp_url: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    super::start_stream(target_id, rtsp_url, state, log_state).await
}

#[tauri::command]
pub fn check_stream_alive(
    target_id: String,
    state: State<'_, StreamState>,
) -> Result<bool, String> {
    super::check_stream_alive(target_id, state)
}

#[tauri::command]
pub async fn restart_stream(
    target_id: String,
    rtsp_url: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    super::restart_stream(target_id, rtsp_url, state, log_state).await
}

#[tauri::command]
pub fn stop_stream(
    target_id: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    super::stop_stream(target_id, state, log_state)
}
