use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tauri::State;
use tokio::time::{sleep, timeout};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredAsset {
    pub ip: String,
    pub port: u16,
    pub protocol: String,
    pub hostname: Option<String>,
    pub org: Option<String>,
    pub os: Option<String>,
    pub banner: String,
    pub source: String,
    pub country: Option<String>,
    pub city: Option<String>,
    pub last_seen: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertTransparencyEntry {
    pub common_name: String,
    pub issuer: String,
    pub not_after: String,
    pub san_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsRecord {
    pub record_type: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDiscoveryReport {
    pub query: String,
    pub total_assets: usize,
    pub assets: Vec<DiscoveredAsset>,
    pub certificates: Vec<CertTransparencyEntry>,
    pub dns_records: Vec<DnsRecord>,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn discover_external_assets(
    target_domain: String,
    shodan_api_key: Option<String>,
    enable_cert_transparency: Option<bool>,
    enable_dns_enum: Option<bool>,
    log_state: State<'_, crate::LogState>,
) -> Result<AssetDiscoveryReport, String> {
    let started = Instant::now();
    let domain = normalize_domain(&target_domain);

    if domain.is_empty() {
        return Err("target_domain пустой".into());
    }

    crate::push_runtime_log(
        &log_state,
        format!("[ASSET_DISCOVERY] Запуск разведки для: {}", domain),
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Hyperion-PTES/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let mut assets = Vec::new();
    let mut certificates = Vec::new();
    let mut dns_records = Vec::new();

    if let Some(api_key) = shodan_api_key.as_ref().filter(|k| !k.trim().is_empty()) {
        crate::push_runtime_log(&log_state, "[ASSET_DISCOVERY] Shodan query...".to_string());
        match query_shodan(&client, &domain, api_key).await {
            Ok(found) => {
                crate::push_runtime_log(
                    &log_state,
                    format!("[ASSET_DISCOVERY] Shodan: {} assets", found.len()),
                );
                assets.extend(found);
            }
            Err(e) => crate::push_runtime_log(
                &log_state,
                format!("[ASSET_DISCOVERY] Shodan error: {}", e),
            ),
        }
        sleep(Duration::from_millis(1100)).await;
    }

    if enable_cert_transparency.unwrap_or(true) {
        crate::push_runtime_log(&log_state, "[ASSET_DISCOVERY] crt.sh query...".to_string());
        match query_crtsh(&client, &domain).await {
            Ok(found) => {
                crate::push_runtime_log(
                    &log_state,
                    format!("[ASSET_DISCOVERY] crt.sh: {} certs", found.len()),
                );
                certificates = found;
            }
            Err(e) => crate::push_runtime_log(
                &log_state,
                format!("[ASSET_DISCOVERY] crt.sh error: {}", e),
            ),
        }
        sleep(Duration::from_millis(700)).await;
    }

    if enable_dns_enum.unwrap_or(true) {
        crate::push_runtime_log(&log_state, "[ASSET_DISCOVERY] DNS enum...".to_string());
        match query_dns_records(&client, &domain).await {
            Ok(found) => {
                dns_records = found;
                for rec in &dns_records {
                    if rec.record_type == "A" || rec.record_type == "AAAA" {
                        assets.push(DiscoveredAsset {
                            ip: rec.value.clone(),
                            port: 0,
                            protocol: "dns".into(),
                            hostname: Some(domain.clone()),
                            org: None,
                            os: None,
                            banner: format!("DNS {} запись", rec.record_type),
                            source: "dns".into(),
                            country: None,
                            city: None,
                            last_seen: None,
                        });
                    }
                }
                crate::push_runtime_log(
                    &log_state,
                    format!("[ASSET_DISCOVERY] DNS records: {}", dns_records.len()),
                );
            }
            Err(e) => crate::push_runtime_log(
                &log_state,
                format!("[ASSET_DISCOVERY] DNS enum error: {}", e),
            ),
        }
    }

    dedupe_assets(&mut assets);

    Ok(AssetDiscoveryReport {
        query: domain,
        total_assets: assets.len(),
        assets,
        certificates,
        dns_records,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

fn normalize_domain(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .split('/')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn dedupe_assets(assets: &mut Vec<DiscoveredAsset>) {
    let mut seen = HashSet::new();
    assets.retain(|item| seen.insert(format!("{}:{}:{}", item.ip, item.port, item.source)));
}

async fn query_shodan(client: &Client, target_domain: &str, api_key: &str) -> Result<Vec<DiscoveredAsset>, String> {
    let query = format!("hostname:{}", target_domain);
    let url = format!(
        "https://api.shodan.io/shodan/host/search?key={}&query={}",
        api_key,
        urlencoding::encode(&query)
    );

    let payload: Value = timeout(Duration::from_secs(15), async {
        let response = client.get(url).send().await.map_err(|e| e.to_string())?;
        response.json::<Value>().await.map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "Shodan timeout".to_string())??;

    let mut output = Vec::new();
    if let Some(matches) = payload["matches"].as_array() {
        for row in matches {
            output.push(DiscoveredAsset {
                ip: row["ip_str"].as_str().unwrap_or_default().to_string(),
                port: row["port"].as_u64().unwrap_or(0) as u16,
                protocol: row["transport"].as_str().unwrap_or("tcp").to_string(),
                hostname: row["hostnames"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                org: row["org"].as_str().map(ToString::to_string),
                os: row["os"].as_str().map(ToString::to_string),
                banner: row["data"].as_str().unwrap_or_default().to_string(),
                source: "shodan".into(),
                country: row["location"]["country_name"].as_str().map(ToString::to_string),
                city: row["location"]["city"].as_str().map(ToString::to_string),
                last_seen: row["timestamp"].as_str().map(ToString::to_string),
            });
        }
    }

    Ok(output)
}

async fn query_crtsh(client: &Client, target_domain: &str) -> Result<Vec<CertTransparencyEntry>, String> {
    let url = format!(
        "https://crt.sh/?q={}&output=json",
        urlencoding::encode(&format!("%.{}", target_domain))
    );

    let payload: Value = timeout(Duration::from_secs(15), async {
        let response = client.get(url).send().await.map_err(|e| e.to_string())?;
        response.json::<Value>().await.map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "crt.sh timeout".to_string())??;

    let mut entries = Vec::new();
    if let Some(rows) = payload.as_array() {
        for row in rows.iter().take(100) {
            let sans = row["name_value"]
                .as_str()
                .unwrap_or_default()
                .split('\n')
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>();

            entries.push(CertTransparencyEntry {
                common_name: row["common_name"].as_str().unwrap_or_default().to_string(),
                issuer: row["issuer_name"].as_str().unwrap_or_default().to_string(),
                not_after: row["not_after"].as_str().unwrap_or_default().to_string(),
                san_domains: sans,
            });
        }
    }

    Ok(entries)
}

async fn query_dns_records(client: &Client, target_domain: &str) -> Result<Vec<DnsRecord>, String> {
    let mut records = Vec::new();
    for record_type in ["A", "AAAA", "CNAME", "MX", "TXT"] {
        let url = format!(
            "https://cloudflare-dns.com/dns-query?name={}&type={}",
            urlencoding::encode(target_domain),
            record_type
        );

        let payload: Value = timeout(Duration::from_secs(10), async {
            let response = client
                .get(url)
                .header("accept", "application/dns-json")
                .send()
                .await
                .map_err(|e| e.to_string())?;
            response.json::<Value>().await.map_err(|e| e.to_string())
        })
        .await
        .map_err(|_| format!("DNS timeout for {}", record_type))??;

        if let Some(answers) = payload["Answer"].as_array() {
            for answer in answers {
                if let Some(value) = answer["data"].as_str() {
                    records.push(DnsRecord {
                        record_type: record_type.to_string(),
                        name: target_domain.to_string(),
                        value: value.trim_matches('"').to_string(),
                    });
                }
            }
        }

        sleep(Duration::from_millis(250)).await;
    }

    Ok(records)
}
