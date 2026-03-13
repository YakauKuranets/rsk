use super::*;

#[tauri::command]
pub fn cancel_download_task(
    task_id: String,
    cancel_state: State<'_, DownloadCancelState>,
) -> Result<String, String> {
    cancel_download_task_impl(task_id, cancel_state)
}

#[tauri::command]
pub async fn probe_archive_export_endpoints(
    playback_uri: String,
    source_host: Option<String>,
    log_state: State<'_, LogState>,
) -> Result<Vec<ArchiveExportEndpointProbe>, String> {
    probe_archive_export_endpoints_impl(playback_uri, source_host, log_state).await
}

#[tauri::command]
pub async fn download_isapi_playback_uri(
    playback_uri: String,
    login: String,
    pass: String,
    filename_hint: Option<String>,
    task_id: Option<String>,
    source_host: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
    ffmpeg_limiter: State<'_, FfmpegLimiterState>,
) -> Result<DownloadReport, String> {
    download_isapi_playback_uri_impl(
        playback_uri,
        login,
        pass,
        filename_hint,
        task_id,
        source_host,
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
    filename_hint: Option<String>,
    source_host: Option<String>,
    fallback_duration: u64,
    task_id: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
    ffmpeg_limiter: State<'_, FfmpegLimiterState>,
) -> Result<ArchiveExportJobResult, String> {
    start_archive_export_job_impl(
        playback_uri,
        login,
        pass,
        filename_hint,
        source_host,
        fallback_duration,
        task_id,
        log_state,
        cancel_state,
        ffmpeg_limiter,
    )
    .await
}
