// src-tauri/src/feedback_store.rs
// ПОЛНАЯ ЗАМЕНА — скопировать целиком


use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use tauri::State;


// ── Причины провала техники ─────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FailureReason {
    WrongCredentials,
    PatchedVulnerability,
    WafBlocked,
    RateLimit,
    Timeout,
    AuthRequired,
    NotDetected,
}


// ── Условия при которых техника сработала ───────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessCondition {
    pub vendor: String,
    pub firmware_pattern: String,
    pub open_ports: Vec<u16>,
    pub timestamp: String,
}


// ── Результат выполнения одной техники (многомерный) ───────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechniqueOutcome {
    pub success: bool,
    pub got_credentials: bool,
    pub got_archive: bool,
    pub got_stream: bool,
    pub no_alert: bool,
    pub failure_reason: Option<FailureReason>,
    pub time_ms: u64,
    pub condition: Option<SuccessCondition>,
}


// ── Профиль устройства для k-NN поиска ─────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProfile {
    pub ip_hash: String,
    pub vendor: String,
    pub firmware_version: String,
    pub open_ports: Vec<u16>,
    pub has_waf: bool,
    pub has_auth: bool,
    pub successful_techniques: Vec<String>,
    pub last_seen: String,
}


// ── Запись по технике (расширенная) ────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechniqueRecord {
    // Старые поля (совместимость)
    pub technique: String,
    pub target_vendor: String,
    pub success_count: u32,
    pub fail_count: u32,
    pub last_success: Option<String>,
    pub avg_time_ms: u64,
    // Новые поля
    pub avg_reward: f32,
    pub failure_reasons: HashMap<String, u32>,
    pub conditions: Vec<SuccessCondition>,
    pub total_trials: u32,
}


impl Default for TechniqueRecord {
    fn default() -> Self {
        Self {
            technique: String::new(), target_vendor: String::new(),
            success_count: 0, fail_count: 0, last_success: None, avg_time_ms: 0,
            avg_reward: 0.5, failure_reasons: HashMap::new(),
            conditions: Vec::new(), total_trials: 0,
        }
    }
}


