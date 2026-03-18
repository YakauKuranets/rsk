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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    fn sample_finding() -> Finding {
        Finding {
            host: "192.168.1.100".to_string(),
            finding_type: FindingType::Exposure,
            severity: Severity::High,
            cve: Some("CVE-2021-36260".to_string()),
            cvss_score: Some(9.8),
            description: "Hikvision RCE".to_string(),
            evidence: Some("uid=0(root)".to_string()),
            confidence_score: 0.95,
        }
    }

    #[test]
    fn test_handoff_packet_serde_roundtrip() {
        let packet = HandoffPacket {
            pipeline_id: "test-pipeline-001".to_string(),
            from_agent: AgentId::ReconAgent,
            timestamp_utc: "2024-01-01T00:00:00Z".to_string(),
            scope_hash: "abc123".to_string(),
            permit_number: None,
            status: HandoffStatus::Success,
            findings: vec![sample_finding()],
            context_carry: serde_json::json!({"scope": "192.168.1.0/24"}),
            operator_notes: Some("Test run".to_string()),
            risk_indicators: vec!["externalExposureDetected".to_string()],
        };
        let json = serde_json::to_string(&packet).expect("serialize");
        let back: HandoffPacket = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.pipeline_id, packet.pipeline_id);
        assert_eq!(back.findings.len(), 1);
        assert_eq!(back.findings[0].host, "192.168.1.100");
        assert!(back.findings[0].cvss_score.is_some());
    }

    #[test]
    fn test_handoff_status_partial_serde() {
        let status = HandoffStatus::Partial {
            reason: "Shodan timeout".to_string(),
        };
        let json = serde_json::to_string(&status).expect("serialize");
        assert!(json.contains("partial"));
        assert!(json.contains("Shodan timeout"));
    }

    #[test]
    fn test_finding_severity_ordering() {
        let c = serde_json::to_string(&Severity::Critical).expect("serialize");
        let h = serde_json::to_string(&Severity::High).expect("serialize");
        assert!(c.contains("Critical"));
        assert!(h.contains("High"));
        assert_ne!(c, h);
    }

    #[test]
    fn test_agent_id_all_variants_serde() {
        let agents = vec![
            AgentId::ReconAgent,
            AgentId::ScanAgent,
            AgentId::ExploitVerifyAgent,
            AgentId::RiskAgent,
            AgentId::ReportAgent,
        ];
        for agent in agents {
            let json = serde_json::to_string(&agent).expect("serialize");
            let back: AgentId = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, agent);
        }
    }
}
