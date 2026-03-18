use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechniqueRecord {
    pub technique: String,
    pub target_vendor: String,
    pub success_count: u32,
    pub fail_count: u32,
    pub last_success: Option<String>,
    pub avg_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CampaignMemory {
    pub techniques: HashMap<String, TechniqueRecord>,
    pub known_credentials: Vec<(String, String)>,
    pub vulnerable_paths: Vec<String>,
    pub blocked_vendors: Vec<String>,
}

pub struct FeedbackStore {
    pub memory: RwLock<HashMap<String, Vec<String>>>,
    pub campaign: RwLock<CampaignMemory>,
    db_path: std::path::PathBuf,
}

impl FeedbackStore {
    pub fn new() -> Self {
        let db_path = crate::get_vault_path().join("campaign_memory.json");
        let campaign = if db_path.exists() {
            std::fs::read_to_string(&db_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            CampaignMemory::default()
        };

        Self {
            memory: RwLock::new(HashMap::new()),
            campaign: RwLock::new(campaign),
            db_path,
        }
    }

    pub fn record_finding(&self, target: &str, finding: &str) {
        if let Ok(mut store) = self.memory.write() {
            store
                .entry(target.to_string())
                .or_default()
                .push(finding.to_string());
        }
    }

    pub fn get_findings(&self, target: &str) -> Option<Vec<String>> {
        self.memory.read().ok()?.get(target).cloned()
    }

    pub fn record_technique(&self, technique: &str, vendor: &str, success: bool, time_ms: u64) {
        if let Ok(mut cam) = self.campaign.write() {
            let rec = cam
                .techniques
                .entry(technique.to_string())
                .or_insert(TechniqueRecord {
                    technique: technique.to_string(),
                    target_vendor: vendor.to_string(),
                    success_count: 0,
                    fail_count: 0,
                    last_success: None,
                    avg_time_ms: 0,
                });
            if success {
                rec.success_count += 1;
                rec.last_success = Some(chrono::Utc::now().to_rfc3339());
                rec.avg_time_ms = if rec.avg_time_ms == 0 {
                    time_ms
                } else {
                    (rec.avg_time_ms + time_ms) / 2
                };
            } else {
                rec.fail_count += 1;
            }
        }
        self.persist();
    }

    pub fn get_prioritized_techniques(&self, vendor: &str) -> Vec<String> {
        let Ok(cam) = self.campaign.read() else {
            return vec![];
        };
        let mut techniques: Vec<(&str, f32)> = cam
            .techniques
            .iter()
            .filter(|(_, r)| r.target_vendor.is_empty() || r.target_vendor == vendor)
            .map(|(name, r)| {
                let total = r.success_count + r.fail_count;
                let rate = if total > 0 {
                    r.success_count as f32 / total as f32
                } else {
                    0.5
                };
                (name.as_str(), rate)
            })
            .collect();
        techniques.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        techniques
            .into_iter()
            .map(|(technique, _)| technique.to_string())
            .collect()
    }

    pub fn record_working_creds(&self, user: &str, pass: &str) {
        if let Ok(mut cam) = self.campaign.write() {
            let pair = (user.to_string(), pass.to_string());
            if !cam.known_credentials.contains(&pair) {
                cam.known_credentials.push(pair);
            }
        }
        self.persist();
    }

    pub fn get_working_creds(&self) -> Vec<(String, String)> {
        self.campaign
            .read()
            .ok()
            .map(|c| c.known_credentials.clone())
            .unwrap_or_default()
    }

    fn persist(&self) {
        if let Ok(cam) = self.campaign.read() {
            if let Ok(json) = serde_json::to_string_pretty(&*cam) {
                let _ = std::fs::write(&self.db_path, json);
            }
        }
    }
}

#[tauri::command]
pub fn get_technique_stats(feedback: State<'_, std::sync::Arc<FeedbackStore>>) -> Vec<TechniqueRecord> {
    feedback
        .campaign
        .read()
        .ok()
        .map(|c| c.techniques.values().cloned().collect())
        .unwrap_or_default()
}

#[tauri::command]
pub fn reset_campaign_memory(feedback: State<'_, std::sync::Arc<FeedbackStore>>) -> Result<(), String> {
    if let Ok(mut cam) = feedback.campaign.write() {
        *cam = CampaignMemory::default();
    }
    let _ = std::fs::remove_file(crate::get_vault_path().join("campaign_memory.json"));
    Ok(())
}
