use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playbook {
    pub name: String,
    pub description: Option<String>,
    pub scope: PlaybookScope,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    pub steps: Vec<PlaybookStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybookScope {
    pub targets: Vec<String>,
    #[serde(default)]
    pub excluded: Vec<String>,
    pub max_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybookStep {
    pub id: String,
    pub name: String,
    pub module: String,
    #[serde(default)]
    pub params: HashMap<String, serde_yaml::Value>,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    pub use_output_from: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    pub step_id: String,
    pub step_name: String,
    pub module: String,
    pub status: StepStatus,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepStatus {
    Pending,
    WaitingApproval,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

const KNOWN_MODULES: &[&str] = &[
    "camera_scan",
    "port_scan",
    "credential_audit",
    "vuln_scan",
    "spider",
    "archive_search",
    "archive_download",
    "security_headers",
    "metadata_collect",
    "mass_audit",
    "compliance_check",
    "report_generate",
    "asset_discovery",
];

pub fn parse_playbook(yaml_content: &str) -> Result<Playbook, String> {
    serde_yaml::from_str::<Playbook>(yaml_content).map_err(|e| format!("YAML parse error: {e}"))
}

pub fn validate_playbook(pb: &Playbook) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if pb.steps.is_empty() {
        errors.push("Playbook must contain at least one step".to_string());
    }
    if pb.scope.targets.is_empty() {
        errors.push("Playbook scope.targets must not be empty".to_string());
    }

    let step_ids: HashSet<String> = pb.steps.iter().map(|s| s.id.clone()).collect();
    for step in &pb.steps {
        if !KNOWN_MODULES.contains(&step.module.as_str()) {
            errors.push(format!("Unknown module '{}' in step '{}'", step.module, step.id));
        }

        if let Some(dep) = &step.use_output_from {
            if !step_ids.contains(dep) {
                errors.push(format!("Step '{}' references unknown use_output_from '{}'", step.id, dep));
            }
        }

        if ["credential_audit", "mass_audit", "spider"].contains(&step.module.as_str())
            && !step.requires_approval
        {
            errors.push(format!(
                "Step '{}' module '{}' requires approval gate",
                step.id, step.module
            ));
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
