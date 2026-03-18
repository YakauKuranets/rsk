use futures::stream::{self, StreamExt};
use serde::Serialize;
use std::time::Duration;
use tauri::State;
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredCamera {
    pub id: String,
    pub ip: String,
    pub open_ports: Vec<PortInfo>,
    pub vendor: String,
    pub vendor_confidence: f32,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub device_name: Option<String>,
    pub serial: Option<String>,
    pub rtsp_paths: Vec<RtspPath>,
    pub archive_protocols: Vec<String>,
    pub admin_url: Option<String>,
    pub ftp_accessible: bool,
    pub onvif_supported: bool,
    pub auth_status: String,
    pub credentials: Option<CameraCredentials>,
    pub scan_timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortInfo {
    pub port: u16,
    pub service: String,
    pub banner: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RtspPath {
    pub url: String,
    pub channel: u32,
    pub substream: bool,
    pub codec: Option<String>,
    pub resolution: Option<String>,
    pub requires_auth: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraCredentials {
    pub login: String,
    pub password: String,
    pub method: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraScanReport {
    pub scan_id: String,
    pub target_input: String,
    pub hosts_scanned: usize,
    pub cameras_found: usize,
    pub cameras: Vec<DiscoveredCamera>,
    pub duration_ms: u64,
    pub phases_completed: Vec<String>,
}

#[tauri::command]
pub async fn unified_camera_scan(
    target_input: String,
    scan_mode: Option<String>,
    max_concurrent: Option<u32>,
    known_login: Option<String>,
    known_password: Option<String>,
    log_state: State<'_, crate::LogState>,
) -> Result<CameraScanReport, String> {
    let start = std::time::Instant::now();
    let mode = scan_mode.unwrap_or_else(|| "normal".into());
    let concurrency = max_concurrent.unwrap_or(20) as usize;
    let scan_id = format!("scan_{}", chrono::Utc::now().timestamp_millis());

    crate::push_runtime_log(
        &log_state,
        format!(
            "[CAMERA_SCAN] Start: target={}, mode={}",
            target_input, mode
        ),
    );

    let ips = expand_target_input(&target_input)?;
    crate::push_runtime_log(
        &log_state,
        format!("[CAMERA_SCAN] Phase 1: {} IPs to scan", ips.len()),
    );

    let camera_ports: Vec<u16> = vec![
        80, 443, 554, 8000, 8080, 8443, 8554, 8899, 2019, 3702, 5000, 9000, 34567, 37777, 21, 23,
    ];

    let alive_hosts: Vec<(String, Vec<PortInfo>)> = stream::iter(ips)
        .map(|ip| {
            let ports = camera_ports.clone();
            async move {
                let open = scan_ports_fast(&ip, &ports).await;
                if open.is_empty() {
                    None
                } else {
                    Some((ip, open))
                }
            }
        })
        .buffer_unordered(concurrency)
        .filter_map(|r| async move { r })
        .collect()
        .await;

    crate::push_runtime_log(
        &log_state,
        format!("[CAMERA_SCAN] Phase 2: {} alive hosts", alive_hosts.len()),
    );

    let mut phases = vec!["host_discovery".into(), "port_scan".into()];
    let mut cameras: Vec<DiscoveredCamera> = Vec::new();

    for (ip, ports) in &alive_hosts {
        let mut camera = DiscoveredCamera {
            id: format!("cam_{}", ip.replace('.', "_")),
            ip: ip.clone(),
            open_ports: ports.clone(),
            vendor: "Unknown".into(),
            vendor_confidence: 0.0,
            model: None,
            firmware: None,
            device_name: None,
            serial: None,
            rtsp_paths: Vec::new(),
            archive_protocols: Vec::new(),
            admin_url: None,
            ftp_accessible: ports.iter().any(|p| p.port == 21),
            onvif_supported: ports.iter().any(|p| p.port == 3702),
            auth_status: "unknown".into(),
            credentials: None,
            scan_timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let fp = fingerprint_camera(ip, ports).await;
        camera.vendor = fp.vendor;
        camera.vendor_confidence = fp.confidence;
        camera.model = fp.model;
        camera.firmware = fp.firmware;
        camera.device_name = fp.device_name;
        camera.admin_url = fp.admin_url;

        if mode != "fast" && ports.iter().any(|p| p.port == 554) {
            let login = known_login.as_deref().unwrap_or("admin");
            let pass = known_password.as_deref().unwrap_or("");
            camera.rtsp_paths = discover_rtsp_paths(ip, &camera.vendor, login, pass).await;

            if camera.rtsp_paths.is_empty() {
                camera.auth_status = "auth_required".into();
            } else {
                camera.auth_status = "accessible".into();
            }
        }

        if mode == "deep" {
            if camera.auth_status == "auth_required" {
                if let Some(creds) = try_default_credentials(ip, &camera.vendor).await {
                    camera.credentials = Some(creds.clone());
                    camera.auth_status = "weak_credentials".into();
                    camera.rtsp_paths =
                        discover_rtsp_paths(ip, &camera.vendor, &creds.login, &creds.password)
                            .await;
                }
            }

            camera.archive_protocols = detect_archive_protocols(ip, &camera).await;
        }

        cameras.push(camera);
    }

    if mode != "fast" {
        phases.push("fingerprint".into());
        phases.push("rtsp_discovery".into());
    }
    if mode == "deep" {
        phases.push("credential_audit".into());
        phases.push("archive_probe".into());
    }

    crate::push_runtime_log(
        &log_state,
        format!(
            "[CAMERA_SCAN] Complete: {} cameras found in {}ms",
            cameras.len(),
            start.elapsed().as_millis()
        ),
    );

    Ok(CameraScanReport {
        scan_id,
        target_input,
        hosts_scanned: alive_hosts.len(),
        cameras_found: cameras.len(),
        cameras,
        duration_ms: start.elapsed().as_millis() as u64,
        phases_completed: phases,
    })
}

fn expand_target_input(input: &str) -> Result<Vec<String>, String> {
    let trimmed = input.trim();

    if trimmed.parse::<std::net::Ipv4Addr>().is_ok() {
        return Ok(vec![trimmed.to_string()]);
    }

    if trimmed.contains('/') {
        use ipnetwork::IpNetwork;
        let network: IpNetwork = trimmed
            .parse()
            .map_err(|e| format!("Invalid CIDR: {}", e))?;
        let ips: Vec<String> = network.iter().map(|ip| ip.to_string()).collect();
        if ips.len() > 1024 {
            return Err(format!(
                "CIDR слишком большой: {} адресов (макс 1024)",
                ips.len()
            ));
        }
        return Ok(ips);
    }

    if trimmed.contains('-') {
        if let Some((start_str, end_str)) = trimmed.split_once('-') {
            let start_str = start_str.trim();
            let end_str = end_str.trim();

            let end_ip = if end_str.contains('.') {
                end_str.to_string()
            } else if let Some(prefix_end) = start_str.rfind('.') {
                format!("{}.{}", &start_str[..prefix_end], end_str)
            } else {
                return Err("Неверный формат диапазона".into());
            };

            let start: std::net::Ipv4Addr = start_str.parse().map_err(|_| "Неверный start IP")?;
            let end: std::net::Ipv4Addr = end_ip.parse().map_err(|_| "Неверный end IP")?;

            let start_u32 = u32::from(start);
            let end_u32 = u32::from(end);

            if end_u32 < start_u32 || (end_u32 - start_u32) > 1024 {
                return Err("Диапазон слишком большой или неверный".into());
            }

            let ips: Vec<String> = (start_u32..=end_u32)
                .map(|n| std::net::Ipv4Addr::from(n).to_string())
                .collect();
            return Ok(ips);
        }
    }

    if trimmed == "auto" {
        return expand_target_input("192.168.1.0/24");
    }

    Ok(vec![trimmed.to_string()])
}

async fn scan_ports_fast(ip: &str, ports: &[u16]) -> Vec<PortInfo> {
    let mut results = Vec::new();

    for &port in ports {
        let addr = format!("{}:{}", ip, port);
        let is_open = timeout(Duration::from_millis(400), TcpStream::connect(&addr))
            .await
            .is_ok_and(|r| r.is_ok());

        if is_open {
            results.push(PortInfo {
                port,
                service: guess_camera_service(port),
                banner: String::new(),
                state: "open".into(),
            });
        }
    }
    results
}

fn guess_camera_service(port: u16) -> String {
    match port {
        21 => "FTP",
        23 => "Telnet",
        80 => "HTTP",
        443 => "HTTPS",
        554 => "RTSP",
        2019 => "Hikvision-Web",
        3702 => "ONVIF-WSD",
        5000 => "SDK",
        8000 => "Hikvision-SDK",
        8080 => "HTTP-Alt",
        8443 => "HTTPS-Alt",
        8554 => "RTSP-Alt",
        8899 => "XMeye-Alt",
        9000 => "Dahua-HTTP",
        34567 => "XMeye-Protocol",
        37777 => "Dahua-SDK",
        _ => "Unknown",
    }
    .to_string()
}

struct FingerprintResult {
    vendor: String,
    confidence: f32,
    model: Option<String>,
    firmware: Option<String>,
    device_name: Option<String>,
    admin_url: Option<String>,
}

async fn fingerprint_camera(ip: &str, ports: &[PortInfo]) -> FingerprintResult {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        // device client: self-signed certs expected on local network hardware
        .danger_accept_invalid_certs(true)
        .user_agent("Mozilla/5.0")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut server_header = String::new();
    let mut html_body = String::new();
    let mut admin_url = None;

    for web_port in &[80u16, 8080, 2019, 443, 8443, 9000] {
        if !ports.iter().any(|p| p.port == *web_port) {
            continue;
        }
        let scheme = if *web_port == 443 || *web_port == 8443 {
            "https"
        } else {
            "http"
        };
        let url = format!("{}://{}:{}/", scheme, ip, web_port);

        if let Ok(resp) = client.get(&url).send().await {
            if let Some(srv) = resp.headers().get("server").and_then(|v| v.to_str().ok()) {
                server_header = srv.to_string();
            }
            admin_url = Some(url.clone());
            html_body = resp.text().await.unwrap_or_default();
            if !server_header.is_empty() {
                break;
            }
        }
    }

    let mut rtsp_server = String::new();
    if ports.iter().any(|p| p.port == 554) {
        if let Ok(Ok(mut stream)) = timeout(
            Duration::from_secs(2),
            TcpStream::connect(format!("{}:554", ip)),
        )
        .await
        {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let probe = format!("OPTIONS rtsp://{}:554/ RTSP/1.0\r\nCSeq: 1\r\n\r\n", ip);
            let _ = stream.write_all(probe.as_bytes()).await;
            let mut buf = [0u8; 512];
            if let Ok(Ok(n)) = timeout(Duration::from_millis(800), stream.read(&mut buf)).await {
                let txt = String::from_utf8_lossy(&buf[..n]);
                for line in txt.lines() {
                    if line.to_lowercase().starts_with("server:") {
                        rtsp_server = line[7..].trim().to_string();
                    }
                }
            }
        }
    }

    let combined = format!(
        "{} {} {} {}",
        server_header.to_lowercase(),
        rtsp_server.to_lowercase(),
        html_body
            .to_lowercase()
            .chars()
            .take(2000)
            .collect::<String>(),
        ports
            .iter()
            .map(|p| p.port.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );

    let (vendor, confidence, model) = detect_vendor_advanced(&combined, ports);
    let firmware = extract_firmware_from_html(&html_body);
    let device_name = extract_device_name(&html_body);

    FingerprintResult {
        vendor,
        confidence,
        model,
        firmware,
        device_name,
        admin_url,
    }
}

fn detect_vendor_advanced(combined: &str, ports: &[PortInfo]) -> (String, f32, Option<String>) {
    if combined.contains("hikvision")
        || combined.contains("app-webs")
        || combined.contains("davinci")
        || combined.contains("webcomponents")
        || (ports.iter().any(|p| p.port == 8000) && combined.contains("digest"))
    {
        return ("Hikvision".into(), 0.95, extract_model_hikvision(combined));
    }

    if combined.contains("dahua")
        || combined.contains("dss")
        || combined.contains("dhwebclientplugin")
        || ports.iter().any(|p| p.port == 37777)
    {
        return ("Dahua".into(), 0.95, None);
    }

    if combined.contains("goahead")
        || combined.contains("xmeye")
        || ports.iter().any(|p| p.port == 34567)
    {
        return ("XMeye".into(), 0.85, None);
    }

    if combined.contains("axis") || combined.contains("boa") {
        return ("Axis".into(), 0.9, None);
    }

    if combined.contains("reolink") {
        return ("Reolink".into(), 0.9, None);
    }

    if combined.contains("uniview") || combined.contains("unv") {
        return ("Uniview".into(), 0.85, None);
    }

    if combined.contains("trassir") || combined.contains("dssl") {
        return ("Trassir".into(), 0.85, None);
    }

    if ports.iter().any(|p| p.port == 554) {
        return ("Unknown Camera".into(), 0.5, None);
    }

    ("Unknown".into(), 0.0, None)
}

fn extract_model_hikvision(combined: &str) -> Option<String> {
    let re = regex::Regex::new(r"(DS-\w{4,20})").ok()?;
    re.captures(combined).map(|c| c[1].to_string())
}

fn extract_firmware_from_html(html: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)(V?\d+\.\d+\.\d+[\.\-]?\w*\s*(?:build|build\s*\d+)?)").ok()?;
    re.captures(html).map(|c| c[1].to_string())
}

fn extract_device_name(html: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)<title>([^<]{2,60})</title>").ok()?;
    let name = re.captures(html)?.get(1)?.as_str().trim().to_string();
    if name.is_empty() || name.to_lowercase().contains("404") {
        None
    } else {
        Some(name)
    }
}

async fn discover_rtsp_paths(ip: &str, vendor: &str, login: &str, pass: &str) -> Vec<RtspPath> {
    let ffmpeg = crate::get_ffmpeg_path();
    let mut paths = Vec::new();

    let vendor_paths = get_vendor_rtsp_paths(vendor);
    let generic_paths = vec![
        "/live/ch0",
        "/live/ch1",
        "/live",
        "/stream1",
        "/stream2",
        "/cam/realmonitor?channel=1&subtype=0",
        "/cam/realmonitor?channel=1&subtype=1",
        "/h264",
        "/",
    ];

    let all_paths: Vec<&str> = vendor_paths
        .iter()
        .chain(generic_paths.iter())
        .copied()
        .collect();

    for (idx, path) in all_paths.iter().enumerate() {
        let url = if login.is_empty() && pass.is_empty() {
            format!("rtsp://{}:554{}", ip, path)
        } else {
            format!("rtsp://{}:{}@{}:554{}", login, pass, ip, path)
        };

        let probe = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::process::Command::new(&ffmpeg)
                .args(crate::ffmpeg::FfmpegProfiles::probe(&url))
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status(),
        )
        .await;

        if let Ok(Ok(status)) = probe {
            if status.success() {
                let is_substream = path.contains("subtype=1")
                    || path.contains("Channels/102")
                    || path.contains("stream2")
                    || path.contains("/ch0");

                let channel = if path.contains("channel=") {
                    path.split("channel=")
                        .nth(1)
                        .and_then(|s| {
                            s.chars()
                                .take_while(|c| c.is_ascii_digit())
                                .collect::<String>()
                                .parse()
                                .ok()
                        })
                        .unwrap_or(1)
                } else {
                    (idx as u32 / 2) + 1
                };

                paths.push(RtspPath {
                    url,
                    channel,
                    substream: is_substream,
                    codec: None,
                    resolution: None,
                    requires_auth: !login.is_empty(),
                });
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        if paths.len() >= 4 {
            break;
        }
    }

    paths
}

fn get_vendor_rtsp_paths(vendor: &str) -> Vec<&'static str> {
    match vendor.to_lowercase().as_str() {
        v if v.contains("hikvision") => vec![
            "/Streaming/Channels/101",
            "/Streaming/Channels/102",
            "/Streaming/Channels/201",
            "/Streaming/Channels/202",
            "/Streaming/Channels/301",
            "/ISAPI/Streaming/channels/101",
        ],
        v if v.contains("dahua") => vec![
            "/cam/realmonitor?channel=1&subtype=0",
            "/cam/realmonitor?channel=1&subtype=1",
            "/cam/realmonitor?channel=2&subtype=0",
            "/cam/realmonitor?channel=2&subtype=1",
        ],
        v if v.contains("xmeye") => vec![
            "/user=admin&password=&channel=1&stream=0.sdp",
            "/user=admin&password=&channel=1&stream=1.sdp",
        ],
        v if v.contains("axis") => vec![
            "/axis-media/media.amp",
            "/axis-media/media.amp?streamprofile=Quality",
        ],
        v if v.contains("reolink") => vec!["/h264Preview_01_main", "/h264Preview_01_sub"],
        v if v.contains("uniview") || v.contains("unv") => vec!["/media/video1", "/media/video2"],
        _ => vec![],
    }
}

async fn try_default_credentials(ip: &str, vendor: &str) -> Option<CameraCredentials> {
    let ffmpeg = crate::get_ffmpeg_path();

    let cred_list: Vec<(&str, &str)> = match vendor.to_lowercase().as_str() {
        v if v.contains("hikvision") => vec![
            ("admin", "12345"),
            ("admin", "123456"),
            ("admin", "admin"),
            ("admin", "Hik12345"),
        ],
        v if v.contains("dahua") => {
            vec![("admin", "admin"), ("admin", "123456"), ("admin", "888888")]
        }
        v if v.contains("xmeye") => vec![("admin", ""), ("admin", "admin")],
        _ => vec![
            ("admin", "admin"),
            ("admin", "12345"),
            ("admin", ""),
            ("admin", "123456"),
        ],
    };

    let test_path = match vendor.to_lowercase().as_str() {
        v if v.contains("hikvision") => "/Streaming/Channels/102",
        v if v.contains("dahua") => "/cam/realmonitor?channel=1&subtype=1",
        _ => "/cam/realmonitor?channel=1&subtype=1",
    };

    for (login, pass) in cred_list {
        let url = format!("rtsp://{}:{}@{}:554{}", login, pass, ip, test_path);

        let probe = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::process::Command::new(&ffmpeg)
                .args(crate::ffmpeg::FfmpegProfiles::probe(&url))
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status(),
        )
        .await;

        if let Ok(Ok(status)) = probe {
            if status.success() {
                let km = crate::knowledge::KnowledgeManager::new();
                km.save_success(ip, vendor, &url, login, pass);

                return Some(CameraCredentials {
                    login: login.to_string(),
                    password: pass.to_string(),
                    method: "default_creds".into(),
                });
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    None
}

async fn detect_archive_protocols(ip: &str, camera: &DiscoveredCamera) -> Vec<String> {
    let mut protocols = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        // device client: self-signed certs expected on local network hardware
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let (login, pass) = camera
        .credentials
        .as_ref()
        .map(|c| (c.login.as_str(), c.password.as_str()))
        .unwrap_or(("admin", ""));

    let isapi_url = format!("http://{}/ISAPI/ContentMgmt/search", ip);
    if let Ok(resp) = client
        .get(&isapi_url)
        .basic_auth(login, Some(pass))
        .send()
        .await
    {
        if resp.status() != reqwest::StatusCode::NOT_FOUND {
            protocols.push("ISAPI".into());
        }
    }

    if camera.onvif_supported {
        protocols.push("ONVIF".into());
    }

    if camera.ftp_accessible {
        protocols.push("FTP".into());
    }

    let xm_url = format!("http://{}/cgi-bin/magicBox.cgi?action=getSystemInfo", ip);
    if let Ok(resp) = client.get(&xm_url).send().await {
        if resp.status().is_success() {
            protocols.push("XM_CGI".into());
        }
    }

    if camera.vendor.to_lowercase().contains("hikvision") {
        protocols.push("RTSP_PLAYBACK".into());
    }

    protocols
}
