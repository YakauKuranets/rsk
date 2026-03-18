use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use tauri::command;
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebshellReport {
    pub url: String,
    pub shell_type: String,
    pub status: String,
}

const WEBSHELL_PATHS: &[&str] = &[
    "/shell.php",
    "/cmd.php",
    "/c99.php",
    "/wso.php",
    "/b374k.php",
    "/wp-content/uploads/shell.php",
    "/images/1.php",
    "/test.php",
    "/up.php",
    "/cmd.jsp",
    "/browser.jsp",
    "/shell.aspx",
];

const WEBSHELL_SIGNATURES: &[(&str, &str)] = &[
    ("c99", "c99shell"),
    ("wso", "Web Shell by oRb"),
    ("b374k", "b374k"),
    ("generic_cmd", "cmd_execute"),
    ("generic_eval", "eval("),
];

/// Пассивная проверка портов административного доступа (SSH / Telnet)
#[command]
pub async fn assess_persistence_risk(ip: String) -> Result<String, String> {
    let mut report = format!(
        "[PERSISTENCE_AUDIT] 🔍 Оценка риска закрепления для {}...\n",
        ip
    );
    let mut risk_found = false;

    let ports_to_check = [
        (22, "SSH (Secure Shell)"),
        (23, "Telnet (Незащищенный терминал)"),
    ];

    let check_timeout = Duration::from_millis(1500);

    for (port, service) in ports_to_check {
        let addr = format!("{}:{}", ip, port);

        if let Ok(Ok(mut _stream)) = timeout(check_timeout, TcpStream::connect(&addr)).await {
            report.push_str(&format!(
                "🚨 КРИТИЧЕСКИЙ РИСК: Открыт порт {} ({}).\n",
                port, service
            ));

            if port == 23 {
                report.push_str(
                    "   ⚠️ Telnet передает данные в открытом виде. Крайне высокий риск перехвата!\n",
                );
            }

            report.push_str("   ⚠️ Злоумышленник может использовать найденные RTSP-пароли для получения shell-доступа (RCE) и закрепления в системе (например, через crontab).\n");
            risk_found = true;
        }
    }

    if !risk_found {
        report.push_str(
            "✅ Риск закрепления минимален: порты удаленного управления (SSH/Telnet) закрыты извне.\n",
        );
    }

    Ok(report)
}

#[command]
pub async fn check_persistence(target: String) -> Result<Vec<WebshellReport>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        // device client: self-signed certs expected on local network hardware
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let base_url = if !target.starts_with("http") {
        format!("http://{}", target)
    } else {
        target.clone()
    };

    let mut found_shells = Vec::new();

    for path in WEBSHELL_PATHS {
        let test_url = format!("{}{}", base_url, path);
        if let Ok(response) = client.get(&test_url).send().await {
            if response.status().is_success() {
                if let Ok(text) = response.text().await {
                    let mut detected_type = String::from("Unknown / Possible False Positive");

                    for (name, sig) in WEBSHELL_SIGNATURES {
                        if text.contains(sig) {
                            detected_type = name.to_string();
                            break;
                        }
                    }

                    found_shells.push(WebshellReport {
                        url: test_url.clone(),
                        shell_type: detected_type.clone(),
                        status: "FOUND".to_string(),
                    });

                    eprintln!("⚠️ НАЙДЕН WEBSHELL: {} ({})", test_url, detected_type);
                }
            }
        }
    }

    Ok(found_shells)
}
