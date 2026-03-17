use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use tauri::State;
use tokio::time::{sleep, timeout};

pub async fn run_fuzzer(target_url: &str) -> Result<Option<String>, String> {
    let base_url = if !target_url.starts_with("http") {
        format!("http://{}", target_url)
    } else {
        target_url.to_string()
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

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
        sleep(Duration::from_millis(300)).await;
        if let Ok(response) = client.get(&url).send().await {
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzFinding {
    pub endpoint: String,
    pub mutation_type: String,
    pub payload: String,
    pub status_code: u16,
    pub indicator: String,
    pub baseline_len: usize,
    pub response_len: usize,
}

#[tauri::command]
pub async fn smart_fuzz_api(
    target: String,
    mode: String,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<FuzzFinding>, String> {
    crate::push_runtime_log(
        &log_state,
        format!("[SMART_FUZZ] start {} ({})", target, mode),
    );

    let base_url = if !target.starts_with("http") {
        format!("http://{}", target)
    } else {
        target
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut endpoints = vec![
        "api/v1/users".to_string(),
        "api/health".to_string(),
        "graphql".to_string(),
    ];

    if mode.to_lowercase() != "quick" {
        if let Ok(mut sw) = discover_swagger_endpoints(&client, &base_url).await {
            endpoints.append(&mut sw);
        }
    }

    endpoints.sort();
    endpoints.dedup();

    let mutations = vec![
        ("sql_injection", "' OR 1=1--"),
        ("path_traversal", "../../../etc/passwd"),
        ("xss_probe", "<script>alert(1)</script>"),
        ("cmd_injection", ";id"),
        ("null_byte", "%00"),
    ];

    let mut findings = Vec::new();

    for ep in endpoints {
        let url = format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            ep.trim_start_matches('/')
        );

        let baseline = timeout(Duration::from_secs(6), client.get(&url).send())
            .await
            .ok()
            .and_then(|r| r.ok());
        let baseline_body = if let Some(resp) = baseline {
            resp.text().await.unwrap_or_default()
        } else {
            String::new()
        };
        let baseline_len = baseline_body.len();

        for (kind, payload) in &mutations {
            sleep(Duration::from_millis(220)).await;
            let fuzz_url = format!("{}?q={}", url, urlencoding::encode(payload));

            let response = timeout(Duration::from_secs(6), client.get(&fuzz_url).send())
                .await
                .map_err(|_| "fuzz timeout".to_string())?
                .map_err(|e| e.to_string())?;
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            let response_len = body.len();

            let body_low = body.to_lowercase();
            let mut indicator = String::new();
            if body.contains("root:x:0:0") {
                indicator = "path traversal confirmed".into();
            } else if body_low.contains("syntax error") || body_low.contains("mysql") {
                indicator = "sql injection indicator".into();
            } else if baseline_len > 0 && response_len.abs_diff(baseline_len) > (baseline_len / 2) {
                indicator = "response length anomaly".into();
            }

            if !indicator.is_empty() {
                findings.push(FuzzFinding {
                    endpoint: ep.clone(),
                    mutation_type: (*kind).to_string(),
                    payload: (*payload).to_string(),
                    status_code: status,
                    indicator,
                    baseline_len,
                    response_len,
                });
            }
        }
    }

    crate::push_runtime_log(
        &log_state,
        format!("[SMART_FUZZ] findings {}", findings.len()),
    );
    Ok(findings)
}

async fn discover_swagger_endpoints(
    client: &Client,
    base_url: &str,
) -> Result<Vec<String>, String> {
    let paths = ["swagger.json", "openapi.json", "v2/api-docs"];
    for p in paths {
        let url = format!("{}/{}", base_url.trim_end_matches('/'), p);
        let resp = match timeout(Duration::from_secs(5), client.get(&url).send()).await {
            Ok(Ok(r)) => r,
            _ => continue,
        };
        if !resp.status().is_success() {
            continue;
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        if let Some(obj) = json["paths"].as_object() {
            for key in obj.keys() {
                out.push(key.trim_start_matches('/').to_string());
            }
        }
        if !out.is_empty() {
            return Ok(out);
        }
    }
    Ok(Vec::new())
}

#[tauri::command]
pub async fn run_api_fuzzer(ip: String) -> Result<String, String> {
    match run_fuzzer(&ip).await? {
        Some(findings) => Ok(format!("[API_FUZZER] Найдены эндпоинты: {}", findings)),
        None => Ok("[API_FUZZER] Эндпоинты не обнаружены".to_string()),
    }
}
