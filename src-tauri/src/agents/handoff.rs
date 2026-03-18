use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HandoffPacket {
    pub pipeline_id: String,
    pub from_agent: AgentId,
    pub timestamp_utc: String,
    pub scope_hash: String,
    pub permit_number: Option<String>,
    pub status: HandoffStatus,
    pub findings: Vec<Finding>,
    pub context_carry: serde_json::Value,
    pub operator_notes: Option<String>,
    pub risk_indicators: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AgentId {
    ReconAgent,
    ScanAgent,
    ExploitVerifyAgent,
    RiskAgent,
    ReportAgent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HandoffStatus {
    Success,
    Partial { reason: String },
    Error { message: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FindingType {
    Asset,
    Exposure,
    Geolocation,
    Vulnerability,
    Intelligence,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub host: String,
    pub finding_type: FindingType,
    pub severity: Severity,
    pub cve: Option<String>,
    pub cvss_score: Option<f32>,
    pub description: String,
    pub evidence: Option<String>,
    pub confidence_score: f32,
}
