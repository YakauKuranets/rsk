// src-tauri/src/continuous_monitor.rs
// Real-time attack surface monitoring with diff alerts
use crate::agents::handoff::{Finding, FindingType, Severity};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorJob {
    pub job_id: String,
    pub scope: String,
    pub interval_minutes: u64,
    pub shodan_key: Option<String>,
    pub permit_token: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceChange {
    pub change_type: String,
    pub host: String,
    pub description: String,
    pub severity: String,
    pub detected_at: String,
}

pub struct MonitorState {
    pub jobs: Mutex<HashMap<String, MonitorJob>>,
    pub last_findings: Mutex<HashMap<String, Vec<Finding>>>,
    pub running: Mutex<HashSet<String>>,
}

impl MonitorState {
    pub fn new() -> Self {
        Self {
            jobs: Mutex::new(HashMap::new()),
            last_findings: Mutex::new(HashMap::new()),
            running: Mutex::new(HashSet::new()),
        }
    }
}

pub fn diff_findings(prev: &[Finding], curr: &[Finding]) -> Vec<SurfaceChange> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut changes = Vec::new();

    let prev_hosts: HashSet<&str> = prev.iter().map(|f| f.host.as_str()).collect();
    let curr_hosts: HashSet<&str> = curr.iter().map(|f| f.host.as_str()).collect();

    // New hosts appeared
    for h in curr_hosts.difference(&prev_hosts) {
        changes.push(SurfaceChange {
            change_type: "new_host".to_string(),
            host: h.to_string(),
            description: format!("New host discovered: {}", h),
            severity: "HIGH".to_string(),
            detected_at: now.clone(),
        });
    }

    // Hosts gone offline
    for h in prev_hosts.difference(&curr_hosts) {
        changes.push(SurfaceChange {
            change_type: "host_gone".to_string(),
            host: h.to_string(),
            description: format!("Host no longer responding: {}", h),
            severity: "INFO".to_string(),
            detected_at: now.clone(),
        });
    }

    // New CVEs on existing hosts
    let prev_cves: HashSet<String> = prev
        .iter()
        .filter_map(|f| f.cve.as_ref().map(|c| format!("{}:{}", f.host, c)))
        .collect();

    for f in curr {
        if let Some(cve) = &f.cve {
            let key = format!("{}:{}", f.host, cve);
            if !prev_cves.contains(&key) {
                let sev = match f.severity {
                    Severity::Critical => "CRITICAL",
                    Severity::High => "HIGH",
                    _ => "MEDIUM",
                };
                changes.push(SurfaceChange {
                    change_type: "new_vuln".to_string(),
                    host: f.host.clone(),
                    description: format!("New {} on {}: {}", cve, f.host, f.description),
                    severity: sev.to_string(),
                    detected_at: now.clone(),
                });
            }
        }
    }

    // Severity escalation
    let prev_map: HashMap<(&str, &str), &Severity> = prev
        .iter()
        .filter_map(|f| f.cve.as_ref().map(|c| ((f.host.as_str(), c.as_str()), &f.severity)))
        .collect();

    for f in curr {
        if let Some(cve) = &f.cve {
            if let Some(prev_sev) = prev_map.get(&(f.host.as_str(), cve.as_str())) {
                let prev_score = severity_ord(prev_sev);
                let curr_score = severity_ord(&f.severity);
                if curr_score > prev_score {
                    changes.push(SurfaceChange {
                        change_type: "severity_escalated".to_string(),
                        host: f.host.clone(),
                        description: format!("{} severity increased on {}", cve, f.host),
                        severity: "HIGH".to_string(),
                        detected_at: now.clone(),
                    });
                }
            }
        }
    }

    changes
}

fn severity_ord(s: &Severity) -> u8 {
    match s {
        Severity::Info => 0,
        Severity::Low => 1,
        Severity::Medium => 2,
        Severity::High => 3,
        Severity::Critical => 4,
    }
}

