use crate::auditor;
use crate::broker::send_intel;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobModule {
    FtpScanner,
    PortScanner,
    ApiFuzzer,
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
pub async fn run_worker_loop(mut receiver: mpsc::Receiver<Job>) {
    println!("[JobRunner] Worker loop started...");
    while let Some(job) = receiver.recv().await {
        println!(
            "[JobRunner] Executing job: {} for target: {}",
            job.id, job.target
        );

        match job.module {
            JobModule::FtpScanner => {
                let vendor = job.payload.clone().unwrap_or_else(|| "hikvision".to_string());
                match auditor::adaptive_credential_audit(job.target.clone(), vendor, None).await {
                    Ok(Some((login, password))) => {
                        let credentials = format!("{}:{}", login, password);
                        println!("[JobRunner] 🟢 УЯЗВИМОСТЬ НАЙДЕНА: {}", credentials);

                        let payload = format!("LEAK: {} -> {}", job.target, credentials);
                        if let Err(err) = send_intel(payload).await {
                            println!("[JobRunner] ⚠️ Ошибка отправки в Redpanda: {}", err);
                        }
                    }
                    Ok(None) => println!("[JobRunner] ⚪ Цель безопасна: {}", job.target),
                    Err(e) => println!("[JobRunner] 🔴 Ошибка сканирования {}: {}", job.target, e),
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
