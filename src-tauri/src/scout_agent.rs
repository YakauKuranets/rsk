// src-tauri/src/scout_agent.rs
// Autonomous 24/7 scout agent: monitors Shodan/Censys/GitHub/Telegram
// Writes discoveries to FeedbackStore and emits surface-change events
use crate::feedback_store::FeedbackStore;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoutConfig {
    pub scout_id: String,
    pub targets: Vec<String>,
    pub keywords: Vec<String>,
    pub interval_minutes: u64,
    pub sources: Vec<String>,
    pub shodan_key: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub telegram_channels: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoutAlert {
    pub scout_id: String,
    pub source: String,
    pub alert_type: String,
    pub target: String,
    pub description: String,
    pub severity: String,
    pub raw_data: serde_json::Value,
    pub detected_at: String,
}

pub struct ScoutState {
    pub active_scouts: Mutex<HashSet<String>>,
}

impl ScoutState {
    pub fn new() -> Self {
        Self {
            active_scouts: Mutex::new(HashSet::new()),
        }
    }
}

async fn scout_shodan(keywords: &[String], api_key: &str) -> Vec<ScoutAlert> {
    let Ok(client) = reqwest::Client::builder().timeout(Duration::from_secs(15)).build() else {
        return Vec::new();
    };
    let mut alerts = Vec::new();

    for kw in keywords.iter().take(5) {
        let url = format!(
            "https://api.shodan.io/shodan/host/search?key={}&query={}&minify=true",
            api_key,
            urlencoding::encode(kw)
        );
        let Ok(Ok(resp)) = tokio::time::timeout(Duration::from_secs(10), client.get(&url).send()).await else {
            continue;
        };
        let Ok(json) = resp.json::<serde_json::Value>().await else {
            continue;
        };

        let matches = json["matches"].as_array().cloned().unwrap_or_default();
        for m in matches.iter().take(10) {
            let ip = m["ip_str"].as_str().unwrap_or("").to_string();
            if ip.is_empty() {
                continue;
            }
            alerts.push(ScoutAlert {
                scout_id: String::new(),
                source: "shodan".to_string(),
                alert_type: "new_exposure".to_string(),
                target: ip.clone(),
                description: format!("{} found on Shodan for query: {}", ip, kw),
                severity: "MEDIUM".to_string(),
                raw_data: m.clone(),
                detected_at: chrono::Utc::now().to_rfc3339(),
            });
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    alerts
}

async fn scout_github(keywords: &[String]) -> Vec<ScoutAlert> {
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Hyperion-Scout/1.0")
        .build()
    else {
        return Vec::new();
    };
    let mut alerts = Vec::new();

    for kw in keywords.iter().take(3) {
        let query = format!("{} exploit OR poc OR vulnerability", kw);
        let url = format!(
            "https://api.github.com/search/repositories?q={}&sort=updated&per_page=5",
            urlencoding::encode(&query)
        );
        let Ok(Ok(resp)) = tokio::time::timeout(Duration::from_secs(10), client.get(&url).send()).await else {
            continue;
        };
        let Ok(json) = resp.json::<serde_json::Value>().await else {
            continue;
        };

        let repos = json["items"].as_array().cloned().unwrap_or_default();
        for repo in repos.iter().take(3) {
            let name = repo["full_name"].as_str().unwrap_or("").to_string();
            let stars = repo["stargazers_count"].as_u64().unwrap_or(0);
            let desc = repo["description"].as_str().unwrap_or("").to_string();
            let desc_low = desc.to_lowercase();

            if stars > 5 || desc_low.contains("poc") || desc_low.contains("exploit") {
                alerts.push(ScoutAlert {
                    scout_id: String::new(),
                    source: "github".to_string(),
                    alert_type: "new_exploit_repo".to_string(),
                    target: kw.clone(),
                    description: format!("GitHub PoC: {} ({} stars) — {}", name, stars, desc),
                    severity: if stars > 100 { "HIGH" } else { "MEDIUM" }.to_string(),
                    raw_data: repo.clone(),
                    detected_at: chrono::Utc::now().to_rfc3339(),
                });
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    alerts
}

async fn scout_telegram(channels: &[String], keywords: &[String]) -> Vec<ScoutAlert> {
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 Hyperion-Scout/1.0")
        .build()
    else {
        return Vec::new();
    };
    let mut alerts = Vec::new();

    for channel in channels.iter().take(5) {
        let url = format!("https://t.me/s/{}", channel.trim_start_matches('@'));
        let Ok(Ok(resp)) = tokio::time::timeout(Duration::from_secs(10), client.get(&url).send()).await else {
            continue;
        };
        let Ok(Ok(html)) = tokio::time::timeout(Duration::from_secs(5), resp.text()).await else {
            continue;
        };
        let html_low = html.to_lowercase();

        for kw in keywords {
            let kw_low = kw.to_lowercase();
            if html_low.contains(&kw_low) {
                let snippet = html_low
                    .split(&kw_low)
                    .nth(1)
                    .unwrap_or("")
                    .chars()
                    .take(200)
                    .collect::<String>();
                alerts.push(ScoutAlert {
                    scout_id: String::new(),
                    source: "telegram".to_string(),
                    alert_type: "keyword_mention".to_string(),
                    target: channel.clone(),
                    description: format!(
                        "Keyword '{}' mentioned in @{}: ...{}...",
                        kw,
                        channel,
                        snippet.chars().take(100).collect::<String>()
                    ),
                    severity: "HIGH".to_string(),
                    raw_data: serde_json::json!({"channel": channel, "keyword": kw}),
                    detected_at: chrono::Utc::now().to_rfc3339(),
                });
            }
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    alerts
}

#[tauri::command]
pub async fn start_scout_agent(
    config: ScoutConfig,
    app: AppHandle,
    scout_state: State<'_, ScoutState>,
    feedback: State<'_, Arc<FeedbackStore>>,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    let sid = config.scout_id.clone();
    {
        let mut active = scout_state.active_scouts.lock().map_err(|_| "lock")?;
        active.insert(sid.clone());
    }

    crate::push_runtime_log(
        &log_state,
        format!(
            "SCOUT_START|id={}|sources={:?}|interval={}m",
            sid, config.sources, config.interval_minutes
        ),
    );

    let keywords = config.keywords.clone();
    let sources = config.sources.clone();
    let shodan_key = config.shodan_key.clone();
    let tg_channels = config.telegram_channels.clone();
    let interval = config.interval_minutes.max(1);
    let fb = Arc::clone(&feedback);
    let sid2 = sid.clone();
    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(interval * 60));
        ticker.tick().await;

        loop {
            let is_active = app_handle
                .state::<ScoutState>()
                .active_scouts
                .lock()
                .map(|active| active.contains(&sid2))
                .unwrap_or(false);
            if !is_active {
                break;
            }

            ticker.tick().await;
            let mut all_alerts: Vec<ScoutAlert> = Vec::new();

            if sources.iter().any(|s| s == "shodan") {
                if let Some(ref key) = shodan_key {
                    let mut alerts = scout_shodan(&keywords, key).await;
                    alerts.iter_mut().for_each(|a| a.scout_id = sid2.clone());
                    all_alerts.extend(alerts);
                }
            }

            if sources.iter().any(|s| s == "github") {
                let mut alerts = scout_github(&keywords).await;
                alerts.iter_mut().for_each(|a| a.scout_id = sid2.clone());
                all_alerts.extend(alerts);
            }

            if sources.iter().any(|s| s == "telegram") && !tg_channels.is_empty() {
                let mut alerts = scout_telegram(&tg_channels, &keywords).await;
                alerts.iter_mut().for_each(|a| a.scout_id = sid2.clone());
                all_alerts.extend(alerts);
            }

            if !all_alerts.is_empty() {
                for alert in all_alerts.iter().filter(|a| a.severity == "HIGH") {
                    fb.record_finding(&alert.target, &alert.description);
                }
                let _ = app_handle.emit("scout-alert", &all_alerts);
            }
        }
    });

    Ok(sid)
}

#[tauri::command]
pub fn stop_scout_agent(scout_id: String, scout_state: State<'_, ScoutState>) -> Result<(), String> {
    let mut active = scout_state.active_scouts.lock().map_err(|_| "lock")?;
    active.remove(&scout_id);
    Ok(())
}

#[tauri::command]
pub fn list_scout_agents(scout_state: State<'_, ScoutState>) -> Vec<String> {
    scout_state
        .active_scouts
        .lock()
        .map(|scouts| scouts.iter().cloned().collect())
        .unwrap_or_default()
}
