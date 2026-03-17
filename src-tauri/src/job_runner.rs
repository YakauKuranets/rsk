use crate::api_fuzzer;
use crate::auditor;
use crate::broker::send_intel;
use crate::breach_analyzer;
use crate::feedback_store::FeedbackStore;
use crate::lateral_scanner;
use crate::rce_verifier;
use crate::session_checker;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobModule {
    FtpScanner,
    PortScanner,
    ApiFuzzer,
    SessionChecker,
    RceVerifier,
    BreachAnalyzer,
    LateralScanner,
    // В будущем добавим новые модули сюда
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub target: String,
    pub module: JobModule,
    pub payload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: String,
    pub status: String, // "success", "failed", "timeout"
    pub finding: Option<String>,
}

pub struct JobManager {
    sender: mpsc::Sender<Job>,
}

impl JobManager {
    pub fn new() -> (Self, mpsc::Receiver<Job>) {
        // Канал на 100 одновременных задач
        let (tx, rx) = mpsc::channel(100);
        (Self { sender: tx }, rx)
    }

    pub async fn submit_job(&self, job: Job) -> Result<(), String> {
        self.sender.send(job).await.map_err(|e| e.to_string())
    }
}

// Фоновый воркер, который будет разгребать очередь
pub async fn run_worker_loop(
    mut receiver: mpsc::Receiver<Job>,
    feedback_store: Arc<FeedbackStore>,
    app_handle: AppHandle,
) {
    println!("[JobRunner] Worker loop started...");
    while let Some(job) = receiver.recv().await {
        println!(
            "[JobRunner] Executing job: {} for target: {}",
            job.id, job.target
        );

        if let Some(known_vulns) = feedback_store.get_findings(&job.target) {
            println!(
                "[JobRunner] 💡 Использую память! Известные уязвимости для {}: {:?}",
                job.target, known_vulns
            );
            // Здесь в будущем можно передавать известные пароли внутрь сканеров для ускорения
        }

        match job.module {
            JobModule::FtpScanner => {
                let vendor = job.payload.clone().unwrap_or_else(|| "hikvision".to_string());
                match auditor::adaptive_credential_audit(job.target.clone(), vendor, None).await {
                    Ok(Some((login, password))) => {
                        let credentials = format!("{}:{}", login, password);
                        let msg = format!(
                            "🟢 НАЙДЕНА УЯЗВИМОСТЬ на {}: {}",
                            job.target, credentials
                        );
                        println!("[JobRunner] {}", msg);
                        let _ = app_handle.emit("hyperion-audit-event", msg);
                        feedback_store.record_finding(
                            &job.target,
                            &format!("FTP Creds: {}", credentials),
                        );

                        let payload = format!("LEAK: {} -> {}", job.target, credentials);
                        if let Err(err) = send_intel(payload).await {
                            println!("[JobRunner] ⚠️ Ошибка отправки в Redpanda: {}", err);
                        }
                    }
                    Ok(None) => println!("[JobRunner] ⚪ Цель безопасна: {}", job.target),
                    Err(e) => println!("[JobRunner] 🔴 Ошибка сканирования {}: {}", job.target, e),
                }
            }
            JobModule::ApiFuzzer => {
                match api_fuzzer::run_fuzzer(&job.target).await {
                    Ok(Some(findings)) => {
                        let msg = format!(
                            "🟢 НАЙДЕНЫ СКРЫТЫЕ API на {}: {}",
                            job.target, findings
                        );
                        println!("[JobRunner] {}", msg);
                        let _ = app_handle.emit("hyperion-audit-event", msg);
                        feedback_store.record_finding(
                            &job.target,
                            &format!("API Discovery: {}", findings),
                        );
                        let payload = format!("API_DISCOVERY: {} -> {}", job.target, findings);
                        if let Err(err) = send_intel(payload).await {
                            println!("[JobRunner] ⚠️ Ошибка отправки в Redpanda: {}", err);
                        }
                    }
                    Ok(None) => println!("[JobRunner] ⚪ API не обнаружены: {}", job.target),
                    Err(e) => println!("[JobRunner] 🔴 Ошибка фаззера на {}: {}", job.target, e),
                }
            }
            JobModule::RceVerifier => {
                match rce_verifier::verify_rce(&job.target).await {
                    Ok(Some(findings)) => {
                        let msg = format!(
                            "🔴 КРИТИЧЕСКАЯ УЯЗВИМОСТЬ RCE на {}: {}",
                            job.target, findings
                        );
                        println!("[JobRunner] {}", msg);
                        let _ = app_handle.emit("hyperion-audit-event", msg);
                        feedback_store
                            .record_finding(&job.target, &format!("RCE: {}", findings));
                        let payload = format!("RCE_VULN: {} -> {}", job.target, findings);
                        if let Err(err) = send_intel(payload).await {
                            println!("[JobRunner] ⚠️ Ошибка отправки в Redpanda: {}", err);
                        }
                    }
                    Ok(None) => println!("[JobRunner] ⚪ RCE не обнаружено: {}", job.target),
                    Err(e) => println!("[JobRunner] ⚠️ Ошибка проверки RCE на {}: {}", job.target, e),
                }
            }
            JobModule::BreachAnalyzer => {
                match breach_analyzer::check_breaches(&job.target).await {
                    Ok(Some(findings)) => {
                        let msg = format!(
                            "⚠️ НАЙДЕНЫ СЛЕДЫ УТЕЧЕК для {}: {}",
                            job.target, findings
                        );
                        println!("[JobRunner] {}", msg);
                        let _ = app_handle.emit("hyperion-audit-event", msg);
                        feedback_store.record_finding(
                            &job.target,
                            &format!("Breach Data: {}", findings),
                        );
                        let payload = format!("BREACH_DATA: {} -> {}", job.target, findings);
                        if let Err(err) = send_intel(payload).await {
                            println!("[JobRunner] ⚠️ Ошибка отправки в Redpanda: {}", err);
                        }
                    }
                    Ok(None) => println!("[JobRunner] ⚪ В базах утечек не числится: {}", job.target),
                    Err(e) => println!(
                        "[JobRunner] 🔴 Ошибка анализа утечек для {}: {}",
                        job.target, e
                    ),
                }
            }
            JobModule::LateralScanner => {
                // 1. Получаем известные креды из памяти для этого таргета!
                let known_vulns = feedback_store.get_findings(&job.target).unwrap_or_default();

                if known_vulns.is_empty() {
                    println!(
                        "[JobRunner] ⏭️ Пропуск Lateral Movement: нет известных кредов для {}",
                        job.target
                    );
                    continue;
                }

                match lateral_scanner::check_neighbors(&job.target, known_vulns).await {
                    Ok(Some(findings)) => {
                        let msg = format!(
                            "🕸️ УСПЕШНОЕ БОКОВОЕ ПЕРЕМЕЩЕНИЕ от {}: {}",
                            job.target, findings
                        );
                        println!("[JobRunner] {}", msg);
                        let _ = app_handle.emit("hyperion-audit-event", msg);
                        feedback_store
                            .record_finding(&job.target, &format!("Lateral: {}", findings));
                        let payload = format!("LATERAL_MOVEMENT: {} -> {}", job.target, findings);
                        if let Err(err) = send_intel(payload).await {
                            println!("[JobRunner] ⚠️ Ошибка отправки в Redpanda: {}", err);
                        }
                    }
                    Ok(None) => println!("[JobRunner] ⚪ Соседи {} безопасны.", job.target),
                    Err(e) => println!("[JobRunner] 🔴 Ошибка Lateral Scanner на {}: {}", job.target, e),
                }
            }
            JobModule::SessionChecker => {
                match session_checker::check_session(&job.target).await {
                    Ok(Some(vulns)) => {
                        let msg = format!(
                            "🟢 НАЙДЕНА УЯЗВИМОСТЬ СЕССИИ на {}: {}",
                            job.target, vulns
                        );
                        println!("[JobRunner] {}", msg);
                        let _ = app_handle.emit("hyperion-audit-event", msg);
                        feedback_store.record_finding(
                            &job.target,
                            &format!("Session Vuln: {}", vulns),
                        );
                        let payload = format!("SESSION_VULN: {} -> {}", job.target, vulns);
                        if let Err(err) = send_intel(payload).await {
                            println!("[JobRunner] ⚠️ Ошибка отправки в Redpanda: {}", err);
                        }
                    }
                    Ok(None) => println!("[JobRunner] ⚪ Сессии безопасны: {}", job.target),
                    Err(e) => println!("[JobRunner] 🔴 Ошибка HTTP на {}: {}", job.target, e),
                }
            }
            // Заглушки для будущих модулей
            _ => {
                println!("[JobRunner] Module not implemented yet.");
            }
        }

        println!("[JobRunner] Job {} finished.", job.id);
    }
}

