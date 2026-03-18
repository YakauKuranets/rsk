use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::time::Duration;
use tauri::{command, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreachReport {
    pub source: String,
    pub title: String,
    pub date: String,
    pub description: String,
}

async fn query_breaches(target: &str, client: &Client) -> Vec<BreachReport> {
    let mut reports = Vec::new();

    // Интеграция с AlienVault OTX (доменный OSINT)
    let otx_url = format!(
        "https://otx.alienvault.com/api/v1/indicators/domain/{}/general",
        target
    );

    if let Ok(resp) = client.get(&otx_url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(pulses) = json["pulse_info"]["count"].as_u64() {
                    if pulses > 0 {
                        reports.push(BreachReport {
                            source: "AlienVault OTX".to_string(),
                            title: format!(
                                "Обнаружено {} упоминаний в хакерских кампаниях",
                                pulses
                            ),
                            date: "Актуально".to_string(),
                            description:
                                "Домен фигурирует в базах вредоносной активности или спам-листах."
                                    .to_string(),
                        });
                    }
                }
            }
        }
    }

    // Эвристика для e-mail (заглушка под HIBP/DeHashed/LeakIX)
    if target.contains('@') {
        reports.push(BreachReport {
            source: "DarkWeb Monitor (Эвристика)".to_string(),
            title: "Возможная компрометация пароля".to_string(),
            date: "Требуется уточнение".to_string(),
            description: format!(
                "Email {} подлежит проверке по базе COMB (Compilation of Many Breaches).",
                target
            ),
        });
    }

    reports
}

#[tauri::command]
pub async fn check_breaches(
    target: String,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<BreachReport>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    crate::push_runtime_log(
        &log_state,
        format!("🕵️ THREAT INTEL: Поиск утечек для цели [{}]", target),
    );

    Ok(query_breaches(&target, &client).await)
}

pub async fn check_breaches_summary(target: &str) -> Result<Option<String>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let reports = query_breaches(target, &client).await;
    if reports.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            reports
                .into_iter()
                .map(|r| format!("{}: {}", r.source, r.title))
                .collect::<Vec<_>>()
                .join("; "),
        ))
    }
}

/// Безопасная проверка пароля по базе Have I Been Pwned (k-Anonymity)
#[command]
pub async fn check_password_breach(password: String) -> Result<String, String> {
    if password.is_empty() {
        return Ok("Пароль пуст (без пароля). Проверка HIBP пропущена.".into());
    }

    let mut hasher = Sha1::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    let hash_hex = format!("{:X}", result);

    let prefix = &hash_hex[..5];
    let suffix = &hash_hex[5..];

    let client = Client::new();
    let url = format!("https://api.pwnedpasswords.com/range/{}", prefix);

    let response = client
        .get(&url)
        .header("User-Agent", "Hyperion-PTES-Auditor/1.0")
        .send()
        .await
        .map_err(|e| format!("Ошибка сети HIBP: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HIBP API вернул ошибку: {}", response.status()));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Ошибка чтения ответа: {}", e))?;

    for line in body.lines() {
        if let Some((line_suffix, count_str)) = line.split_once(':') {
            if line_suffix == suffix {
                let count: u32 = count_str.trim().parse().unwrap_or(0);
                return Ok(format!(
                    "🚨 КРИТИЧЕСКАЯ УГРОЗА: Пароль найден в публичных утечках {} раз!",
                    count
                ));
            }
        }
    }

    Ok("✅ Пароль чист: в известных утечках не фигурирует.".into())
}
