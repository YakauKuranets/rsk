use crate::*;

#[tauri::command]
pub fn cancel_download_task(
    task_id: String,
    cancel_state: State<'_, DownloadCancelState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    crate::cancel_download_task(task_id, cancel_state, log_state)
}

#[tauri::command]
pub async fn probe_archive_export_endpoints(
    host: String,
    login: String,
    pass: String,
    log_state: State<'_, LogState>,
) -> Result<Vec<ArchiveEndpointResult>, String> {
    crate::probe_archive_export_endpoints(host, login, pass, log_state).await
}

#[tauri::command]
pub async fn download_isapi_playback_uri(
    playback_uri: String,
    login: String,
    pass: String,
    source_host: Option<String>,
    filename_hint: Option<String>,
    task_id: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
    ffmpeg_limiter: State<'_, FfmpegLimiterState>,
) -> Result<DownloadReport, String> {
    crate::download_isapi_playback_uri(
        playback_uri,
        login,
        pass,
        source_host,
        filename_hint,
        task_id,
        log_state,
        cancel_state,
        ffmpeg_limiter,
    )
    .await
}

#[tauri::command]
pub async fn start_archive_export_job(
    playback_uri: String,
    login: String,
    pass: String,
    source_host: Option<String>,
    filename_hint: Option<String>,
    task_id: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
    ffmpeg_limiter: State<'_, FfmpegLimiterState>,
) -> Result<ArchiveExportJobResult, String> {
    crate::start_archive_export_job(
        playback_uri,
        login,
        pass,
        source_host,
        filename_hint,
        task_id,
        log_state,
        cancel_state,
        ffmpeg_limiter,
    )
    .await
}
