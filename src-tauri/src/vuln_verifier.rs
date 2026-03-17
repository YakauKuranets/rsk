use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tauri::State;
use tokio::time::{sleep, timeout};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VulnVerificationResult {
    pub cve_id: String,
    pub vendor: String,
    pub description: String,
    pub cvss_score: Option<f32>,
    pub in_cisa_kev: bool,
    pub exploit_available: bool,
    pub exploit_sources: Vec<ExploitSource>,
    pub verification_status: String,
    pub evidence: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploitSource {
    pub source: String,
    pub url: String,
    pub title: String,
    pub severity: String,
}

#[tauri::command]
pub async fn verify_vulnerability(
    ip: String,
    vendor: String,
    firmware_version: Option<String>,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<VulnVerificationResult>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Hyperion-PTES/1.0")
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    crate::push_runtime_log(
        &log_state,
        format!("[VULN_VERIFY] Анализ {} (vendor: {})", ip, vendor),
    );

    let fw = firmware_version.as_deref().unwrap_or("");
    let query = if fw.is_empty() {
        vendor.clone()
    } else {
        format!("{} {}", vendor, fw)
    };

    crate::push_runtime_log(&log_state, format!("[VULN_VERIFY] NVD query: {}", query));

    let nvd_url = format!(
        "https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch={}&resultsPerPage=10",
        urlencoding::encode(&query)
    );

    let nvd_json = timeout(Duration::from_secs(12), async {
        let resp = client
            .get(&nvd_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        resp.json::<Value>().await.map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "NVD timeout".to_string())??;

    if let Some(vulns) = nvd_json["vulnerabilities"].as_array() {
        for vuln in vulns.iter().take(10) {
            let cve = &vuln["cve"];
            let cve_id = cve["id"].as_str().unwrap_or("").to_string();
            if cve_id.is_empty() {
                continue;
            }

            let description = cve["descriptions"]
                .as_array()
                .and_then(|d| d.iter().find(|e| e["lang"].as_str() == Some("en")))
                .and_then(|e| e["value"].as_str())
                .unwrap_or("")
                .to_string();

            let cvss_score = cve["metrics"]["cvssMetricV31"]
                .as_array()
                .and_then(|m| m.first())
                .and_then(|m| m["cvssData"]["baseScore"].as_f64())
                .map(|s| s as f32);

            let in_cisa_kev = check_cisa_kev(&client, &cve_id).await.unwrap_or(false);

            sleep(Duration::from_millis(500)).await;
            let exploit_sources = search_exploits_for_cve(&client, &cve_id).await;

            sleep(Duration::from_millis(250)).await;
            let verification = passive_verify(&client, &ip, &cve_id, &vendor).await;

            results.push(VulnVerificationResult {
                cve_id: cve_id.clone(),
                vendor: vendor.clone(),
                description,
                cvss_score,
                in_cisa_kev,
                exploit_available: !exploit_sources.is_empty(),
                exploit_sources,
                verification_status: verification.0,
                evidence: verification.1,
                recommendation: generate_recommendation(&vendor, cvss_score),
            });
        }
    }

    results.sort_by(|a, b| {
        b.cvss_score
            .partial_cmp(&a.cvss_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    crate::push_runtime_log(
        &log_state,
        format!("[VULN_VERIFY] Найдено {} CVE записей", results.len()),
    );

    Ok(results)
}

async fn search_exploits_for_cve(client: &Client, cve_id: &str) -> Vec<ExploitSource> {
    let mut sources = Vec::new();

    sources.push(ExploitSource {
        source: "nvd".into(),
        url: format!("https://nvd.nist.gov/vuln/detail/{}", cve_id),
        title: format!("NVD entry {}", cve_id),
        severity: "reference".into(),
    });

    let github_url = format!(
        "https://api.github.com/search/repositories?q={}&sort=stars&per_page=3",
        urlencoding::encode(cve_id)
    );

    if let Ok(Ok(json)) = timeout(Duration::from_secs(10), async {
        let resp = client
            .get(&github_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        resp.json::<Value>().await.map_err(|e| e.to_string())
    })
    .await
    {
        if let Some(items) = json["items"].as_array() {
            for item in items {
                sources.push(ExploitSource {
                    source: "github".into(),
                    url: item["html_url"].as_str().unwrap_or("").to_string(),
                    title: item["name"].as_str().unwrap_or("").to_string(),
                    severity: "PoC".into(),
                });
            }
        }
    }

    let exploit_db_search = format!(
        "https://www.exploit-db.com/search?cve={}",
        urlencoding::encode(cve_id)
    );
    if let Ok(Ok(body)) = timeout(Duration::from_secs(10), async {
        let resp = client
            .get(&exploit_db_search)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        resp.text().await.map_err(|e| e.to_string())
    })
    .await
    {
        let re = regex::Regex::new(r#"/exploits/(\d{3,7})"#).ok();
        if let Some(re) = re {
            for cap in re.captures_iter(&body).take(3) {
                if let Some(id) = cap.get(1) {
                    sources.push(ExploitSource {
                        source: "exploit_db".into(),
                        url: format!("https://www.exploit-db.com/exploits/{}", id.as_str()),
                        title: format!("Exploit-DB {} for {}", id.as_str(), cve_id),
                        severity: "PoC".into(),
                    });
                }
            }
        }
    }

    dedupe_sources(&mut sources);
    sources
}

async fn check_cisa_kev(client: &Client, cve_id: &str) -> Result<bool, String> {
    let kev_json = timeout(Duration::from_secs(10), async {
        let resp = client
            .get("https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        resp.json::<Value>().await.map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "CISA KEV timeout".to_string())??;

    Ok(kev_json["vulnerabilities"]
        .as_array()
        .map(|items| items.iter().any(|v| v["cveID"].as_str() == Some(cve_id)))
        .unwrap_or(false))
}

async fn passive_verify(client: &Client, ip: &str, cve_id: &str, vendor: &str) -> (String, String) {
    if vendor.to_lowercase().contains("hikvision") && cve_id.contains("2017") {
        let test_url = format!("http://{}/ISAPI/System/deviceInfo", ip);
        if let Ok(Ok(resp)) = timeout(Duration::from_secs(6), client.get(&test_url).send()).await {
            if resp.status().is_success() {
                return (
                    "confirmed".into(),
                    format!(
                        "Endpoint {} доступен без авторизации (HTTP {})",
                        test_url,
                        resp.status()
                    ),
                );
            }
            if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
                return (
                    "not_vulnerable".into(),
                    format!("Endpoint {} защищён (HTTP 401)", test_url),
                );
            }
        }
    }

    (
        "inconclusive".into(),
        "Пассивная верификация не дала результата".into(),
    )
}

fn dedupe_sources(sources: &mut Vec<ExploitSource>) {
    let mut seen = std::collections::HashSet::new();
    sources.retain(|s| seen.insert(format!("{}|{}", s.source, s.url)));
}

fn generate_recommendation(vendor: &str, cvss: Option<f32>) -> String {
    let urgency = match cvss {
        Some(s) if s >= 9.0 => "КРИТИЧНО",
        Some(s) if s >= 7.0 => "ВЫСОКИЙ ПРИОРИТЕТ",
        Some(s) if s >= 4.0 => "СРЕДНИЙ ПРИОРИТЕТ",
        _ => "НИЗКИЙ ПРИОРИТЕТ",
    };
    format!(
        "[{}] Рекомендация: обновить прошивку {}, ограничить доступ через VPN/ACL, включить строгую аутентификацию.",
        urgency, vendor
    )
}
