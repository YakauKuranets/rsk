use reqwest::Client;
use sha1::{Digest, Sha1};
use std::time::Duration;
use tauri::command;

pub async fn check_breaches(target: &str) -> Result<Option<String>, String> {
    // Guardrail: Этическая задержка имитирующая запрос к локальной БД или k-anonymity API
    tokio::time::sleep(Duration::from_millis(800)).await;

    // В будущем здесь будет загрузка SQLite базы с хэшами утечек (k-anonymity).
    // Для безопасного PoC используем хардкод-кэш известных скомпрометированных тестовых данных.
    let local_breach_cache = vec![
        ("admin@hikvision.com", "CamLeak2021 (Passwords, Emails)"),
        ("root@192.168.1.5", "LocalIoT_Breach (Default Creds)"),
        ("test@example.com", "ExampleDB_2019"),
    ];

    let mut found_breaches = Vec::new();

    for (compromised_target, breach_name) in local_breach_cache {
        if target.contains(compromised_target) {
            found_breaches.push(breach_name.to_string());
        }
    }

    if found_breaches.is_empty() {
        Ok(None)
    } else {
        Ok(Some(format!(
            "Скомпрометирован в базах: {}",
            found_breaches.join(", ")
        )))
    }
}

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
