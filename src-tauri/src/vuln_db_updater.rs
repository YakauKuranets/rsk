use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tauri::State;
use tokio::time::{sleep, timeout};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VulnDbEntry {
    pub cve_id: String,
    pub description: String,
    pub cvss_v31: Option<f32>,
    pub in_kev: bool,
    pub vendor_tags: Vec<String>,
    pub last_updated: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VulnDbUpdateReport {
    pub updated_count: usize,
    pub kev_count: usize,
    pub nvd_count: usize,
    pub last_updated: i64,
}

pub async fn auto_update_if_needed() -> Result<(), String> {
    let db = sled::open(crate::get_vault_path().join("vuln_db")).map_err(|e| e.to_string())?;
    let tree = db.open_tree("vuln_db").map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp();
    let should_update = match tree.get("meta:last_update").map_err(|e| e.to_string())? {
        Some(v) => String::from_utf8(v.to_vec())
            .ok()
            .and_then(|x| x.parse::<i64>().ok())
            .map(|last| now - last >= 24 * 3600)
            .unwrap_or(true),
        None => true,
    };
    if should_update {
        // best-effort network update without UI state
        let _ = update_vuln_database_internal().await;
    }
    Ok(())
}

async fn update_vuln_database_internal() -> Result<VulnDbUpdateReport, String> {
    let db = sled::open(crate::get_vault_path().join("vuln_db")).map_err(|e| e.to_string())?;
    let tree = db.open_tree("vuln_db").map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Hyperion-PTES/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let mut updated_count = 0usize;

    let kev_json = timeout(Duration::from_secs(15), async {
        let r = client
            .get("https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        r.json::<Value>().await.map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "kev timeout".to_string())??;

    let kev_items = kev_json["vulnerabilities"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    for item in &kev_items {
        if let Some(cve_id) = item["cveID"].as_str() {
            let entry = VulnDbEntry {
                cve_id: cve_id.to_string(),
                description: item["shortDescription"].as_str().unwrap_or("").to_string(),
                cvss_v31: None,
                in_kev: true,
                vendor_tags: vec![item["vendorProject"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_lowercase()],
                last_updated: now,
            };
            tree.insert(
                format!("cve:{}", cve_id).as_bytes(),
                serde_json::to_vec(&entry).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            updated_count += 1;
        }
    }

    sleep(Duration::from_millis(300)).await;

    let nvd_url = format!(
        "https://services.nvd.nist.gov/rest/json/cves/2.0?resultsPerPage=100&lastModStartDate={}",
        urlencoding::encode(&(chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339())
    );

    let nvd_json = timeout(Duration::from_secs(20), async {
        let r = client
            .get(&nvd_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        r.json::<Value>().await.map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "nvd timeout".to_string())??;

    let nvd_items = nvd_json["vulnerabilities"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    for vuln in &nvd_items {
        let cve = &vuln["cve"];
        let cve_id = cve["id"].as_str().unwrap_or("");
        if cve_id.is_empty() {
            continue;
        }
        let description = cve["descriptions"]
            .as_array()
            .and_then(|d| d.iter().find(|e| e["lang"].as_str() == Some("en")))
            .and_then(|e| e["value"].as_str())
            .unwrap_or("")
            .to_string();
        let cvss = cve["metrics"]["cvssMetricV31"]
            .as_array()
            .and_then(|m| m.first())
            .and_then(|m| m["cvssData"]["baseScore"].as_f64())
            .map(|x| x as f32);

        let mut tags = vec![];
        let low = description.to_lowercase();
        for v in ["hikvision", "dahua", "axis", "xm", "camera"] {
            if low.contains(v) {
                tags.push(v.to_string());
            }
        }
        if tags.is_empty() {
            tags.push("generic".into());
        }

        let mut in_kev = false;
        if let Some(raw) = tree
            .get(format!("cve:{}", cve_id).as_bytes())
            .map_err(|e| e.to_string())?
        {
            if let Ok(prev) = serde_json::from_slice::<VulnDbEntry>(&raw) {
                in_kev = prev.in_kev;
            }
        }

        let entry = VulnDbEntry {
            cve_id: cve_id.to_string(),
            description,
            cvss_v31: cvss,
            in_kev,
            vendor_tags: tags,
            last_updated: now,
        };
        tree.insert(
            format!("cve:{}", cve_id).as_bytes(),
            serde_json::to_vec(&entry).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        updated_count += 1;
    }

    tree.insert("meta:last_update", now.to_string().as_bytes())
        .map_err(|e| e.to_string())?;
    tree.flush().map_err(|e| e.to_string())?;

    Ok(VulnDbUpdateReport {
        updated_count,
        kev_count: kev_items.len(),
        nvd_count: nvd_items.len(),
        last_updated: now,
    })
}
#[tauri::command]
pub async fn update_vuln_database(
    log_state: State<'_, crate::LogState>,
) -> Result<VulnDbUpdateReport, String> {
    crate::push_runtime_log(&log_state, "[VULN_DB] update started".to_string());
    let report = update_vuln_database_internal().await?;
    crate::push_runtime_log(
        &log_state,
        format!("[VULN_DB] updated {} entries", report.updated_count),
    );
    Ok(report)
}

#[tauri::command]
pub async fn query_local_vuln_db(
    vendor: String,
    keyword: String,
) -> Result<Vec<VulnDbEntry>, String> {
    let db = sled::open(crate::get_vault_path().join("vuln_db")).map_err(|e| e.to_string())?;
    let tree = db.open_tree("vuln_db").map_err(|e| e.to_string())?;

    let v = vendor.to_lowercase();
    let k = keyword.to_lowercase();
    let mut out = Vec::new();

    for kv in tree.scan_prefix("cve:") {
        let (_, val) = kv.map_err(|e| e.to_string())?;
        let entry: VulnDbEntry = serde_json::from_slice(&val).map_err(|e| e.to_string())?;
        let vendor_ok = v.is_empty() || entry.vendor_tags.iter().any(|t| t.contains(&v));
        let key_ok = k.is_empty()
            || entry.description.to_lowercase().contains(&k)
            || entry.cve_id.to_lowercase().contains(&k);
        if vendor_ok && key_ok {
            out.push(entry);
        }
    }

    out.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));
    Ok(out)
}