#[tauri::command]
pub async fn start_monitor_job(
    job: MonitorJob,
    app: AppHandle,
    monitor_state: State<'_, MonitorState>,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    let job_id = job.job_id.clone();
    let interval_mins = job.interval_minutes.max(1);

    {
        let mut jobs = monitor_state.jobs.lock().map_err(|_| "lock")?;
        jobs.insert(job_id.clone(), job.clone());
    }
    {
        let mut running = monitor_state.running.lock().map_err(|_| "lock")?;
        running.insert(job_id.clone());
    }

    crate::push_runtime_log(
        &log_state,
        format!(
            "MONITOR_START|id={}|scope={}|interval={}m",
            job_id, job.scope, interval_mins
        ),
    );

    let scope = job.scope.clone();
    let shodan = job.shodan_key.clone();
    let jid = job_id.clone();

    // Spawn monitoring loop in background
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(interval_mins * 60));
        ticker.tick().await; // skip first immediate tick

        loop {
            ticker.tick().await;

            // Break if job was stopped
            let should_run = app
                .state::<MonitorState>()
                .jobs
                .lock()
                .map(|jobs| jobs.contains_key(&jid))
                .unwrap_or(false);
            if !should_run {
                break;
            }

            let pid = format!("monitor_{}_{}", jid, chrono::Utc::now().timestamp());
            let curr_findings = match run_quick_scan(&scope, &shodan, &pid).await {
                Ok(f) => f,
                Err(e) => {
                    let _ = app.emit(
                        "monitor-error",
                        serde_json::json!({"job_id": jid, "error": e}),
                    );
                    continue;
                }
            };

            // Diff against previous state from shared monitor state
            let prev = app
                .state::<MonitorState>()
                .last_findings
                .lock()
                .ok()
                .and_then(|m| m.get(&jid).cloned())
                .unwrap_or_default();

            let changes = diff_findings(&prev, &curr_findings);

            // Emit only if something changed
            if !changes.is_empty() {
                let payload = serde_json::json!({
                    "job_id": jid,
                    "scope": scope,
                    "changes": changes,
                    "total_findings": curr_findings.len(),
                    "scanned_at": chrono::Utc::now().to_rfc3339(),
                });
                let _ = app.emit("surface-change", payload);
            }

            if let Ok(mut lf) = app.state::<MonitorState>().last_findings.lock() {
                lf.insert(jid.clone(), curr_findings);
            }
        }

        if let Ok(mut running) = app.state::<MonitorState>().running.lock() {
            running.remove(&jid);
        }
    });

    Ok(job_id)
}

/// Lightweight recon scan for monitoring (no full pipeline)
async fn run_quick_scan(
    scope: &str,
    _shodan_key: &Option<String>,
    _pipeline_id: &str,
) -> Result<Vec<Finding>, String> {
    let report = crate::asset_discovery::discover_assets(scope.to_string())
        .await
        .unwrap_or_else(|_| crate::asset_discovery::AssetDiscoveryReport {
            query: scope.to_string(),
            total_assets: 0,
            assets: vec![],
            certificates: vec![],
            dns_records: vec![],
            duration_ms: 0,
        });

    let findings: Vec<Finding> = report
        .assets
        .iter()
        .map(|a| Finding {
            host: a.ip.clone(),
            finding_type: FindingType::Asset,
            severity: if a.open_ports.contains(&22) || a.open_ports.contains(&3389) {
                Severity::High
            } else {
                Severity::Info
            },
            cve: None,
            cvss_score: None,
            description: format!("Asset {} ports={:?}", a.ip, a.open_ports),
            evidence: None,
            confidence_score: 0.8,
        })
        .collect();

    Ok(findings)
}

#[tauri::command]
pub fn stop_monitor_job(
    job_id: String,
    monitor_state: State<'_, MonitorState>,
    log_state: State<'_, crate::LogState>,
) -> Result<(), String> {
    let mut jobs = monitor_state.jobs.lock().map_err(|_| "lock")?;
    jobs.remove(&job_id);

    if let Ok(mut running) = monitor_state.running.lock() {
        running.remove(&job_id);
    }

    crate::push_runtime_log(&log_state, format!("MONITOR_STOP|id={}", job_id));
    Ok(())
}

#[tauri::command]
pub fn list_monitor_jobs(monitor_state: State<'_, MonitorState>) -> Result<Vec<MonitorJob>, String> {
    let jobs = monitor_state.jobs.lock().map_err(|_| "lock")?;
    Ok(jobs.values().cloned().collect())
}

#[tauri::command]
pub fn get_surface_snapshot(
    job_id: String,
    monitor_state: State<'_, MonitorState>,
) -> Result<Vec<Finding>, String> {
    let lf = monitor_state.last_findings.lock().map_err(|_| "lock")?;
    Ok(lf.get(&job_id).cloned().unwrap_or_default())
}
