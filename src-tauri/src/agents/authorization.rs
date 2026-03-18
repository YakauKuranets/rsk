use chrono::{Duration, NaiveDate, Utc};
use rand::RngCore;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationRequest {
    pub target_ips: Vec<String>,
    pub permit_number: String,
    pub permit_date: String,
    pub operator_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationToken {
    pub token: String,
    pub expires_at: String,
    pub authorized_ips: Vec<String>,
    pub permit_number: String,
    pub operator_id: String,
}

fn generate_session_token() -> String {
    let mut random = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut random);
    let mut hasher = Sha256::new();
    hasher.update(random);
    hasher.update(Utc::now().to_rfc3339().as_bytes());
    format!("{:x}", hasher.finalize())
}

#[tauri::command]
pub async fn validate_exploit_authorization(
    request: AuthorizationRequest,
    log_state: State<'_, crate::LogState>,
) -> Result<AuthorizationToken, String> {
    let permit_re = Regex::new(r"^PT-\d{4}-\d{3,}$").map_err(|e| e.to_string())?;
    if !permit_re.is_match(&request.permit_number) {
        return Err("Неверный формат: ожидается PT-YYYY-NNN".into());
    }

    let permit_date = NaiveDate::parse_from_str(&request.permit_date, "%Y-%m-%d")
        .map_err(|_| "Неверный формат даты".to_string())?;
    if permit_date < Utc::now().date_naive() {
        return Err("Разрешение на пентест просрочено".into());
    }

    if request.target_ips.is_empty() {
        return Err("Не переданы target_ips для авторизации".into());
    }
    for target in &request.target_ips {
        target
            .parse::<std::net::IpAddr>()
            .map_err(|_| format!("Неверный IP в scope: {}", target))?;
    }

    crate::push_runtime_log(
        &log_state,
        format!(
            "EXPLOIT_AUTH|permit={}|operator={}|targets={:?}",
            request.permit_number, request.operator_id, request.target_ips
        ),
    );

    Ok(AuthorizationToken {
        token: generate_session_token(),
        expires_at: (Utc::now() + Duration::hours(8)).to_rfc3339(),
        authorized_ips: request.target_ips,
        permit_number: request.permit_number,
        operator_id: request.operator_id,
    })
}
