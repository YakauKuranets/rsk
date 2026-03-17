use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tauri::State;
use tokio::sync::Semaphore;
use tokio::time::{sleep, timeout};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialAuditResult {
    pub ip: String,
    pub vendor: String,
    pub success: bool,
    pub login: Option<String>,
    pub password: Option<String>,
    pub attempts_made: u32,
    pub method: String,
    pub duration_ms: u64,
    pub breach_status: Option<String>,
    pub adaptive_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialAuditConfig {
    pub max_attempts_per_host: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub attempt_timeout_secs: u64,
}

impl Default for CredentialAuditConfig {
    fn default() -> Self {
        Self {
            max_attempts_per_host: 50,
            base_delay_ms: 500,
            max_delay_ms: 5000,
            attempt_timeout_secs: 5,
        }
    }
}

fn credential_probe_semaphore() -> Arc<Semaphore> {
    static SEM: OnceLock<Arc<Semaphore>> = OnceLock::new();
    SEM.get_or_init(|| Arc::new(Semaphore::new(4))).clone()
}

#[tauri::command]
pub async fn advanced_credential_audit(
    ip: String,
    vendor: String,
    custom_wordlist: Option<Vec<String>>,
    max_attempts: Option<u32>,
    osint_context: Option<String>,
    log_state: State<'_, crate::LogState>,
) -> Result<CredentialAuditResult, String> {
    let start = Instant::now();
    let config = CredentialAuditConfig {
        max_attempts_per_host: max_attempts.unwrap_or(50),
        ..Default::default()
    };

    crate::push_runtime_log(
        &log_state,
        format!(
            "[CRED_AUDIT] Старт для {} (vendor: {}, max: {})",
            ip, vendor, config.max_attempts_per_host
        ),
    );

    let km = crate::knowledge::KnowledgeManager::new();
    let history = km.load_all();
    if let Some(exp) = history.get(&ip) {
        if !exp.login.is_empty() {
            crate::push_runtime_log(
                &log_state,
                format!("[CRED_AUDIT] Cache hit: {}@{}", exp.login, ip),
            );

            return Ok(CredentialAuditResult {
                ip,
                vendor,
                success: true,
                login: Some(exp.login.clone()),
                password: Some(exp.pass.clone()),
                attempts_made: 0,
                method: "cache".into(),
                duration_ms: start.elapsed().as_millis() as u64,
                breach_status: None,
                adaptive_delay_ms: 0,
            });
        }
    }

    let mut wordlist = build_smart_wordlist(&vendor, osint_context.as_deref());

    if let Some(custom) = custom_wordlist {
        for entry in custom {
            if let Some((l, p)) = entry.split_once(':') {
                wordlist.push((l.trim().to_string(), p.trim().to_string()));
            }
        }
    }

    dedupe_wordlist(&mut wordlist);
    wordlist.truncate(config.max_attempts_per_host as usize);

    crate::push_runtime_log(
        &log_state,
        format!("[CRED_AUDIT] Словарь: {} записей", wordlist.len()),
    );

    let ffmpeg = crate::get_ffmpeg_path();
    let mut adaptive_delay = config.base_delay_ms;
    let mut attempts = 0u32;

    for (login, pass) in &wordlist {
        attempts += 1;

        let test_url = build_rtsp_url(&vendor, &ip, login, pass);

        let permit = credential_probe_semaphore()
            .acquire_owned()
            .await
            .map_err(|e| e.to_string())?;

        let probe_start = Instant::now();
        let probe_result = timeout(
            Duration::from_secs(config.attempt_timeout_secs),
            tokio::process::Command::new(&ffmpeg)
                .args(crate::ffmpeg::FfmpegProfiles::probe(&test_url))
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status(),
        )
        .await;
        drop(permit);

        let rtt = probe_start.elapsed().as_millis() as u64;

        match probe_result {
            Ok(Ok(status)) if status.success() => {
                crate::push_runtime_log(
                    &log_state,
                    format!(
                        "[CRED_AUDIT] УСПЕХ: {}:*** @ {} (попытка #{})",
                        login, ip, attempts
                    ),
                );

                km.save_success(&ip, &vendor, &test_url, login, pass);

                let breach_status = if !pass.is_empty() {
                    crate::breach_analyzer::check_password_breach(pass.clone())
                        .await
                        .ok()
                } else {
                    None
                };

                return Ok(CredentialAuditResult {
                    ip,
                    vendor,
                    success: true,
                    login: Some(login.clone()),
                    password: Some(pass.clone()),
                    attempts_made: attempts,
                    method: "rtsp_probe".into(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    breach_status,
                    adaptive_delay_ms: adaptive_delay,
                });
            }
            _ => {}
        }

        if rtt > 2000 {
            adaptive_delay = (adaptive_delay + 1000).min(config.max_delay_ms);
            crate::push_runtime_log(
                &log_state,
                format!(
                    "[CRED_AUDIT] Slow response ({}ms), delay -> {}ms",
                    rtt, adaptive_delay
                ),
            );
        } else {
            adaptive_delay = config.base_delay_ms + (rtt % 300);
        }

        sleep(Duration::from_millis(adaptive_delay)).await;
    }

    crate::push_runtime_log(
        &log_state,
        format!("[CRED_AUDIT] Завершён без успеха ({} попыток)", attempts),
    );

    Ok(CredentialAuditResult {
        ip,
        vendor,
        success: false,
        login: None,
        password: None,
        attempts_made: attempts,
        method: "rtsp_probe".into(),
        duration_ms: start.elapsed().as_millis() as u64,
        breach_status: None,
        adaptive_delay_ms: adaptive_delay,
    })
}

fn dedupe_wordlist(wordlist: &mut Vec<(String, String)>) {
    let mut seen = HashSet::new();
    wordlist.retain(|(l, p)| !l.is_empty() && seen.insert(format!("{}:{}", l, p)));
}

fn build_smart_wordlist(vendor: &str, osint_context: Option<&str>) -> Vec<(String, String)> {
    let mut dict = Vec::new();

    match vendor {
        "hikvision" => {
            for pass in &["12345", "123456", "admin", "123456789abc", "Hik12345"] {
                dict.push(("admin".into(), pass.to_string()));
            }
        }
        "dahua" => {
            for pass in &["admin", "123456", "888888", "666666"] {
                dict.push(("admin".into(), pass.to_string()));
            }
        }
        "xmeye" => {
            dict.push(("admin".into(), String::new()));
            dict.push(("admin".into(), "admin".into()));
            dict.push(("admin".into(), "123456".into()));
        }
        "axis" => {
            dict.push(("root".into(), "pass".into()));
            dict.push(("root".into(), "root".into()));
        }
        _ => {
            for pass in &["admin", "12345", "123456", "", "password", "1234"] {
                dict.push(("admin".into(), pass.to_string()));
            }
        }
    }

    if let Some(ctx) = osint_context {
        let ctx_lower = ctx.to_lowercase();
        dict.push(("admin".into(), ctx_lower.clone()));
        dict.push(("admin".into(), format!("{}123", ctx_lower)));
        dict.push(("admin".into(), format!("{}2024", ctx_lower)));
        dict.push(("admin".into(), format!("{}2025", ctx_lower)));
        dict.push(("admin".into(), format!("{}2026", ctx_lower)));
    }

    dict
}

fn build_rtsp_url(vendor: &str, ip: &str, login: &str, pass: &str) -> String {
    match vendor {
        "hikvision" => format!(
            "rtsp://{}:{}@{}:554/Streaming/Channels/102",
            login, pass, ip
        ),
        "dahua" => format!(
            "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=1",
            login, pass, ip
        ),
        _ => format!(
            "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=1",
            login, pass, ip
        ),
    }
}
