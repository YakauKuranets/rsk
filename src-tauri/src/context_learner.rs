// src-tauri/src/context_learner.rs
// НОВЫЙ ФАЙЛ — создать с нуля


use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::State;


// ── Контекстный вектор цели ─────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetContext {
    pub ip_hash: String,           // sha256(ip)
    pub vendor: String,
    pub firmware_major: u8,        // из "V5.4.2" → 5
    pub firmware_minor: u8,        // из "V5.4.2" → 4
    pub open_ports: Vec<u16>,
    pub has_waf: bool,
    pub has_auth: bool,
    pub geo_country: String,
    pub timestamp: String,
}


// ── Исторический снапшот: контекст + что сработало ─────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalSnapshot {
    pub context: TargetContext,
    pub successful_techniques: Vec<String>,
    pub avg_reward: f32,
    pub campaign_id: String,
}


// ── Результат k-NN поиска ────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnnResult {
    pub similar_count: usize,
    pub recommended_techniques: Vec<String>,
    pub avg_similarity: f32,
    pub top_match_vendor: String,
}


// ── Состояние хранилища ──────────────────────────────────────
pub struct ContextStore {
    pub snapshots: std::sync::RwLock<Vec<HistoricalSnapshot>>,
    db_path: std::path::PathBuf,
}


impl ContextStore {
    pub fn new() -> Self {
        let db_path = crate::get_vault_path().join("context_store.json");
        let snapshots = if db_path.exists() {
            std::fs::read_to_string(&db_path)
                .ok().and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else { Vec::new() };
        Self { snapshots: std::sync::RwLock::new(snapshots), db_path }
    }


    // ── Добавить снапшот ────────────────────────────────────
    pub fn record_snapshot(&self, snap: HistoricalSnapshot) {
        if let Ok(mut snaps) = self.snapshots.write() {
            if let Some(e) = snaps.iter_mut()
                .find(|s| s.context.ip_hash == snap.context.ip_hash)
            { *e = snap; }
            else {
                if snaps.len() >= 1000 { snaps.remove(0); }
                snaps.push(snap);
            }
        }
        self.persist();
    }


    // ── k-NN поиск: Jaccard по портам + vendor ──────────────
    pub fn find_knn(
        &self,
        context: &TargetContext,
        k: usize,
    ) -> KnnResult {
        let Ok(snaps) = self.snapshots.read() else {
            return KnnResult { similar_count:0, recommended_techniques:vec![], avg_similarity:0.0, top_match_vendor:String::new() };
        };


        let my_ports: HashSet<u16> = context.open_ports.iter().cloned().collect();


        let mut scored: Vec<(f32, &HistoricalSnapshot)> = snaps.iter()
            .map(|s| {
                let their_ports: HashSet<u16> = s.context.open_ports.iter().cloned().collect();
                // Jaccard similarity для портов
                let inter = my_ports.intersection(&their_ports).count() as f32;
                let union = my_ports.union(&their_ports).count() as f32;
                let port_sim = if union > 0.0 { inter / union } else { 0.0 };
                // Vendor: полное совпадение = +0.35
                let vendor_sim = if s.context.vendor.to_lowercase() == context.vendor.to_lowercase()
                    { 0.35 } else { 0.0 };
                // Firmware major: совпадение = +0.15
                let fw_sim = if s.context.firmware_major == context.firmware_major
                    { 0.15 } else { 0.0 };
                // Итоговое взвешенное сходство
                let sim = port_sim * 0.5 + vendor_sim + fw_sim;
                (sim, s)
            })
            .filter(|(s, _)| *s > 0.15)
            .collect();


        scored.sort_by(|a,b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let top_k: Vec<_> = scored.into_iter().take(k).collect();


        // Собрать техники из похожих снапшотов
        let mut tech_freq: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let mut top_vendor = String::new();
        let mut best_sim = 0.0f32;


        for (sim, snap) in &top_k {
            for t in &snap.successful_techniques {
                *tech_freq.entry(t.clone()).or_insert(0) += 1;
            }
            if *sim > best_sim { best_sim = *sim; top_vendor = snap.context.vendor.clone(); }
        }


        // Отсортировать по частоте
        let mut techs: Vec<(String, u32)> = tech_freq.into_iter().collect();
        techs.sort_by(|a,b| b.1.cmp(&a.1));


        let avg_sim = if top_k.is_empty() { 0.0 }
            else { top_k.iter().map(|(s,_)| s).sum::<f32>() / top_k.len() as f32 };


        KnnResult {
            similar_count: top_k.len(),
            recommended_techniques: techs.into_iter().map(|(t,_)| t).take(6).collect(),
            avg_similarity: avg_sim,
            top_match_vendor: top_vendor,
        }
    }


    // ── Разобрать версию прошивки ────────────────────────────
    pub fn parse_firmware(version: &str) -> (u8, u8) {
        let parts: Vec<&str> = version.trim_start_matches('V')
            .split('.').collect();
        let major = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        (major, minor)
    }


    // ── Персистентность ──────────────────────────────────────
    fn persist(&self) {
        if let Ok(snaps) = self.snapshots.read() {
            if let Ok(json) = serde_json::to_string_pretty(&*snaps) {
                let _ = std::fs::write(&self.db_path, json);
            }
        }
    }
}


// ── Tauri команды ────────────────────────────────────────────
#[tauri::command]
pub fn context_find_similar(
    vendor: String,
    ports: Vec<u16>,
    firmware: String,
    k: Option<usize>,
    ctx_store: State<'_, std::sync::Arc<ContextStore>>,
) -> KnnResult {
    let (fw_major, fw_minor) = ContextStore::parse_firmware(&firmware);
    let context = TargetContext {
        ip_hash: "query".to_string(),
        vendor: vendor.clone(),
        firmware_major: fw_major,
        firmware_minor: fw_minor,
        open_ports: ports,
        has_waf: false, has_auth: true,
        geo_country: String::new(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    ctx_store.find_knn(&context, k.unwrap_or(5))
}


#[tauri::command]
pub fn context_get_stats(
    ctx_store: State<'_, std::sync::Arc<ContextStore>>,
) -> serde_json::Value {
    let count = ctx_store.snapshots.read()
        .map(|s| s.len()).unwrap_or(0);
    serde_json::json!({ "total_snapshots": count })
}
