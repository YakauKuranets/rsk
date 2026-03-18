// src-tauri/src/ioc_sharing.rs
// Lightweight P2P-like IoC sharing via signed broadcasts
// No central server — peers discover via shared secret URL pattern
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IocEntry {
    pub ioc_type: String, // "ip" | "cve" | "hash" | "domain" | "user_agent"
    pub value: String,
    pub threat_level: String,
    pub source: String,
    pub first_seen: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IocBroadcast {
    pub sender_id: String,
    pub timestamp: String,
    pub iocs: Vec<IocEntry>,
    pub signature: String, // HMAC-SHA256 of iocs JSON
}

fn hmac_sign(data: &str, secret: &str) -> String {
    use sha2::Digest;
    let key_hash = sha2::Sha256::digest(secret.as_bytes());
    let msg_hash = sha2::Sha256::digest(data.as_bytes());
    let combined = [key_hash.as_slice(), msg_hash.as_slice()].concat();
    hex::encode(sha2::Sha256::digest(&combined))
}

/// Export local IoCs from VulnDB KEV list + findings
pub async fn build_local_iocs() -> Vec<IocEntry> {
    let mut iocs = Vec::new();

    if let Ok(kev_entries) =
        crate::vuln_db_updater::query_local_vuln_db(String::new(), String::new()).await
    {
        for entry in kev_entries.iter().filter(|e| e.in_kev).take(100) {
            iocs.push(IocEntry {
                ioc_type: "cve".to_string(),
                value: entry.cve_id.clone(),
                threat_level: "HIGH".to_string(),
                source: "CISA_KEV".to_string(),
                first_seen: chrono::Utc::now().to_rfc3339(),
                tags: entry.vendor_tags.clone(),
            });
        }
    }

    iocs
}

#[tauri::command]
pub async fn share_iocs(
    peer_url: String,
    shared_secret: String,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    let iocs = build_local_iocs().await;
    if iocs.is_empty() {
        return Ok("No IoCs to share".to_string());
    }

    let iocs_json = serde_json::to_string(&iocs).map_err(|e| e.to_string())?;
    let sig = hmac_sign(&iocs_json, &shared_secret);
    let sender_suffix: String = shared_secret.chars().take(8).collect();

    let broadcast = IocBroadcast {
        sender_id: format!("hyperion_{}", sender_suffix),
        timestamp: chrono::Utc::now().to_rfc3339(),
        iocs,
        signature: sig,
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(format!("{}/ioc/receive", peer_url.trim_end_matches('/')))
        .json(&broadcast)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    crate::push_runtime_log(
        &log_state,
        format!(
            "IOC_SHARE|peer={}|count={}|status={}",
            peer_url,
            broadcast.iocs.len(),
            resp.status()
        ),
    );

    Ok(format!(
        "Shared {} IoCs with {} ({})",
        broadcast.iocs.len(),
        peer_url,
        resp.status()
    ))
}

#[tauri::command]
pub async fn receive_iocs(
    broadcast_json: String,
    shared_secret: String,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<IocEntry>, String> {
    let broadcast: IocBroadcast =
        serde_json::from_str(&broadcast_json).map_err(|e| e.to_string())?;

    let iocs_json = serde_json::to_string(&broadcast.iocs).map_err(|e| e.to_string())?;
    let expected_sig = hmac_sign(&iocs_json, &shared_secret);
    if expected_sig != broadcast.signature {
        return Err("IoC signature verification failed — untrusted source".to_string());
    }

    let db = sled::open(crate::get_vault_path().join("ioc_db")).map_err(|e| e.to_string())?;
    for ioc in &broadcast.iocs {
        let key = format!("{}:{}", ioc.ioc_type, ioc.value);
        let value = serde_json::to_vec(ioc).map_err(|e| e.to_string())?;
        let _ = db.insert(key.as_bytes(), value).map_err(|e| e.to_string())?;
    }
    let _ = db.flush().map_err(|e| e.to_string())?;

    crate::push_runtime_log(
        &log_state,
        format!(
            "IOC_RECEIVE|sender={}|count={}",
            broadcast.sender_id,
            broadcast.iocs.len()
        ),
    );

    Ok(broadcast.iocs)
}

#[tauri::command]
pub async fn lookup_ioc(
    value: String,
    ioc_type: Option<String>,
) -> Result<Option<IocEntry>, String> {
    let db = sled::open(crate::get_vault_path().join("ioc_db")).map_err(|e| e.to_string())?;
    let search_types = ioc_type.map(|t| vec![t]).unwrap_or_else(|| {
        vec!["ip", "cve", "hash", "domain", "user_agent"]
            .into_iter()
            .map(String::from)
            .collect()
    });

    for t in search_types {
        let key = format!("{}:{}", t, value);
        if let Ok(Some(bytes)) = db.get(key.as_bytes()) {
            if let Ok(ioc) = serde_json::from_slice::<IocEntry>(&bytes) {
                return Ok(Some(ioc));
            }
        }
    }

    Ok(None)
}
