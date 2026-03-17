use reqwest::Client;
use sha1::{Digest, Sha1};
use tauri::command;

/// Безопасная проверка пароля по базе Have I Been Pwned (k-Anonymity)
#[command]
pub async fn check_password_breach(password: String) -> Result<String, String> {
    if password.is_empty() {
        return Ok("Пароль пуст (без пароля). Проверка HIBP пропущена.".into());
    }

    // 1. Вычисляем SHA-1 хэш пароля
    let mut hasher = Sha1::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    let hash_hex = format!("{:X}", result); // HIBP требует заглавные буквы

    // 2. Разделяем хэш для k-Anonymity (отправляем только первые 5 символов)
    let prefix = &hash_hex[..5];
    let suffix = &hash_hex[5..];

    // 3. Делаем анонимный запрос к API
    let client = Client::new();
    let url = format!("https://api.pwnedpasswords.com/range/{}", prefix);

    let response = client
        .get(&url)
        .header("User-Agent", "Hyperion-PTES-Auditor/1.0") // Обязательный заголовок для HIBP
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

    // 4. Локально сверяем суффиксы из ответа
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
