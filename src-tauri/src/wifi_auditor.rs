use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub channel: u8,
    pub signal_dbm: i32,
    pub encryption: String,
    pub wps_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiAuditReport {
    pub networks: Vec<WifiNetwork>,
    pub handshake_path: Option<String>,
    pub pixie_dust_result: Option<String>,
    pub warnings: Vec<String>,
}

fn run_cmd(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("{}: {}", program, e))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[tauri::command]
pub async fn scan_wifi_networks(
    interface: String,
    permit_token: String,
    log_state: State<'_, crate::LogState>,
) -> Result<WifiAuditReport, String> {
    if permit_token.trim().len() < 8 {
        return Err("Unauthorized: provide valid permit token".to_string());
    }
    crate::push_runtime_log(
        &log_state,
        format!("WIFI_SCAN|iface={}|permit={}", interface, &permit_token[..8]),
    );

    let output = run_cmd("iw", &[&interface, "scan"]).unwrap_or_else(|_| {
        run_cmd(
            "nmcli",
            &["-t", "-f", "SSID,BSSID,CHAN,SIGNAL,SECURITY", "dev", "wifi"],
        )
        .unwrap_or_default()
    });

    let mut networks = Vec::new();
    let mut current_ssid = String::new();
    let mut current_bssid = String::new();
    let mut current_chan: u8 = 0;
    let mut current_sig: i32 = 0;
    let mut current_enc = String::new();
    let mut wps = false;

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("BSS ") {
            if !current_bssid.is_empty() {
                networks.push(WifiNetwork {
                    ssid: current_ssid.clone(),
                    bssid: current_bssid.clone(),
                    channel: current_chan,
                    signal_dbm: current_sig,
                    encryption: current_enc.clone(),
                    wps_enabled: wps,
                });
            }
            current_bssid = line[4..].split('(').next().unwrap_or("").trim().to_string();
            current_ssid = String::new();
            current_chan = 0;
            current_sig = 0;
            current_enc = "OPEN".to_string();
            wps = false;
        } else if line.starts_with("SSID:") {
            current_ssid = line[5..].trim().to_string();
        } else if line.starts_with("* primary channel:") {
            current_chan = line
                .split(':')
                .nth(1)
                .unwrap_or("0")
                .trim()
                .parse()
                .unwrap_or(0);
        } else if line.starts_with("signal:") {
            current_sig = line
                .split_whitespace()
                .nth(1)
                .unwrap_or("0")
                .parse::<f32>()
                .unwrap_or(0.0) as i32;
        } else if line.contains("WPA") || line.contains("RSN") {
            current_enc = "WPA".to_string();
        } else if line.contains("WPS") {
            wps = true;
        }
    }
    if !current_bssid.is_empty() {
        networks.push(WifiNetwork {
            ssid: current_ssid,
            bssid: current_bssid,
            channel: current_chan,
            signal_dbm: current_sig,
            encryption: current_enc,
            wps_enabled: wps,
        });
    }

    Ok(WifiAuditReport {
        networks,
        handshake_path: None,
        pixie_dust_result: None,
        warnings: vec!["Требуется монопольный доступ к интерфейсу".to_string()],
    })
}

#[tauri::command]
pub async fn capture_wifi_handshake(
    interface: String,
    bssid: String,
    channel: u8,
    output_dir: String,
    permit_token: String,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    if permit_token.trim().len() < 8 {
        return Err("Unauthorized: provide valid permit token".to_string());
    }
    crate::push_runtime_log(
        &log_state,
        format!(
            "WIFI_HANDSHAKE|bssid={}|chan={}|permit={}",
            bssid,
            channel,
            &permit_token[..8]
        ),
    );

    // Set monitor mode
    let _ = run_cmd("ip", &["link", "set", &interface, "down"]);
    let _ = run_cmd("iw", &[&interface, "set", "monitor", "none"]);
    let _ = run_cmd("ip", &["link", "set", &interface, "up"]);
    let _ = run_cmd("iw", &[&interface, "set", "channel", &channel.to_string()]);

    let cap_file = format!("{}/cap_{}", output_dir, bssid.replace(':', "_"));
    // Run airodump-ng for 30 seconds targeting specific BSSID
    let args = vec![
        "--bssid".to_string(),
        bssid.clone(),
        "--channel".to_string(),
        channel.to_string(),
        "--write".to_string(),
        cap_file.clone(),
        "--write-interval".to_string(),
        "1".to_string(),
        "--output-format".to_string(),
        "pcap".to_string(),
        interface.clone(),
    ];
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new("airodump-ng")
            .args(&args_ref)
            .output(),
    )
    .await
    .ok();

    Ok(format!("{}.pcap", cap_file))
}
