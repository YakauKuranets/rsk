use std::collections::HashMap;
use std::sync::RwLock;

pub struct FeedbackStore {
    // Храним данные в формате: Target IP -> List of Findings
    pub memory: RwLock<HashMap<String, Vec<String>>>,
}

impl FeedbackStore {
    pub fn new() -> Self {
        Self {
            memory: RwLock::new(HashMap::new()),
        }
    }

    pub fn record_finding(&self, target: &str, finding: &str) {
        let mut store = self.memory.write().unwrap();
        store
            .entry(target.to_string())
            .or_insert_with(Vec::new)
            .push(finding.to_string());
        println!(
            "[FeedbackStore] 🧠 Запомнил новую уязвимость для {}: {}",
            target, finding
        );
    }

    pub fn get_findings(&self, target: &str) -> Option<Vec<String>> {
        let store = self.memory.read().unwrap();
        store.get(target).cloned()
    }
}
