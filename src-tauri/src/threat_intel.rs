// src-tauri/src/threat_intel.rs
use crate::agents::handoff::{Finding, HandoffPacket, Severity};
use crate::mitre_atlas::map_to_mitre;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostThreatProfile {
    pub ip: String,
    pub threat_score: f32,
    pub risk_level: String,
    pub top_technique_id: Option<String>,
    pub top_tactic: Option<String>,
    pub kev_count: u32,
    pub factors: Vec<String>,
    pub remediation_priority: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatIntelReport {
    pub profiles: Vec<HostThreatProfile>,
    pub critical_count: usize,
    pub total_kev: u32,
    pub top_tactic: String,
}

fn sev_score(s: &Severity) -> f32 {
    match s {
        Severity::Critical => 9.5,
        Severity::High => 7.5,
        Severity::Medium => 5.0,
        Severity::Low => 2.5,
        Severity::Info => 0.5,
    }
}

pub fn build_threat_report(packet: &HandoffPacket) -> ThreatIntelReport {
    let mut by_host: HashMap<String, Vec<&Finding>> = HashMap::new();
    for f in &packet.findings {
        by_host.entry(f.host.clone()).or_default().push(f);
    }
    let mut profiles = Vec::new();
    let mut total_kev = 0u32;
    let mut tactic_counts: HashMap<String, usize> = HashMap::new();

    for (ip, findings) in &by_host {
        let mut factors = Vec::new();
        let mut base: f32 = 0.0;
        let mut kev = 0u32;
        let mut top_tech: Option<String> = None;
        let mut top_tac: Option<String> = None;

        for f in findings {
            let s = f.cvss_score.unwrap_or_else(|| sev_score(&f.severity));
            if s > base {
                base = s;
            }

            if let Some(ref cve) = f.cve {
                let is_kev = packet.context_carry["kev_ids"]
                    .as_array()
                    .map(|a| a.iter().any(|k| k.as_str() == Some(cve.as_str())))
                    .unwrap_or(false);
                if is_kev {
                    kev += 1;
                    factors.push(format!("{} actively exploited (CISA KEV)", cve));
                }
            }

            let mitre = map_to_mitre(&f.description, None);
            if let Some(m) = mitre.first() {
                *tactic_counts.entry(m.tactic.clone()).or_insert(0) += 1;
                if top_tech.is_none() {
                    top_tech = Some(m.technique_id.clone());
                    top_tac = Some(m.tactic.clone());
                }
            }

            match f.severity {
                Severity::Critical | Severity::High => {
                    factors.push(format!("{:?}: {}", f.severity, f.description))
                }
                _ => {}
            }
        }

        total_kev += kev;
        if kev > 0 {
            base = (base + 1.5).min(10.0);
        }
        let risk = match base as u8 {
            9..=10 => "CRITICAL",
            7..=8 => "HIGH",
            4..=6 => "MEDIUM",
            1..=3 => "LOW",
            _ => "INFO",
        };

        profiles.push(HostThreatProfile {
            ip: ip.clone(),
            threat_score: base,
            risk_level: risk.to_string(),
            top_technique_id: top_tech,
            top_tactic: top_tac,
            kev_count: kev,
            factors,
            remediation_priority: if base >= 9.0 {
                1
            } else if base >= 7.0 {
                2
            } else {
                3
            },
        });
    }

    profiles.sort_by(|a, b| {
        b.threat_score
            .partial_cmp(&a.threat_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_tactic = tactic_counts
        .iter()
        .max_by_key(|(_, v)| *v)
        .map(|(k, _)| k.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    ThreatIntelReport {
        critical_count: profiles.iter().filter(|p| p.risk_level == "CRITICAL").count(),
        profiles,
        total_kev,
        top_tactic,
    }
}

#[tauri::command]
pub fn analyze_threat_intelligence(
    packet_json: String,
    log_state: State<'_, crate::LogState>,
) -> Result<ThreatIntelReport, String> {
    let packet: HandoffPacket = serde_json::from_str(&packet_json).map_err(|e| e.to_string())?;
    crate::push_runtime_log(&log_state, format!("THREAT_INTEL|findings={}", packet.findings.len()));
    Ok(build_threat_report(&packet))
}
