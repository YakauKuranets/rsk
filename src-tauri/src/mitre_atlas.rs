// src-tauri/src/mitre_atlas.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MitreMapping {
    pub technique_id: String,
    pub tactic: String,
    pub technique_name: String,
    pub confidence: f32,
}

pub fn map_to_mitre(description: &str, protocol: Option<&str>) -> Vec<MitreMapping> {
    let desc = description.to_lowercase();
    let proto = protocol.unwrap_or("").to_lowercase();
    let rules: &[(&str, &str, &str, &str, f32)] = &[
        (
            "modbus",
            "T0801",
            "Lateral Movement",
            "Lateral Tool Transfer",
            0.9,
        ),
        (
            "bacnet",
            "T0801",
            "Lateral Movement",
            "Lateral Tool Transfer",
            0.9,
        ),
        (
            "dnp3",
            "T0835",
            "Lateral Movement",
            "Manipulate I/O Image",
            0.85,
        ),
        (
            "default password",
            "T1078.001",
            "Initial Access",
            "Default Accounts",
            0.95,
        ),
        (
            "default cred",
            "T1078.001",
            "Initial Access",
            "Default Accounts",
            0.95,
        ),
        (
            "brute force",
            "T1110",
            "Credential Access",
            "Brute Force",
            0.9,
        ),
        (
            "rce",
            "T1190",
            "Initial Access",
            "Exploit Public-Facing App",
            0.95,
        ),
        (
            "command injection",
            "T1059",
            "Execution",
            "Command Interpreter",
            0.9,
        ),
        (
            "lateral",
            "T1021",
            "Lateral Movement",
            "Remote Services",
            0.8,
        ),
        (
            "unencrypted",
            "T1557",
            "Credential Access",
            "Adversary-in-the-Middle",
            0.8,
        ),
        (
            "cleartext",
            "T1557",
            "Credential Access",
            "Adversary-in-the-Middle",
            0.8,
        ),
        ("rtsp", "T1125", "Collection", "Video Capture", 0.9),
        ("camera", "T1125", "Collection", "Video Capture", 0.85),
        (
            "public s3",
            "T1530",
            "Collection",
            "Data from Cloud Storage",
            0.95,
        ),
        (
            "privileged container",
            "T1611",
            "Privilege Escalation",
            "Escape to Host",
            0.95,
        ),
        (
            "docker socket",
            "T1611",
            "Privilege Escalation",
            "Escape to Host",
            0.95,
        ),
        (
            "firmware",
            "T1195.003",
            "Initial Access",
            "Compromise Hardware",
            0.7,
        ),
        (
            "cve",
            "T1190",
            "Initial Access",
            "Exploit Public-Facing App",
            0.85,
        ),
        ("snmp", "T1602.001", "Collection", "SNMP", 0.85),
        ("ssh", "T1021.004", "Lateral Movement", "SSH", 0.85),
        (
            "ftp",
            "T1021.002",
            "Lateral Movement",
            "Remote Services FTP",
            0.75,
        ),
        (
            "iam no mfa",
            "T1078.004",
            "Persistence",
            "Cloud Accounts",
            0.9,
        ),
        (
            "privilege escalation",
            "T1068",
            "Privilege Escalation",
            "Exploitation for PE",
            0.85,
        ),
    ];
    let mut out: Vec<MitreMapping> = rules
        .iter()
        .filter(|&&(kw, ..)| desc.contains(kw) || proto.contains(kw))
        .map(|&(_, id, tac, name, conf)| MitreMapping {
            technique_id: id.to_string(),
            tactic: tac.to_string(),
            technique_name: name.to_string(),
            confidence: conf,
        })
        .collect();
    out.dedup_by(|a, b| a.technique_id == b.technique_id);
    out.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(3);
    out
}

#[tauri::command]
pub fn map_findings_to_mitre(findings_json: String) -> Result<Vec<MitreMapping>, String> {
    let findings: Vec<serde_json::Value> =
        serde_json::from_str(&findings_json).map_err(|e| e.to_string())?;
    let mut all: Vec<MitreMapping> = findings
        .iter()
        .flat_map(|f| {
            let desc = f["description"].as_str().unwrap_or("");
            let proto = f["protocol"].as_str();
            map_to_mitre(desc, proto)
        })
        .collect();
    all.dedup_by(|a, b| a.technique_id == b.technique_id);
    Ok(all)
}
