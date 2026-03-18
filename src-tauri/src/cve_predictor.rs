// src-tauri/src/cve_predictor.rs
// CVE risk prediction using EPSS scores + KEV status + device profile
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpssEntry {
    pub cve: String,
    pub epss_score: f32, // 0.0-1.0: probability of exploitation in 30 days
    pub percentile: f32,
    pub model_date: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CvePrediction {
    pub cve_id: String,
    pub cvss_score: Option<f32>,
    pub epss_score: Option<f32>,
    pub in_kev: bool,
    pub exploit_probability_30d: f32,
    pub priority: String, // "IMMEDIATE" | "HIGH" | "MEDIUM" | "LOW"
    pub action: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictionReport {
    pub predictions: Vec<CvePrediction>,
    pub immediate_count: usize,
    pub patch_window_days: u32,
}

/// Fetch EPSS scores from FIRST.org API
pub async fn fetch_epss(cve_ids: &[String]) -> Vec<EpssEntry> {
    if cve_ids.is_empty() {
        return vec![];
    }
    let query = cve_ids.iter().take(50).cloned().collect::<Vec<_>>().join(",");
    let url = format!("https://api.first.org/data/v1/epss?cve={}", query);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Hyperion-PTES/1.0")
        .build()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let Ok(Ok(resp)) =
        tokio::time::timeout(Duration::from_secs(15), client.get(&url).send()).await
    else {
        return vec![];
    };

    let Ok(json): Result<serde_json::Value, _> = resp.json().await else {
        return vec![];
    };

    json["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|e| EpssEntry {
            cve: e["cve"].as_str().unwrap_or("").to_string(),
            epss_score: e["epss"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0),
            percentile: e["percentile"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            model_date: e["model_version"].as_str().unwrap_or("").to_string(),
        })
        .collect()
}

/// Priority formula: KEV + EPSS + CVSS combined
fn calculate_priority(cvss: Option<f32>, epss: Option<f32>, in_kev: bool) -> (String, f32, String) {
    let cvss_score = cvss.unwrap_or(5.0);
    let epss_score = epss.unwrap_or(0.01);

    // Probability of exploitation in 30 days
    let exploit_prob = if in_kev {
        // KEV = confirmed exploitation in the wild
        (epss_score + 0.5).min(1.0)
    } else {
        epss_score
    };

    // SSVC-inspired priority
    let (priority, action) = if in_kev && cvss_score >= 7.0 {
        ("IMMEDIATE", "Patch within 24 hours. Isolate device immediately.")
    } else if epss_score >= 0.5 || (in_kev && cvss_score >= 4.0) {
        (
            "HIGH",
            "Patch within 7 days. Monitor for exploitation attempts.",
        )
    } else if epss_score >= 0.1 || cvss_score >= 7.0 {
        ("MEDIUM", "Patch within 30 days. Add to next maintenance window.")
    } else {
        ("LOW", "Patch when convenient. Monitor EPSS score for changes.")
    };

    (priority.to_string(), exploit_prob, action.to_string())
}

#[tauri::command]
pub async fn predict_cve_risk(
    cve_ids: Vec<String>,
    log_state: State<'_, crate::LogState>,
) -> Result<PredictionReport, String> {
    crate::push_runtime_log(&log_state, format!("CVE_PREDICT|count={}", cve_ids.len()));

    // Fetch EPSS scores from FIRST.org
    let epss_data = fetch_epss(&cve_ids).await;
    let epss_map: std::collections::HashMap<String, f32> =
        epss_data.iter().map(|e| (e.cve.clone(), e.epss_score)).collect();

    // Check local VulnDB for CVSS and KEV status
    let mut predictions = Vec::new();
    for cve_id in &cve_ids {
        let db_entry = crate::vuln_db_updater::query_local_vuln_db(String::new(), cve_id.clone())
            .await
            .ok()
            .and_then(|v| v.into_iter().find(|e| &e.cve_id == cve_id));

        let cvss = db_entry.as_ref().and_then(|e| e.cvss_v31);
        let in_kev = db_entry.as_ref().map(|e| e.in_kev).unwrap_or(false);
        let epss = epss_map.get(cve_id).copied();

        let (priority, prob, action) = calculate_priority(cvss, epss, in_kev);

        predictions.push(CvePrediction {
            cve_id: cve_id.clone(),
            cvss_score: cvss,
            epss_score: epss,
            in_kev,
            exploit_probability_30d: prob,
            priority,
            action,
        });
    }

    // Sort by priority: IMMEDIATE first
    predictions.sort_by(|a, b| {
        let ord = |p: &str| match p {
            "IMMEDIATE" => 0,
            "HIGH" => 1,
            "MEDIUM" => 2,
            _ => 3,
        };
        ord(&a.priority).cmp(&ord(&b.priority))
    });

    let immediate_count = predictions
        .iter()
        .filter(|p| p.priority == "IMMEDIATE")
        .count();
    let patch_window = if immediate_count > 0 {
        1
    } else if predictions.iter().any(|p| p.priority == "HIGH") {
        7
    } else {
        30
    };

    crate::push_runtime_log(
        &log_state,
        format!(
            "CVE_PREDICT_DONE|immediate={}|window={}d",
            immediate_count, patch_window
        ),
    );

    Ok(PredictionReport {
        predictions,
        immediate_count,
        patch_window_days: patch_window,
    })
}

/// Fetch and update EPSS scores in local VulnDB
#[tauri::command]
pub async fn sync_epss_scores(log_state: State<'_, crate::LogState>) -> Result<String, String> {
    crate::push_runtime_log(&log_state, "EPSS_SYNC|start".to_string());

    // Get all CVEs from local DB
    let all_cves = crate::vuln_db_updater::query_local_vuln_db(String::new(), String::new())
        .await
        .map_err(|e| e.to_string())?;

    let cve_ids: Vec<String> = all_cves.iter().map(|e| e.cve_id.clone()).collect();
    let epss_data = fetch_epss(&cve_ids).await;

    // Store EPSS in sled
    let db = sled::open(crate::get_vault_path().join("vuln_db")).map_err(|e| e.to_string())?;
    let tree = db.open_tree("epss").map_err(|e| e.to_string())?;
    for entry in &epss_data {
        let _ = tree.insert(
            entry.cve.as_bytes(),
            serde_json::to_vec(entry).unwrap_or_default().as_slice(),
        );
    }
    let _ = tree.flush();

    crate::push_runtime_log(&log_state, format!("EPSS_SYNC_DONE|updated={}", epss_data.len()));
    Ok(format!("Updated {} EPSS scores", epss_data.len()))
}
