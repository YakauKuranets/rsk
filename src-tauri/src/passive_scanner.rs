use futures::stream::{self, StreamExt};
use ipnetwork::IpNetwork;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::time::Instant;
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IoTDevice {
    pub ip: String,
    pub mac_address: Option<String>,
    pub manufacturer: Option<String>,
    pub device_type: String,
    pub hostname: Option<String>,
    pub open_ports: Vec<u16>,
    pub banners: HashMap<u16, String>,
    pub os_guess: Option<String>,
    pub firmware_version: Option<String>,
    pub known_cves: Vec<KnownCve>,
    pub risk_level: String,
    pub detection_method: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownCve {
    pub cve_id: String,
    pub description: String,
    pub cvss_score: Option<f32>,
    pub affected_versions: String,
    pub reference_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassiveScanReport {
    pub scan_id: String,
    pub scan_type: String,
    pub total_devices: usize,
    pub devices: Vec<IoTDevice>,
    pub medical_devices: Vec<IoTDevice>,
    pub high_risk_count: usize,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
}

fn get_oui_database() -> HashMap<&'static str, &'static str> {
    let mut db = HashMap::new();
    db.insert("00:09:FB", "Philips Medical"); db.insert("00:17:C4", "Siemens Medical"); db.insert("00:1E:8F", "GE Healthcare"); db.insert("00:50:F2", "Draeger Medical"); db.insert("00:80:25", "Becton Dickinson");
    db.insert("28:57:BE", "Hangzhou Hikvision"); db.insert("54:C4:15", "Hangzhou Hikvision"); db.insert("BC:AD:28", "Hangzhou Hikvision"); db.insert("3C:EF:8C", "Dahua Technology"); db.insert("A0:BD:1D", "Dahua Technology"); db.insert("AC:CC:8E", "Axis Communications");
    db.insert("00:1A:2B", "Cisco"); db.insert("00:50:56", "VMware"); db.insert("B8:27:EB", "Raspberry Pi"); db.insert("DC:A6:32", "Raspberry Pi");
    db
}

fn classify_device(manufacturer: &str, banners: &HashMap<u16, String>, open_ports: &[u16]) -> String {
    let m = manufacturer.to_lowercase();
    let b = banners.values().cloned().collect::<Vec<_>>().join(" ").to_lowercase();
    if ["philips medical", "siemens medical", "ge healthcare", "draeger", "becton"].iter().any(|x| m.contains(x)) { return "medical".into(); }
    if ["hikvision", "dahua", "axis"].iter().any(|x| m.contains(x)) { return "camera".into(); }
    if open_ports.contains(&554) || b.contains("rtsp") { return "camera".into(); }
    if open_ports.contains(&9100) || b.contains("printer") || b.contains("hp ") || b.contains("xerox") { return "printer".into(); }
    if open_ports.contains(&502) || open_ports.contains(&47808) { return "plc".into(); }
    if open_ports.contains(&80) && (b.contains("router") || b.contains("mikrotik") || b.contains("routeros")) { return "router".into(); }
    "unknown".into()
}

async fn lookup_cves_for_device(manufacturer: &str, firmware: Option<&str>, _device_type: &str) -> Vec<KnownCve> {
    let query = if let Some(fw) = firmware { format!("{} {}", manufacturer, fw) } else { manufacturer.to_string() };
    let local = crate::vuln_db_updater::query_local_vuln_db(manufacturer.to_string(), query.clone()).await.unwrap_or_default();
    let mut out: Vec<KnownCve> = local.into_iter().take(10).map(|x| {
        let cve_id = x.cve_id.clone();
        KnownCve {
            cve_id,
            description: x.description,
            cvss_score: x.cvss_v31,
            affected_versions: "unknown".to_string(),
            reference_url: format!("https://nvd.nist.gov/vuln/detail/{}", x.cve_id),
        }
    }).collect();
    out.sort_by(|a,b| b.cvss_score.partial_cmp(&a.cvss_score).unwrap_or(std::cmp::Ordering::Equal));
    out
}

fn expand_subnet(target_subnet: &str) -> Result<Vec<String>, String> {
    let net: IpNetwork = target_subnet.parse().map_err(|e| format!("Invalid subnet: {e}"))?;
    Ok(net.iter().take(1024).map(|ip| ip.to_string()).collect())
}

fn parse_arp_table() -> HashMap<String, String> {
    let output = if cfg!(windows) { Command::new("arp").arg("-a").output() } else { Command::new("arp").arg("-a").output() };
    let mut map = HashMap::new();
    if let Ok(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);
        let mac_re = regex::Regex::new(r"(?i)([0-9a-f]{2}[:-]){5}[0-9a-f]{2}").unwrap();
        let ip_re = regex::Regex::new(r"\b(\d{1,3}(?:\.\d{1,3}){3})\b").unwrap();
        for line in s.lines() {
            if let (Some(ip), Some(mac)) = (ip_re.find(line), mac_re.find(line)) {
                map.insert(ip.as_str().to_string(), mac.as_str().replace('-', ":").to_uppercase());
            }
        }
    }
    map
}

async fn banner_grab(ip: &str, port: u16) -> Option<String> {
    let addr = format!("{}:{}", ip, port);
    let mut stream = timeout(Duration::from_millis(300), TcpStream::connect(&addr)).await.ok()?.ok()?;
    if port == 80 || port == 8080 || port == 443 { let _ = stream.write_all(b"HEAD / HTTP/1.0\r\n\r\n").await; }
    if port == 554 { let _ = stream.write_all(b"OPTIONS rtsp://example.com RTSP/1.0\r\nCSeq: 1\r\n\r\n").await; }
    let mut buf = vec![0u8; 512];
    let n = timeout(Duration::from_millis(300), stream.read(&mut buf)).await.ok()?.ok()?;
    if n == 0 { None } else { Some(String::from_utf8_lossy(&buf[..n]).to_string()) }
}

#[tauri::command]
pub async fn passive_scan_network(target_subnet: String, _scan_depth: Option<String>, log_state: State<'_, crate::LogState>) -> Result<PassiveScanReport, String> {
    crate::push_runtime_log(&log_state, format!("[PASSIVE] scan subnet {}", target_subnet));
    let started = Instant::now();
    let ips = expand_subnet(&target_subnet)?;
    let arp = parse_arp_table();
    let ports: Vec<u16> = vec![80,443,554,8080,21,22,23,161,502,1883,47808,9100,104];

    let devices: Vec<IoTDevice> = stream::iter(ips)
        .map(|ip| {
            let ports = ports.clone();
            let mac = arp.get(&ip).cloned();
            async move {
                let mut open_ports = vec![];
                let mut banners = HashMap::new();
                for p in ports {
                    let addr = format!("{}:{}", ip, p);
                    if timeout(Duration::from_millis(300), TcpStream::connect(&addr)).await.is_ok_and(|x| x.is_ok()) {
                        open_ports.push(p);
                        if let Some(b) = banner_grab(&ip, p).await { banners.insert(p, b); }
                    }
                }
                if open_ports.is_empty() { return None; }

                let manufacturer = mac.as_ref().and_then(|m| {
                    let key = m.split(':').take(3).collect::<Vec<_>>().join(":");
                    get_oui_database().get(key.as_str()).map(|s| s.to_string())
                });
                let device_type = classify_device(manufacturer.as_deref().unwrap_or(""), &banners, &open_ports);
                let known_cves = lookup_cves_for_device(manufacturer.as_deref().unwrap_or("Unknown"), None, &device_type).await;
                let max_cvss = known_cves.iter().filter_map(|c| c.cvss_score).fold(0.0f32, f32::max);
                let risk_level = if max_cvss >= 9.0 { "critical" } else if max_cvss >= 7.0 { "high" } else if max_cvss >= 4.0 { "medium" } else if !open_ports.is_empty() { "low" } else { "info" };
                Some(IoTDevice {
                    ip,
                    mac_address: mac,
                    manufacturer,
                    device_type,
                    hostname: None,
                    open_ports,
                    banners,
                    os_guess: None,
                    firmware_version: None,
                    known_cves,
                    risk_level: risk_level.to_string(),
                    detection_method: "banner".into(),
                })
            }
        })
        .buffer_unordered(50)
        .filter_map(|x| async move { x })
        .collect()
        .await;

    let medical_devices: Vec<IoTDevice> = devices.iter().filter(|d| d.device_type == "medical").cloned().collect();
    let high_risk_count = devices.iter().filter(|d| d.risk_level == "high" || d.risk_level == "critical").count();
    Ok(PassiveScanReport {
        scan_id: format!("passive_{}", chrono::Utc::now().timestamp_millis()),
        scan_type: "live_scan".into(),
        total_devices: devices.len(),
        devices,
        medical_devices,
        high_risk_count,
        duration_ms: started.elapsed().as_millis() as u64,
        warnings: vec!["Read-only mode: only TCP connect/banner/ARP/CVE lookup".into()],
    })
}

#[tauri::command]
pub async fn analyze_pcap_file(pcap_path: String, log_state: State<'_, crate::LogState>) -> Result<PassiveScanReport, String> {
    crate::push_runtime_log(&log_state, format!("[PASSIVE] analyze pcap {}", pcap_path));
    let started = Instant::now();
    let output = Command::new("tshark")
        .args(["-r", &pcap_path, "-T", "fields", "-e", "ip.src", "-e", "ip.dst", "-e", "tcp.dstport", "-e", "http.server", "-e", "rtsp.server", "-e", "eth.src", "-E", "separator=|"])
        .output()
        .map_err(|e| format!("tshark error: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout);

    let mut by_ip: HashMap<String, IoTDevice> = HashMap::new();
    let mut warnings = vec![];
    for line in text.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 6 { continue; }
        let src = parts[0].trim();
        let dst = parts[1].trim();
        let port = parts[2].trim().parse::<u16>().unwrap_or(0);
        let http_server = parts[3].trim();
        let rtsp_server = parts[4].trim();
        let mac = parts[5].trim();
        if src.is_empty() { continue; }

        let d = by_ip.entry(src.to_string()).or_insert(IoTDevice {
            ip: src.to_string(), mac_address: if mac.is_empty(){None}else{Some(mac.to_string())}, manufacturer: None,
            device_type: "unknown".into(), hostname: None, open_ports: vec![], banners: HashMap::new(), os_guess: None,
            firmware_version: None, known_cves: vec![], risk_level: "info".into(), detection_method: "pcap".into(),
        });
        if port > 0 && !d.open_ports.contains(&port) { d.open_ports.push(port); }
        if !http_server.is_empty() { d.banners.insert(80, http_server.to_string()); }
        if !rtsp_server.is_empty() { d.banners.insert(554, rtsp_server.to_string()); }

        if port == 21 { warnings.push(format!("Обнаружена передача учётных данных в открытом виде (FTP) между {} и {}", src, dst)); }
        if port == 23 { warnings.push(format!("Обнаружена передача учётных данных в открытом виде (Telnet) между {} и {}", src, dst)); }
    }

    let mut devices: Vec<IoTDevice> = by_ip.into_values().collect();
    for d in &mut devices {
        let manu = d.manufacturer.clone().unwrap_or_default();
        d.device_type = classify_device(&manu, &d.banners, &d.open_ports);
        d.known_cves = lookup_cves_for_device(&manu, None, &d.device_type).await;
        let max_cvss = d.known_cves.iter().filter_map(|c| c.cvss_score).fold(0.0f32, f32::max);
        d.risk_level = if max_cvss >= 9.0 { "critical".into() } else if max_cvss >= 7.0 { "high".into() } else if max_cvss >= 4.0 { "medium".into() } else if !d.open_ports.is_empty() { "low".into() } else { "info".into() };
    }
    let medical_devices: Vec<IoTDevice> = devices.iter().filter(|d| d.device_type == "medical").cloned().collect();
    let high_risk_count = devices.iter().filter(|d| d.risk_level == "high" || d.risk_level == "critical").count();

    let mut uniq = HashSet::new();
    warnings.retain(|x| uniq.insert(x.clone()));
    Ok(PassiveScanReport {
        scan_id: format!("pcap_{}", chrono::Utc::now().timestamp_millis()),
        scan_type: "pcap_analysis".into(),
        total_devices: devices.len(),
        devices,
        medical_devices,
        high_risk_count,
        duration_ms: started.elapsed().as_millis() as u64,
        warnings,
    })
}
