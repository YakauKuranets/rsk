use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowMode {
    DiscoveryMode,
    VerifiedMode,
    AnalysisMode,
    LabAgentMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryStatus {
    New,
    Profiling,
    AuthRequired,
    Promoted,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Pending,
    InProgress,
    Verified,
    Inconclusive,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityLevel {
    Unknown,
    Inferred,
    Confirmed,
    NotSupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryCard {
    pub ip: String,
    pub address: Option<String>,
    pub site_label: Option<String>,
    pub scan_profile: Option<String>,
    pub suspected_vendor: Option<String>,
    pub discovery_status: DiscoveryStatus,
    pub auth_required: bool,
    pub stream_capability: CapabilityLevel,
    pub archive_capability: CapabilityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedCard {
    pub ip: String,
    pub login: String,
    pub password: String,
    pub vendor_hint: Option<String>,
    pub stream_auth_mode: Option<String>,
    pub archive_auth_mode: Option<String>,
    pub verification_status: VerificationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotedCard {
    pub source_discovery_ip: String,
    pub promotion_reason: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetProfile {
    pub target_id: String,
    pub ip: String,
    pub mode: WorkflowMode,
    pub vendor_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityProfile {
    pub stream_capability: CapabilityLevel,
    pub archive_capability: CapabilityLevel,
    pub capture_capability: CapabilityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub evidence_id: String,
    pub source: String,
    pub summary: String,
    pub refs: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub finding_id: String,
    pub title: String,
    pub severity: String,
    pub confidence: f32,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub decision_id: String,
    pub goal: String,
    pub next_step: String,
    pub capability: String,
    pub reason: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trace {
    pub trace_id: String,
    pub mode: WorkflowMode,
    pub capability: String,
    pub input_hash: String,
    pub output_hash: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub recommendation_id: String,
    pub action: String,
    pub rationale: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowState {
    pub workflow_id: String,
    pub mode: WorkflowMode,
    pub stage: String,
    pub progress: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdictKind {
    Confirmed,
    Inconclusive,
    RetryNeeded,
    FalsePositive,
    ManualReviewRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewVerdict {
    pub verdict: ReviewVerdictKind,
    pub why: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotionCandidate {
    pub source_ip: String,
    pub reason: String,
    pub confidence: f32,
    pub required_fields: Vec<String>,
}
