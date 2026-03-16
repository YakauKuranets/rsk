use reqwest::Client;
use std::collections::HashMap;
use tauri::command;

/// Функция для расчета информационной энтропии Шеннона (для оценки случайности токена)
fn calculate_entropy(s: &str) -> f64 {
    let mut counts = HashMap::new();
    for c in s.chars() {
        *counts.entry(c).or_insert(0) += 1;
    }
    let len = s.len() as f64;
    counts.values().fold(0.0, |acc, &count| {
        let p = count as f64 / len;
        acc - p * p.log2()
    })
}

/// Анализ механизмов сессий и авторизации веб-интерфейса камеры
#[command]
pub async fn check_session_security(ip: String) -> Result<String, String> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true) // Камеры часто используют самоподписанные сертификаты
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("http://{}", ip); // Проверяем стандартный HTTP порт

    let response = match client.get(&url).send().await {
        Ok(resp) => resp,
        Err(_) => {
            return Ok(format!(
                "[-] Веб-интерфейс на 80 порту недоступен для {}.",
                ip
            ))
        }
    };

    let mut report = format!("[SESSION_AUDIT] Отчет для {}:\n", ip);
    let mut vulnerabilities = 0;

    let headers = response.headers();

    // 1. Проверка Basic Auth без HTTPS (перехват паролей в открытом виде)
    if let Some(auth) = headers.get(reqwest::header::WWW_AUTHENTICATE) {
        let auth_str = auth.to_str().unwrap_or("").to_lowercase();
        if auth_str.contains("basic") && !url.starts_with("https") {
            report.push_str("🚨 УЯЗВИМОСТЬ: Используется Basic Auth по HTTP. Учетные данные передаются в открытом виде!\n");
            vulnerabilities += 1;
        } else if auth_str.contains("digest") {
            report.push_str("✅ Безопасность: Используется Digest Auth (защита от перехвата в открытом виде).\n");
        }
    }

    // 2. Анализ сессионных Cookies на предсказуемость (Энтропия)
    if let Some(cookie) = headers.get(reqwest::header::SET_COOKIE) {
        let cookie_str = cookie.to_str().unwrap_or("");
        report.push_str(&format!("🔎 Найден Cookie: {}\n", cookie_str));

        let entropy = calculate_entropy(cookie_str);
        report.push_str(&format!("📊 Энтропия токена: {:.2} бит\n", entropy));

        if entropy < 3.5 {
            report.push_str("🚨 КРИТИЧЕСКАЯ УЯЗВИМОСТЬ: Крайне низкая энтропия токена! Возможна атака Session Hijacking (предсказание сессии).\n");
            vulnerabilities += 1;
        } else if !cookie_str.to_lowercase().contains("httponly") {
            report.push_str("⚠️ ПРЕДУПРЕЖДЕНИЕ: Флаг HttpOnly отсутствует. Возможен перехват токена через XSS.\n");
        } else {
            report.push_str("✅ Токен выглядит надежным.\n");
        }
    } else {
        report.push_str("ℹ️ Cookie при первом запросе не выдаются.\n");
    }

    if vulnerabilities == 0 {
        report.push_str("✅ Явных уязвимостей сессий не обнаружено.");
    }

    Ok(report)
}