#[tauri::command]
pub async fn start_audit_job(
    target: String,
    job_manager: State<'_, Arc<JobManager>>,
) -> Result<String, String> {
    let job = Job {
        id: Utc::now().timestamp_millis().to_string(),
        target: target.clone(),
        module: JobModule::FtpScanner,
        payload: None,
    };

    job_manager.submit_job(job).await?;
    Ok(format!(
        "Задача для {} успешно добавлена в очередь JobRunner",
        target
    ))
}


#[tauri::command]
pub async fn start_session_job(
    target: String,
    job_manager: State<'_, Arc<JobManager>>,
) -> Result<String, String> {
    let job = Job {
        id: Utc::now().timestamp_millis().to_string(),
        target: target.clone(),
        module: JobModule::SessionChecker,
        payload: None,
    };

    job_manager.submit_job(job).await?;
    Ok(format!(
        "Задача проверки сессий для {} добавлена в очередь",
        target
    ))
}


#[tauri::command]
pub async fn start_fuzzer_job(
    target: String,
    job_manager: State<'_, Arc<JobManager>>,
) -> Result<String, String> {
    let job = Job {
        id: Utc::now().timestamp_millis().to_string(),
        target: target.clone(),
        module: JobModule::ApiFuzzer,
        payload: None,
    };
    job_manager.submit_job(job).await?;
    Ok(format!("Задача API Fuzzer для {} добавлена в очередь", target))
}


