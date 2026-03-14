use rand::Rng;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

#[derive(Clone, Serialize)]
pub struct AiEvent {
    pub timestamp_ms: u64,
    pub class: String,
    pub confidence: f32,
}

#[tauri::command]
pub async fn start_archive_analysis(
    app: AppHandle,
    playback_uri: String,
    duration_ms: u64,
) -> Result<(), String> {
    println!("[AI MODULE] Получен запрос на анализ архива: {}", playback_uri);

    tokio::spawn(async move {
        println!("[AI MODULE] Фоновый процесс YOLO запущен (Имитация)...");

        let mut current_ms = 0;
        let step = 5000;

        while current_ms < duration_ms {
            sleep(Duration::from_millis(100)).await;

            if rand::random::<f32>() > 0.85 {
                let confidence = rand::thread_rng().gen_range(0.80..0.99);
                let event = AiEvent {
                    timestamp_ms: current_ms,
                    class: "person".to_string(),
                    confidence,
                };

                if let Err(e) = app.emit("ai-archive-event", &event) {
                    eprintln!("[AI MODULE] Ошибка отправки события: {}", e);
                }
            }
            current_ms += step;
        }

        println!("[AI MODULE] Анализ отрезка завершен!");
        let _ = app.emit("ai-archive-done", ());
    });

    Ok(())
}
