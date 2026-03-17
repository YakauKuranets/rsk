use reqwest::Client;
use std::time::Duration;
use tauri::command;

pub async fn check_session(target_url: &str) -> Result<Option<String>, String> {
    // Добавляем http:// если протокол не указан
    let url = if !target_url.starts_with("http") {
        format!("http://{}", target_url)
    } else {
        target_url.to_string()
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(5)) // Таймаут (guardrail)
        .danger_accept_invalid_certs(true) // Игнорируем ошибки самоподписанных SSL (часто на камерах/роутерах)
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;

    // Собираем куки из заголовка Set-Cookie
    let cookies = response.headers().get_all(reqwest::header::SET_COOKIE);
    let mut vulnerabilities = Vec::new();

    for cookie in cookies {
        if let Ok(cookie_str) = cookie.to_str() {
            let cookie_lower = cookie_str.to_lowercase();
            let mut missing_flags = Vec::new();

            if !cookie_lower.contains("httponly") {
                missing_flags.push("HttpOnly");
            }
            if !cookie_lower.contains("secure") {
                missing_flags.push("Secure");
            }
            if !cookie_lower.contains("samesite") {
                missing_flags.push("SameSite");
            }

            if !missing_flags.is_empty() {
                // Извлекаем имя куки для лога (до первого '=')
                let cookie_name = cookie_str
                    .split('=')
                    .next()
                    .unwrap_or("Unknown")
                    .to_string();
                vulnerabilities.push(format!(
                    "Cookie '{}' is missing: {}",
                    cookie_name,
                    missing_flags.join(", ")
                ));
            }
        }
    }

    if vulnerabilities.is_empty() {
        Ok(None)
    } else {
        Ok(Some(vulnerabilities.join(" | ")))
    }
}

#[command]
pub async fn check_session_security(ip: String) -> Result<String, String> {
    match check_session(&ip).await? {
        Some(v) => Ok(format!("[SESSION_AUDIT] {}", v)),
        None => Ok("[SESSION_AUDIT] Флаги сессионных cookie выглядят безопасно".to_string()),
    }
}
