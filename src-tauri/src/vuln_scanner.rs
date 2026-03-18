use reqwest::Client;
use std::time::Duration;
use tauri::command;

/// Безопасная (read-only) верификация признаков уязвимостей без эксплуатации
#[command]
pub async fn verify_vulnerabilities(ip: String, vendor: String) -> Result<String, String> {
    let client = Client::builder()
        // device client: self-signed certs expected on local network hardware
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;

    let mut report = format!(
        "[VULN_SCAN] 🛡️ Пассивный анализ CVE-индикаторов для {} (Вендор: {})...\n",
        ip, vendor
    );
    let mut findings = 0u8;

    // 1) Hikvision: проверяем только индикаторы открытого доступа к info-эндпоинтам (без bypass-параметров)
    if vendor == "hikvision" || vendor == "unknown" {
        let endpoints = ["/ISAPI/System/deviceInfo", "/ISAPI/Security/users"];
        for ep in endpoints {
            let url = format!("http://{}{}", ip, ep);
            if let Ok(resp) = client.get(&url).send().await {
                let status = resp.status();
                if status.is_success() {
                    report.push_str(&format!(
                        "⚠️ Найден незащищённый доступ к {} (HTTP {}). Возможна уязвимая конфигурация/устаревшая прошивка.\n",
                        ep, status
                    ));
                    findings = findings.saturating_add(1);
                } else if status == reqwest::StatusCode::UNAUTHORIZED {
                    report.push_str(&format!("✅ {} требует авторизации (HTTP 401).\n", ep));
                }
            }
        }
    }

    // 2) XM/Dahua-like: проверяем утечки через публичные info/config endpoints (только чтение)
    if vendor == "xmeye" || vendor == "unknown" {
        let endpoints = [
            "/config/getglobal",
            "/system/deviceInfo",
            "/cgi-bin/magicBox.cgi?action=getSystemInfo",
        ];
        for ep in endpoints {
            let url = format!("http://{}{}", ip, ep);
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    let body = resp.text().await.unwrap_or_default().to_lowercase();
                    if body.contains("mac") || body.contains("version") || body.contains("serial") {
                        report.push_str(&format!(
                            "⚠️ Обнаружена потенциальная утечка данных через {} (read-only).\n",
                            ep
                        ));
                        findings = findings.saturating_add(1);
                    }
                }
            }
        }
    }

    if findings == 0 {
        report.push_str("✅ Явных индикаторов известных уязвимостей/экспозиций не обнаружено (пассивная проверка).");
    } else {
        report.push_str("ℹ️ Рекомендация: обновить прошивку, ограничить доступ ACL/VPN и включить строгую аутентификацию.");
    }

    Ok(report)
}
