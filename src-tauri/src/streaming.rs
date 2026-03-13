use super::*;

#[tauri::command]
pub async fn probe_rtsp_path(host: String, login: String, pass: String) -> Result<String, String> {
    probe_rtsp_path_impl(host, login, pass).await
}

#[tauri::command]
pub fn start_hub_stream(
    target_id: String,
    user_id: String,
    channel_id: String,
    cookie: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    start_hub_stream_impl(target_id, user_id, channel_id, cookie, state, log_state)
}

#[tauri::command]
pub fn start_stream(
    target_id: String,
    rtsp_url: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    start_stream_impl(target_id, rtsp_url, state, log_state)
}

#[tauri::command]
pub fn check_stream_alive(
    target_id: String,
    state: State<'_, StreamState>,
) -> Result<bool, String> {
    check_stream_alive_impl(target_id, state)
}

#[tauri::command]
pub fn restart_stream(
    target_id: String,
    rtsp_url: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    restart_stream_impl(target_id, rtsp_url, state, log_state)
}

#[tauri::command]
pub fn stop_stream(
    target_id: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    stop_stream_impl(target_id, state, log_state)
}
