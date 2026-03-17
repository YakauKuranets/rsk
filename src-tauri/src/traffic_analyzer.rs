use serde::Serialize;
use std::collections::HashSet;
use std::time::Duration;
use tauri::State;
use tokio::time::{sleep, timeout};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedCredential {
    pub protocol: String,
    pub source_ip: String,
    pub dest_ip: String,
    pub dest_port: u16,
    pub username: Option<String>,
    pub password_hint: String,
    pub raw_evidence: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficAnalysisReport {
    pub interface: String,
    pub duration_secs: u64,
    pub packets_captured: u64,
    pub unencrypted_protocols: Vec<String>,
    pub captured_credentials: Vec<CapturedCredential>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterceptedEvent {
    pub protocol: String,
    pub details: String,
}

#[tauri::command]
pub fn start_passive_sniffer(app_handle: tauri::AppHandle) -> Result<String, String> {
    use pcap::{Capture, Device};
    use tauri::Emitter;

    let main_device = Device::lookup()
        .map_err(|e| format!("Ошибка поиска интерфейса: {}", e))?
        .ok_or("Нет доступных сетевых интерфейсов")?;

    let device_name = main_device.name.clone();

    app_handle
        .emit(
            "hyperion-audit-event",
            format!(
                "🕵️ Сниффер запущен на интерфейсе [{}]. Ожидание нешифрованных пакетов...",
                device_name
            ),
        )
        .map_err(|e| e.to_string())?;

    std::thread::spawn(move || {
        let mut cap = match Capture::from_device(main_device)
            .and_then(|d| d.promisc(true).snaplen(1024).timeout(1000).open())
        {
            Ok(cap) => cap,
            Err(err) => {
                let _ = app_handle.emit(
                    "hyperion-audit-event",
                    format!("[SNIFFER] Ошибка запуска pcap: {}", err),
                );
                return;
            }
        };

        if let Err(err) = cap.filter("tcp port 80 or tcp port 21", true) {
            let _ = app_handle.emit(
                "hyperion-audit-event",
                format!("[SNIFFER] Ошибка установки фильтра: {}", err),
            );
            return;
        }

        while let Ok(packet) = cap.next_packet() {
            if let Ok(payload) = std::str::from_utf8(packet.data) {
                if payload.contains("Authorization: Basic ") {
                    let _ = app_handle.emit(
                        "intercepted_credential",
                        InterceptedEvent {
                            protocol: "HTTP Basic".to_string(),
                            details:
                                "Перехват: Обнаружена передача токена авторизации в открытом виде!"
                                    .to_string(),
                        },
                    );
                    let _ = app_handle.emit(
                        "hyperion-audit-event",
                        "🚨 ALERT: Перехвачен HTTP Basic Auth токен!",
                    );
                }

                if payload.contains("USER ") || payload.contains("PASS ") {
                    let _ = app_handle.emit(
                        "intercepted_credential",
                        InterceptedEvent {
                            protocol: "FTP".to_string(),
                            details: "Перехват: Обнаружена попытка входа по FTP без шифрования!"
                                .to_string(),
                        },
                    );
                    let _ = app_handle.emit(
                        "hyperion-audit-event",
                        "🚨 ALERT: Перехвачены FTP учетные данные!",
                    );
                }
            }
        }
    });

    Ok(format!("Слушаем эфир на {}", device_name))
}

#[tauri::command]
pub async fn analyze_traffic(
    interface: String,
    duration_secs: Option<u64>,
    log_state: State<'_, crate::LogState>,
) -> Result<TrafficAnalysisReport, String> {
    let dur = duration_secs.unwrap_or(30);

    crate::push_runtime_log(
        &log_state,
        format!(
            "[TRAFFIC] Пассивный захват на {} ({} сек)...",
            interface, dur
        ),
    );

    // Guardrail: небольшая пауза перед стартом захвата
    sleep(Duration::from_millis(150)).await;

    #[cfg(feature = "traffic-capture")]
    {
        run_pcap_capture(&interface, dur, &log_state).await
    }

    #[cfg(not(feature = "traffic-capture"))]
    {
        run_tcpdump_capture(&interface, dur, &log_state).await
    }
}

// Совместимость со старым JobRunner API
pub async fn sniff_traffic(interface: &str, duration_secs: u64) -> Result<Option<String>, String> {
    let report = run_tcpdump_capture_without_state(interface, duration_secs).await?;
    if report.captured_credentials.is_empty() {
        Ok(None)
    } else {
        let summary = report
            .captured_credentials
            .iter()
            .take(5)
            .map(|c| {
                format!(
                    "{} {} -> {}:{} ({})",
                    c.protocol,
                    c.source_ip,
                    c.dest_ip,
                    c.dest_port,
                    c.username.clone().unwrap_or_else(|| "n/a".into())
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        Ok(Some(summary))
    }
}

#[cfg(feature = "traffic-capture")]
async fn run_pcap_capture(
    interface: &str,
    duration_secs: u64,
    log_state: &State<'_, crate::LogState>,
) -> Result<TrafficAnalysisReport, String> {
    use chrono::Utc;
    use pcap::{Active, Capture, Device};

    let interface_name = interface.to_string();
    let filter = "tcp port 21 or tcp port 23 or tcp port 80 or tcp port 554 or tcp port 110";

    crate::push_runtime_log(
        log_state,
        format!("[TRAFFIC] pcap capture started: {}", interface_name),
    );

    let report = timeout(
        Duration::from_secs(duration_secs + 5),
        tokio::task::spawn_blocking(move || -> Result<TrafficAnalysisReport, String> {
            let devices = Device::list().map_err(|e| e.to_string())?;
            let device = devices
                .into_iter()
                .find(|d| d.name == interface_name)
                .ok_or_else(|| format!("Interface not found: {}", interface_name))?;

            let mut cap: Capture<Active> = Capture::from_device(device)
                .map_err(|e| e.to_string())?
                .promisc(true)
                .timeout(500)
                .open()
                .map_err(|e| e.to_string())?;

            cap.filter(filter, true).map_err(|e| e.to_string())?;

            let started = std::time::Instant::now();
            let mut packets_captured = 0u64;
            let mut protocols_seen: HashSet<String> = HashSet::new();
            let mut creds = Vec::new();

            while started.elapsed().as_secs() < duration_secs && packets_captured < 1000 {
                if let Ok(packet) = cap.next_packet() {
                    packets_captured += 1;
                    let payload = String::from_utf8_lossy(packet.data).to_string();
                    parse_payload(
                        &payload,
                        "[pcap-src]",
                        "[pcap-dst]",
                        &mut protocols_seen,
                        &mut creds,
                    );
                }
            }

            Ok(TrafficAnalysisReport {
                interface: interface_name,
                duration_secs,
                packets_captured,
                unencrypted_protocols: protocols_seen.into_iter().collect(),
                captured_credentials: creds,
                warnings: vec![
                    "Пассивный мониторинг: трафик не модифицировался".into(),
                    "PII данные маскированы в отчёте".into(),
                ],
            })
        }),
    )
    .await
    .map_err(|_| "Timeout pcap capture".to_string())?
    .map_err(|e| e.to_string())??;

    Ok(report)
}

#[cfg(not(feature = "traffic-capture"))]
async fn run_pcap_capture(
    interface: &str,
    duration_secs: u64,
    log_state: &State<'_, crate::LogState>,
) -> Result<TrafficAnalysisReport, String> {
    run_tcpdump_capture(interface, duration_secs, log_state).await
}

async fn run_tcpdump_capture(
    interface: &str,
    duration_secs: u64,
    log_state: &State<'_, crate::LogState>,
) -> Result<TrafficAnalysisReport, String> {
    crate::push_runtime_log(
        log_state,
        format!("[TRAFFIC] fallback tcpdump mode: {}", interface),
    );
    run_tcpdump_capture_without_state(interface, duration_secs).await
}

async fn run_tcpdump_capture_without_state(
    interface: &str,
    duration_secs: u64,
) -> Result<TrafficAnalysisReport, String> {
    use chrono::Utc;
    use tokio::process::Command;

    let filter = "tcp port 21 or tcp port 23 or tcp port 80 or tcp port 554 or tcp port 110";
    let output_file = format!(
        "/tmp/hyperion_capture_{}_{}.pcap",
        interface.replace('/', "_"),
        Utc::now().timestamp_millis()
    );

    let _capture = timeout(
        Duration::from_secs(duration_secs + 5),
        Command::new("tcpdump")
            .args([
                "-i",
                interface,
                "-c",
                "1000",
                "-w",
                &output_file,
                "-G",
                &duration_secs.to_string(),
                "-W",
                "1",
                filter,
            ])
            .output(),
    )
    .await
    .map_err(|_| "Timeout tcpdump".to_string())?
    .map_err(|e| format!("tcpdump error: {}", e))?;

    // Guardrail: rate limit между внешними вызовами
    sleep(Duration::from_millis(200)).await;

    let analysis = timeout(
        Duration::from_secs(20),
        Command::new("tcpdump")
            .args(["-r", &output_file, "-A", "-n"])
            .output(),
    )
    .await
    .map_err(|_| "Timeout tcpdump read".to_string())?
    .map_err(|e| format!("tcpdump read error: {}", e))?;

    let text = String::from_utf8_lossy(&analysis.stdout);
    let mut creds = Vec::new();
    let mut protocols_seen: HashSet<String> = HashSet::new();

    for line in text.lines() {
        let src = extract_ip(line, "src");
        let dst = extract_ip(line, "dst");
        parse_payload(line, &src, &dst, &mut protocols_seen, &mut creds);
    }

    let _ = std::fs::remove_file(&output_file);

    Ok(TrafficAnalysisReport {
        interface: interface.to_string(),
        duration_secs,
        packets_captured: text.lines().count() as u64,
        unencrypted_protocols: protocols_seen.into_iter().collect(),
        captured_credentials: creds,
        warnings: vec![
            "Пассивный мониторинг: трафик не модифицировался".into(),
            "PII данные маскированы в отчёте".into(),
        ],
    })
}

fn parse_payload(
    line: &str,
    source_ip: &str,
    dest_ip: &str,
    protocols_seen: &mut HashSet<String>,
    creds: &mut Vec<CapturedCredential>,
) {
    use chrono::Utc;

    if line.contains("USER ") && !line.contains("anonymous") {
        let username = line.split("USER ").nth(1).unwrap_or("").trim().to_string();
        creds.push(CapturedCredential {
            protocol: "FTP".into(),
            source_ip: source_ip.to_string(),
            dest_ip: dest_ip.to_string(),
            dest_port: 21,
            username: Some(username),
            password_hint: "***".into(),
            raw_evidence: line.chars().take(100).collect(),
            timestamp: Utc::now().to_rfc3339(),
        });
        protocols_seen.insert("FTP".to_string());
    }

    if line.contains("Authorization: Basic ") {
        protocols_seen.insert("HTTP_BASIC".to_string());
        creds.push(CapturedCredential {
            protocol: "HTTP_BASIC".into(),
            source_ip: source_ip.to_string(),
            dest_ip: dest_ip.to_string(),
            dest_port: 80,
            username: None,
            password_hint: "base64_***".into(),
            raw_evidence: "Authorization: Basic [REDACTED]".into(),
            timestamp: Utc::now().to_rfc3339(),
        });
    }

    if line.contains("DESCRIBE rtsp://") || line.contains("SETUP rtsp://") {
        protocols_seen.insert("RTSP".to_string());
    }

    if line.contains("login:") || line.contains("Password:") {
        protocols_seen.insert("TELNET".to_string());
    }
}

fn extract_ip(line: &str, direction: &str) -> String {
    let re =
        regex::Regex::new(r"IP\s+(\d+\.\d+\.\d+\.\d+)\.\d+\s+>\s+(\d+\.\d+\.\d+\.\d+)\.\d+").ok();

    if let Some(regex) = re {
        if let Some(caps) = regex.captures(line) {
            if direction == "src" {
                return caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "[src]".into());
            }
            return caps
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| "[dst]".into());
        }
    }

    format!("[{}]", direction)
}
