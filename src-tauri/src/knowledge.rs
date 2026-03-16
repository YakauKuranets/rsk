use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceExperience {
    pub vendor: String,
    pub successful_path: String,
    pub login: String,
    pub pass: String,
    pub last_seen: i64,
}

pub struct KnowledgeManager {
    db_path: PathBuf,
}

impl KnowledgeManager {
    pub fn new() -> Self {
        let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        path.push("hyperion_knowledge.json");
        Self { db_path: path }
    }

    pub fn load_all(&self) -> HashMap<String, DeviceExperience> {
        if let Ok(content) = fs::read_to_string(&self.db_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        }
    }

    pub fn save_success(&self, ip: &str, vendor: &str, path: &str, login: &str, pass: &str) {
        let mut data = self.load_all();
        let exp = DeviceExperience {
            vendor: vendor.to_string(),
            successful_path: path.to_string(),
            login: login.to_string(),
            pass: pass.to_string(),
            last_seen: Utc::now().timestamp(),
        };
        data.insert(ip.to_string(), exp);

        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = fs::write(&self.db_path, json);
        }
    }
}
