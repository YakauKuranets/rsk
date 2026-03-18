use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;
use tauri::State;
use tokio::net::lookup_host;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubdomainResult {
    pub subdomain: String,
    pub ip_addresses: Vec<String>,
    pub source: String,
    pub alive: bool,
    pub http_status: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubdomainReport {
    pub domain: String,
    pub total_found: usize,
    pub alive_count: usize,
    pub results: Vec<SubdomainResult>,
}

async fn dns_resolve(host: &str) -> Vec<String> {
    match lookup_host(format!("{}:80", host)).await {
        Ok(addrs) => addrs
            .map(|a| a.ip().to_string())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect(),
        Err(_) => vec![],
    }
}

async fn http_probe(host: &str, client: &Client) -> Option<u16> {
    for scheme in &["https", "http"] {
        if let Ok(r) = client
            .head(&format!("{}://{}", scheme, host))
            .timeout(Duration::from_secs(4))
            .send()
            .await
        {
            return Some(r.status().as_u16());
        }
    }
    None
}

/// Certificate Transparency via crt.sh
async fn cert_transparency_search(domain: &str, client: &Client) -> Vec<String> {
    let url = format!("https://crt.sh/?q=%.{}&output=json", domain);
    let Ok(resp) = client.get(&url).timeout(Duration::from_secs(15)).send().await else {
        return vec![];
    };
    let Ok(entries) = resp.json::<Vec<Value>>().await else {
        return vec![];
    };
    let mut subs = HashSet::new();
    for e in &entries {
        if let Some(name) = e["name_value"].as_str() {
            for sub in name.split('\n') {
                let sub = sub.trim().trim_start_matches('*').trim_start_matches('.');
                if sub.ends_with(domain) && sub != domain {
                    subs.insert(sub.to_string());
                }
            }
        }
    }
    subs.into_iter().collect()
}

/// DNS brute-force with built-in wordlist
async fn dns_brute(domain: &str) -> Vec<String> {
    let common: &[&str] = &[
        "www",
        "mail",
        "ftp",
        "admin",
        "dev",
        "api",
        "test",
        "staging",
        "vpn",
        "remote",
        "gitlab",
        "jenkins",
        "jira",
        "confluence",
        "grafana",
        "kibana",
        "vault",
        "db",
        "mysql",
        "redis",
        "mongo",
        "elastic",
        "s3",
        "cdn",
        "media",
        "static",
        "files",
        "beta",
        "alpha",
        "demo",
        "qa",
        "uat",
        "prod",
        "backup",
        "support",
        "help",
        "shop",
    ];
    let mut found = vec![];
    let mut tasks = vec![];
    for &prefix in common {
        let host = format!("{}.{}", prefix, domain);
        tasks.push(tokio::spawn(async move {
            let ips = dns_resolve(&host).await;
            if !ips.is_empty() {
                Some((host, ips))
            } else {
                None
            }
        }));
    }
    for t in tasks {
        if let Ok(Some((h, ips))) = t.await {
            found.push((h, ips));
        }
    }
    found.into_iter().map(|(h, _)| h).collect()
}

#[tauri::command]
pub async fn hunt_subdomains(
    domain: String,
    log_state: State<'_, crate::LogState>,
) -> Result<SubdomainReport, String> {
    let domain = domain.trim().trim_start_matches("*.").to_string();
    if domain.is_empty() || !domain.contains('.') {
        return Err("Invalid domain".to_string());
    }
    crate::push_runtime_log(&log_state, format!("SUBDOMAIN|domain={}", domain));

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Hyperion-PTES/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    // Parallel: CT logs + DNS brute
    let (ct_subs, brute_subs) = tokio::join!(
        cert_transparency_search(&domain, &client),
        dns_brute(&domain),
    );

    let mut all: HashSet<String> = HashSet::new();
    let mut results = vec![];

    for (sub, src) in ct_subs
        .into_iter()
        .map(|s| (s, "crt.sh".to_string()))
        .chain(brute_subs.into_iter().map(|s| (s, "dns_brute".to_string())))
    {
        if all.insert(sub.clone()) {
            let ips = dns_resolve(&sub).await;
            let alive = !ips.is_empty();
            let http_status = if alive {
                http_probe(&sub, &client).await
            } else {
                None
            };
            results.push(SubdomainResult {
                subdomain: sub,
                ip_addresses: ips,
                source: src,
                alive,
                http_status,
            });
        }
    }

    let alive_count = results.iter().filter(|r| r.alive).count();
    crate::push_runtime_log(
        &log_state,
        format!(
            "SUBDOMAIN_DONE|domain={}|found={}|alive={}",
            domain,
            results.len(),
            alive_count
        ),
    );
    Ok(SubdomainReport {
        domain,
        total_found: results.len(),
        alive_count,
        results,
    })
}

#[tauri::command]
pub async fn cert_transparency(
    domain: String,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<String>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Hyperion-PTES/1.0")
        .build()
        .map_err(|e| e.to_string())?;
    crate::push_runtime_log(&log_state, format!("CT_SEARCH|domain={}", domain));
    Ok(cert_transparency_search(&domain, &client).await)
}