#[tauri::command]
pub async fn start_rce_job(
    target: String,
    job_manager: State<'_, Arc<JobManager>>,
) -> Result<String, String> {
    let job = Job {
        id: Utc::now().timestamp_millis().to_string(),
        target: target.clone(),
        module: JobModule::RceVerifier,
        payload: None,
    };
    job_manager.submit_job(job).await?;
    Ok(format!("Задача RCE Verifier для {} добавлена в очередь", target))
}


#[tauri::command]
pub async fn start_breach_job(
    target: String,
    job_manager: State<'_, Arc<JobManager>>,
) -> Result<String, String> {
    let job = Job {
        id: Utc::now().timestamp_millis().to_string(),
        target: target.clone(),
        module: JobModule::BreachAnalyzer,
        payload: None,
    };
    job_manager.submit_job(job).await?;
    Ok(format!(
        "Задача проверки утечек (Breach Data) для {} добавлена в очередь",
        target
    ))
}


#[tauri::command]
pub async fn start_lateral_job(
    target: String,
    job_manager: State<'_, Arc<JobManager>>,
) -> Result<String, String> {
    let job = Job {
        id: Utc::now().timestamp_millis().to_string(),
        target: target.clone(),
        module: JobModule::LateralScanner,
        payload: None,
    };
    job_manager.submit_job(job).await?;
    Ok(format!(
        "Задача Lateral Scanner для {} добавлена в очередь",
        target
    ))
}
