use serde::{Deserialize, Serialize};
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

        // Здесь в будущем будет вызов конкретных сканеров через match job.module
        // Пока просто симулируем работу
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        println!("[JobRunner] Job {} finished.", job.id);
    }
}