// ── Память всей кампании ────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CampaignMemory {
    pub techniques: HashMap<String, TechniqueRecord>,
    pub known_credentials: Vec<(String, String)>,
    pub vulnerable_paths: Vec<String>,
    pub blocked_vendors: Vec<String>,
    pub device_profiles: Vec<DeviceProfile>,
    pub total_global_trials: u32,
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
                .ok().and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else { CampaignMemory::default() };
        Self { memory: RwLock::new(HashMap::new()), campaign: RwLock::new(campaign), db_path }
    }


    // ── Finding (без изменений) ─────────────────────────────────────
    pub fn record_finding(&self, target: &str, finding: &str) {
        if let Ok(mut s) = self.memory.write() {
            s.entry(target.to_string()).or_default().push(finding.to_string());
        }
    }


    pub fn get_findings(&self, target: &str) -> Option<Vec<String>> {
        self.memory.read().ok()?.get(target).cloned()
    }


    // ── Старый API (оставить для совместимости) ─────────────────────
    pub fn record_technique(&self, technique: &str, vendor: &str, success: bool, time_ms: u64) {
        let outcome = TechniqueOutcome {
            success, got_credentials: false, got_archive: false,
            got_stream: false, no_alert: success,
            failure_reason: if !success { Some(FailureReason::NotDetected) } else { None },
            time_ms, condition: None,
        };
        self.record_outcome(technique, vendor, &outcome);
    }


    // ── НОВЫЙ метод: многомерный reward + UCB обновление ────────────
    pub fn record_outcome(&self, technique: &str, vendor: &str, outcome: &TechniqueOutcome) {
        let reward = Self::calculate_reward(outcome);
        if let Ok(mut cam) = self.campaign.write() {
            cam.total_global_trials += 1;
            let rec = cam.techniques.entry(technique.to_string())
                .or_insert_with(|| TechniqueRecord {
                    technique: technique.to_string(),
                    target_vendor: vendor.to_string(),
                    ..TechniqueRecord::default()
                });
            rec.total_trials += 1;
            if outcome.success {
                rec.success_count += 1;
                rec.last_success = Some(chrono::Utc::now().to_rfc3339());
            } else {
                rec.fail_count += 1;
            }
            // EMA alpha=0.3
            rec.avg_reward = 0.7 * rec.avg_reward + 0.3 * reward;
            if outcome.time_ms > 0 {
                rec.avg_time_ms = if rec.avg_time_ms == 0 { outcome.time_ms }
                    else { (rec.avg_time_ms + outcome.time_ms) / 2 };
            }
            if outcome.success {
                if let Some(cond) = &outcome.condition {
                    if rec.conditions.len() < 20 { rec.conditions.push(cond.clone()); }
                }
            }
            if let Some(reason) = &outcome.failure_reason {
                let key = format!("{:?}", reason);
                *rec.failure_reasons.entry(key).or_insert(0) += 1;
            }
        }
        self.persist();
    }


    // ── Reward: диапазон 0.0..2.0 ───────────────────────────────────
    fn calculate_reward(o: &TechniqueOutcome) -> f32 {
        let mut r = 0.0f32;
        if o.success         { r += 1.0; }
        if o.got_credentials { r += 0.4; }
        if o.got_archive     { r += 0.5; }
        if o.got_stream      { r += 0.3; }
        if o.no_alert        { r += 0.2; }
        r -= (o.time_ms as f32 / 30_000.0).min(0.3);
        r.clamp(0.0, 2.0)
    }


    // ── UCB score для одной техники ──────────────────────────────────
    pub fn get_ucb_score(&self, technique: &str, vendor: &str) -> f32 {
        let Ok(cam) = self.campaign.read() else { return 1.0; };
        let total = cam.total_global_trials.max(1) as f32;
        let Some(rec) = cam.techniques.get(technique) else { return f32::MAX; };
        if !rec.target_vendor.is_empty() && rec.target_vendor != vendor { return 0.0; }
        let n = rec.total_trials.max(1) as f32;
        // UCB1: avg_reward + C * sqrt(ln(total) / n)
        rec.avg_reward + 1.4 * (total.ln() / n).sqrt()
    }


    // ── Выбор техники: UCB + epsilon-greedy ─────────────────────────
    pub fn select_technique_ucb(
        &self, vendor: &str, candidates: &[String], epsilon: f32,
    ) -> String {
        if candidates.is_empty() { return "default_creds".to_string(); }
        let rand_val: f32 = rand::random();
        if rand_val < epsilon {
            // Exploration: случайная техника
            let idx = (rand::random::<f32>() * candidates.len() as f32) as usize;
            return candidates[idx.min(candidates.len() - 1)].clone();
        }
        // Exploitation: UCB максимум
        candidates.iter()
            .map(|t| (t, self.get_ucb_score(t, vendor)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(t, _)| t.clone())
            .unwrap_or_else(|| candidates[0].clone())
    }


    // ── Старый метод (совместимость) ────────────────────────────────
    pub fn get_prioritized_techniques(&self, vendor: &str) -> Vec<String> {
        let Ok(cam) = self.campaign.read() else { return vec![]; };
        let total = cam.total_global_trials.max(1) as f32;
        let mut scored: Vec<(String, f32)> = cam.techniques.iter()
            .filter(|(_, r)| r.target_vendor.is_empty() || r.target_vendor == vendor)
            .map(|(name, r)| {
                let n = r.total_trials.max(1) as f32;
                let ucb = r.avg_reward + 1.4 * (total.ln() / n).sqrt();
                (name.clone(), ucb)
            })
            .collect();
        scored.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().map(|(t,_)| t).collect()
    }


    // ── Сохранить профиль устройства ─────────────────────────────────
    pub fn record_device_profile(&self, profile: DeviceProfile) {
        if let Ok(mut cam) = self.campaign.write() {
            if let Some(e) = cam.device_profiles.iter_mut()
                .find(|p| p.ip_hash == profile.ip_hash) { *e = profile; }
            else {
                if cam.device_profiles.len() >= 500 { cam.device_profiles.remove(0); }
                cam.device_profiles.push(profile);
            }
        }
        self.persist();
    }


    // ── k-NN: найти k похожих устройств через Jaccard по портам ─────
    pub fn find_similar_devices(
        &self, ports: &[u16], vendor: &str, k: usize,
    ) -> Vec<DeviceProfile> {
        let Ok(cam) = self.campaign.read() else { return vec![]; };
        let port_set: std::collections::HashSet<u16> = ports.iter().cloned().collect();
        let mut scored: Vec<(f32, &DeviceProfile)> = cam.device_profiles.iter()
            .map(|p| {
                let p_set: std::collections::HashSet<u16> =
                    p.open_ports.iter().cloned().collect();
                let inter = port_set.intersection(&p_set).count() as f32;
                let union = port_set.union(&p_set).count() as f32;
                let port_sim = if union > 0.0 { inter / union } else { 0.0 };
                let vendor_bonus = if p.vendor.to_lowercase() == vendor.to_lowercase()
                    { 0.4 } else { 0.0 };
                (port_sim * 0.6 + vendor_bonus, p)
            })
            .filter(|(s, _)| *s > 0.1)
            .collect();
        scored.sort_by(|a,b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(k).map(|(_,p)| p.clone()).collect()
    }


    // ── Credentials ──────────────────────────────────────────────────
    pub fn record_working_creds(&self, user: &str, pass: &str) {
        if let Ok(mut cam) = self.campaign.write() {
            let pair = (user.to_string(), pass.to_string());
            if !cam.known_credentials.contains(&pair) {
                cam.known_credentials.push(pair); }
        }
        self.persist();
    }


    pub fn get_working_creds(&self) -> Vec<(String, String)> {
        self.campaign.read().ok().map(|c| c.known_credentials.clone()).unwrap_or_default()
    }


    pub fn get_total_trials(&self) -> u32 {
        self.campaign.read().ok().map(|c| c.total_global_trials).unwrap_or(0)
    }


    // ── Персистентность ──────────────────────────────────────────────
    fn persist(&self) {
        if let Ok(cam) = self.campaign.read() {
            if let Ok(json) = serde_json::to_string_pretty(&*cam) {
                let _ = std::fs::write(&self.db_path, json); }
        }
    }
}


// ── Tauri команды ────────────────────────────────────────────────────
#[tauri::command]
pub fn get_technique_stats(
    feedback: State<'_, std::sync::Arc<FeedbackStore>>
) -> Vec<TechniqueRecord> {
    feedback.campaign.read().ok()
        .map(|c| c.techniques.values().cloned().collect())
        .unwrap_or_default()
}


#[tauri::command]
pub fn reset_campaign_memory(
    feedback: State<'_, std::sync::Arc<FeedbackStore>>
) -> Result<(), String> {
    if let Ok(mut cam) = feedback.campaign.write() { *cam = CampaignMemory::default(); }
    let _ = std::fs::remove_file(crate::get_vault_path().join("campaign_memory.json"));
    Ok(())
}
