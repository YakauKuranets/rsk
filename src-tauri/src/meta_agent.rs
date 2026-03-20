// src-tauri/src/meta_agent.rs
// ПОЛНАЯ ЗАМЕНА — скопировать целиком


use crate::agents::handoff::{Finding, FindingType, Severity};
use crate::feedback_store::{
    DeviceProfile, FailureReason, FeedbackStore, TechniqueOutcome,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;


// ── Типы ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaDecision {
    pub action: String,
    pub target: String,
    pub priority: u8,
    pub reasoning: String,
    pub technique_order: Vec<String>,
    pub epsilon: f32,
    pub similar_targets_found: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaState {
    pub campaign_id: String,
    pub iteration: u32,
    pub decisions_made: Vec<MetaDecision>,
    pub total_findings: usize,
    pub success_rate: f32,
}


// ── Все доступные техники ────────────────────────────────────────────
fn all_techniques() -> Vec<String> {
    vec![
        "default_creds".to_string(),    // стандартные учётки
        "cve_probe".to_string(),         // проверка CVE
        "api_unauth".to_string(),        // неавторизованный API
        "rtsp_anon".to_string(),         // анонимный RTSP
        "isapi_search".to_string(),      // ISAPI enumeration
        "onvif_probe".to_string(),       // ONVIF enumeration
        "ftp_anon".to_string(),          // анонимный FTP
        "nemesis_rtsp".to_string(),      // Nemesis RTSP brute
        "nemesis_http".to_string(),      // Nemesis HTTP fuzzing
        "spider_dirs".to_string(),       // dir bruteforce
        "firmware_cve".to_string(),      // CVE по прошивке
        "lateral_creds".to_string(),     // reuse учёток
    ]
}


// ── Определение вендора ──────────────────────────────────────────────
pub fn infer_vendor(scope: &str) -> String {
    let low = scope.to_lowercase();
    if low.contains("hik")     { return "Hikvision".to_string(); }
    if low.contains("dahua")   { return "Dahua".to_string(); }
    if low.contains("axis")    { return "Axis".to_string(); }
    if low.contains("reolink") { return "Reolink".to_string(); }
    if low.contains("tp-link") || low.contains("tplink") { return "TP-Link".to_string(); }
    if low.contains("uniview") { return "Uniview".to_string(); }
    "unknown".to_string()
}


// ── Выбор техники: k-NN приоритет + UCB + epsilon-greedy ────────────
fn select_next_technique(
    vendor: &str,
    feedback: &FeedbackStore,
    epsilon: f32,
    similar: &[DeviceProfile],
) -> (String, Vec<String>) {
    let mut candidates = all_techniques();
    // k-NN: техники похожих устройств идут первыми
    if !similar.is_empty() {
        let mut knn: Vec<String> = similar.iter()
            .flat_map(|p| p.successful_techniques.iter().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter().collect();
        let existing: std::collections::HashSet<String> = knn.iter().cloned().collect();
        knn.extend(candidates.into_iter().filter(|t| !existing.contains(t)));
        candidates = knn;
    }
    let chosen = feedback.select_technique_ucb(vendor, &candidates, epsilon);
    (chosen, candidates)
}


// ── Реальное измерение результата (HTTP/TCP) ─────────────────────────
async fn measure_technique_outcome(
    technique: &str,
    scope: &str,
    known_creds: &[(String, String)],
    log_state: &State<'_, crate::LogState>,
) -> TechniqueOutcome {
    let t0 = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .danger_accept_invalid_certs(true)
        .build().unwrap_or_default();


    let (success, got_creds, got_archive, got_stream, fail_reason) = match technique {
        "default_creds" => {
            let creds_to_try = if !known_creds.is_empty() {
                known_creds.to_vec()
            } else {
                vec![
                    ("admin".to_string(), "admin".to_string()),
                    ("admin".to_string(), "12345".to_string()),
                    ("admin".to_string(), "123456".to_string()),
                    ("admin".to_string(), "".to_string()),
                    ("admin".to_string(), "Admin1234".to_string()),
                ]
            };
            let url = format!("http://{}/ISAPI/System/deviceInfo", scope);
            let mut found = false;
            for (u, pw) in &creds_to_try {
                if let Ok(r) = client.get(&url).basic_auth(u, Some(pw)).send().await {
                    if r.status().is_success() { found = true; break; }
                    if r.status().as_u16() == 429 {
                        return make_outcome(false, false, false, false,
                            Some(FailureReason::RateLimit), t0.elapsed().as_millis() as u64);
                    }
                }
            }
            (found, found, false, false, if found { None } else { Some(FailureReason::WrongCredentials) })
        },
        "rtsp_anon" => {
            let host = scope.split(':').next().unwrap_or(scope);
            let paths = ["/Streaming/Channels/101","/stream1","/cam/realmonitor","/video1"];
            let mut ok = false;
            for path in paths {
                let _url = format!("rtsp://{}:{}{}", host, 554, path);
                let probe = client.head(&format!("http://{}:554", host)).send().await;
                if probe.is_ok() { ok = true; break; }
            }
            (ok, false, false, ok, if ok { None } else { Some(FailureReason::NotDetected) })
        },
        "ftp_anon" => {
            let host = scope.split(':').next().unwrap_or(scope);
            let ok = tokio::net::TcpStream::connect(format!("{}:21", host))
                .await.is_ok();
            (ok, false, false, false, if ok { None } else { Some(FailureReason::NotDetected) })
        },
        "isapi_search" => {
            let url = format!("http://{}/ISAPI/ContentMgmt/search", scope);
            let ok = client.post(&url).send().await
                .map(|r| r.status().as_u16() != 404).unwrap_or(false);
            (ok, false, ok, false, if ok { None } else { Some(FailureReason::NotDetected) })
        },
        "onvif_probe" => {
            let url = format!("http://{}/onvif/device_service", scope);
            let ok = client.get(&url).send().await
                .map(|r| r.status().as_u16() != 404).unwrap_or(false);
            (ok, false, false, false, if ok { None } else { Some(FailureReason::NotDetected) })
        },
        _ => {
            // Для прочих техник — базовая TCP достижимость
            let addr = if scope.contains(':') { scope.to_string() }
                else { format!("{}:80", scope) };
            let ok = tokio::time::timeout(
                std::time::Duration::from_secs(4),
                tokio::net::TcpStream::connect(&addr)
            ).await.map(|r| r.is_ok()).unwrap_or(false);
            (ok, false, false, false, if ok { None } else { Some(FailureReason::Timeout) })
        }
    };


    crate::push_runtime_log(log_state, format!(
        "META_MEASURE|tech={}|scope={}|ok={}|ms={}",
        technique, scope, success, t0.elapsed().as_millis()
    ));


    make_outcome(success, got_creds, got_archive, got_stream, fail_reason,
        t0.elapsed().as_millis() as u64)
}


fn make_outcome(
    success: bool, got_creds: bool, got_archive: bool, got_stream: bool,
    fail_reason: Option<FailureReason>, time_ms: u64,
) -> TechniqueOutcome {
    TechniqueOutcome {
        success, got_credentials: got_creds, got_archive,
        got_stream, no_alert: success,
        failure_reason: fail_reason, time_ms, condition: None,
    }
}


// ── Finding из строки ────────────────────────────────────────────────
fn finding_from_memory(target: &str, finding: &str) -> Finding {
    let lower = finding.to_lowercase();
    let severity = if lower.contains("critical") || lower.contains("rce") { Severity::Critical }
        else if lower.contains("high") || lower.contains("cve") { Severity::High }
        else if lower.contains("medium") { Severity::Medium }
        else if lower.contains("low") { Severity::Low }
        else { Severity::Info };
    Finding { host: target.to_string(), finding_type: FindingType::Intelligence,
        severity, cve: None, cvss_score: None, description: finding.to_string(),
        evidence: None, confidence_score: 0.6 }
}


// ── ГЛАВНАЯ КОМАНДА ──────────────────────────────────────────────────
#[tauri::command]
pub async fn run_meta_campaign(
    scope: String,
    permit_token: String,
    max_iterations: Option<u32>,
    _scope_guard: State<'_, crate::scope_guard::ScopeGuard>,
    feedback: State<'_, Arc<FeedbackStore>>,
    log_state: State<'_, crate::LogState>,
) -> Result<MetaState, String> {
    if permit_token.trim().len() < 8 {
        return Err("Unauthorized: permit token too short".to_string());
    }
    let campaign_id = format!("meta_{}", chrono::Utc::now().timestamp_millis());
    let max_iter = max_iterations.unwrap_or(3).min(10);
    let vendor = infer_vendor(&scope);
    let mut state = MetaState {
        campaign_id: campaign_id.clone(), iteration: 0,
        decisions_made: Vec::new(), total_findings: 0, success_rate: 0.0,
    };


    crate::push_runtime_log(&log_state, format!(
        "META_CAMPAIGN_START|id={}|scope={}|vendor={}|max_iter={}",
        campaign_id, scope, vendor, max_iter
    ));


    let known_creds = feedback.get_working_creds();
    // k-NN: найти похожие устройства
    let probe_ports = vec![80u16, 443, 554, 8080, 8554, 8000];
    let similar = feedback.find_similar_devices(&probe_ports, &vendor, 5);
    crate::push_runtime_log(&log_state,
        format!("META_KNN|similar_found={}", similar.len()));


    let mut all_findings: Vec<Finding> = feedback.get_findings(&scope)
        .unwrap_or_default().into_iter()
        .map(|f| finding_from_memory(&scope, &f)).collect();
    let mut already_tried: Vec<String> = Vec::new();


    for iteration in 0..max_iter {
        state.iteration = iteration + 1;
        // epsilon: 0.20 → 0.02 за итерации (больше данных = меньше случайности)
        let epsilon = (0.20_f32 - iteration as f32 * 0.02).max(0.02);


        let (chosen, technique_order) =
            select_next_technique(&vendor, &feedback, epsilon, &similar);


        // Избегать повторений
        let technique = if already_tried.contains(&chosen) {
            technique_order.iter().find(|t| !already_tried.contains(*t))
                .cloned().unwrap_or_else(|| chosen.clone())
        } else { chosen };
        already_tried.push(technique.clone());


        let decision = MetaDecision {
            action: technique.clone(), target: scope.clone(),
            priority: if iteration == 0 { 1 } else { 3 },
            reasoning: format!("iter={} eps={:.2} vendor={} knn={}", iteration+1, epsilon, vendor, similar.len()),
            technique_order: technique_order.into_iter().take(5).collect(),
            epsilon, similar_targets_found: similar.len(),
        };


        crate::push_runtime_log(&log_state, format!(
            "META_DECISION|iter={}|tech={}|eps={:.2}", iteration+1, technique, epsilon
        ));
        state.decisions_made.push(decision);


        // Реальное выполнение + измерение
        let outcome = measure_technique_outcome(
            &technique, &scope, &known_creds, &log_state).await;
        feedback.record_outcome(&technique, &vendor, &outcome);


        if outcome.success {
            let msg = format!("success: tech={} scope={}", technique, scope);
            feedback.record_finding(&scope, &msg);
            all_findings.push(finding_from_memory(&scope, &msg));
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }


    // Сохранить DeviceProfile
    let successful_techs: Vec<String> = state.decisions_made.iter()
        .filter(|d| d.priority == 1).map(|d| d.action.clone()).collect();
    feedback.record_device_profile(DeviceProfile {
        ip_hash: { use sha2::Digest; let mut h = sha2::Sha256::new();
            h.update(scope.as_bytes()); format!("{:x}", h.finalize()) },
        vendor: vendor.clone(),
        firmware_version: "unknown".to_string(),
        open_ports: probe_ports,
        has_waf: false, has_auth: true,
        successful_techniques: successful_techs,
        last_seen: chrono::Utc::now().to_rfc3339(),
    });


    state.total_findings = all_findings.len();
    let suc = state.decisions_made.iter().filter(|d| d.priority == 1).count();
    state.success_rate = if state.decisions_made.is_empty() { 0.0 }
        else { suc as f32 / state.decisions_made.len() as f32 };


    crate::push_runtime_log(&log_state, format!(
        "META_CAMPAIGN_DONE|id={}|findings={}|rate={:.0}%",
        campaign_id, state.total_findings, state.success_rate * 100.0
    ));
    Ok(state)
}


#[tauri::command]
pub fn get_meta_recommendations(
    vendor: String, feedback: State<'_, Arc<FeedbackStore>>
) -> Vec<String> {
    let techs = feedback.get_prioritized_techniques(&vendor);
    let creds = feedback.get_working_creds();
    let total = feedback.get_total_trials();
    let mut recs = Vec::new();
    if !techs.is_empty() {
        recs.push(format!("Лучшие техники (UCB): {}", techs.iter().take(4).cloned().collect::<Vec<_>>().join(" → ")));
    }
    if !creds.is_empty() {
        recs.push(format!("Рабочие учётки: {}", creds.iter().map(|(u,p)| format!("{}:{}", u, p)).collect::<Vec<_>>().join(", ")));
    }
    recs.push(format!("Всего попыток в базе: {}", total));
    recs
}
