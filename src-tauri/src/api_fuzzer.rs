use reqwest::Client;
use std::time::Duration;

pub async fn run_fuzzer(target_url: &str) -> Result<Option<String>, String> {
    let base_url = if !target_url.starts_with("http") {
        format!("http://{}", target_url)
    } else {
        target_url.to_string()
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(3)) // Строгий таймаут (Guardrail)
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    // Soft payloads: ищем только чувствительные точки конфигурации и API
    let common_paths = vec![
        "api/v1/users",
        "swagger/v1/swagger.json",
        "swagger-ui.html",
        "openapi.json",
        "api/health",
        "graphql",
        ".env",
        "config.json",
    ];

    let mut found_endpoints = Vec::new();

    for path in common_paths {
        let url = format!("{}/{}", base_url.trim_end_matches('/'), path);

        // Этическая задержка (Rate-limit Guardrail)
        tokio::time::sleep(Duration::from_millis(300)).await;

        if let Ok(response) = client.get(&url).send().await {
            // Реагируем на 200 OK или 401 Unauthorized (означает, что эндпоинт существует)
            let status = response.status();
            if status.is_success() || status == reqwest::StatusCode::UNAUTHORIZED {
                found_endpoints.push(format!("/{} ({})", path, status));
            }
        }
    }

    if found_endpoints.is_empty() {
        Ok(None)
    } else {
        Ok(Some(found_endpoints.join(", ")))
    }
}

#[tauri::command]
pub async fn run_api_fuzzer(ip: String) -> Result<String, String> {
    match run_fuzzer(&ip).await? {
        Some(findings) => Ok(format!("[API_FUZZER] Найдены эндпоинты: {}", findings)),
        None => Ok("[API_FUZZER] Эндпоинты не обнаружены".to_string()),
    }
}
