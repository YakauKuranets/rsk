use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rand::Rng;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::time::{sleep, Duration};

// Глобальное состояние для управления ИИ-воркером
pub struct AiState {
    pub is_running: Arc<AtomicBool>,
}

#[derive(Clone, Serialize)]
pub struct AiEvent {
    pub timestamp_ms: u64,
    pub class: String,
    pub confidence: f32,
}

#[tauri::command]
pub async fn start_archive_analysis(
    app: AppHandle,
    state: State<'_, AiState>,
    playback_uri: String,
    duration_ms: u64,
) -> Result<(), String> {
    // Сначала глушим предыдущий процесс (если он был запущен)
    state.is_running.store(false, Ordering::SeqCst);
    sleep(Duration::from_millis(150)).await; // Даем время старому потоку умереть

    // Включаем зеленый свет для нового процесса
    state.is_running.store(true, Ordering::SeqCst);
    let is_running_flag = state.is_running.clone();

    println!("[AI MODULE] Запуск анализа архива: {}", playback_uri);

    tokio::spawn(async move {
        let mut current_ms = 0;
        let step = 5000;

        while current_ms < duration_ms {
            // KILL SWITCH: Проверяем, не нажал ли пользователь кнопку СТОП
            if !is_running_flag.load(Ordering::SeqCst) {
                println!("[AI MODULE] Анализ принудительно прерван пользователем.");
                break;
            }

            sleep(Duration::from_millis(100)).await;

            if rand::random::<f32>() > 0.85 {
                let event = AiEvent {
                    timestamp_ms: current_ms,
                    class: "person".to_string(),
                    confidence: rand::thread_rng().gen_range(0.80..0.99),
                };
                let _ = app.emit("ai-archive-event", &event);
            }
            current_ms += step;
        }

        is_running_flag.store(false, Ordering::SeqCst);
        println!("[AI MODULE] Процесс сканирования завершен.");
        let _ = app.emit("ai-archive-done", ());
    });

    Ok(())
}

#[tauri::command]
pub fn stop_archive_analysis(state: State<'_, AiState>) {
    println!("[AI MODULE] Получена команда СТОП от интерфейса.");
    state.is_running.store(false, Ordering::SeqCst);
}
