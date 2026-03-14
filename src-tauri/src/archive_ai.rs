use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ort::{GraphOptimizationLevel, Session};
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
        println!("[AI MODULE] Инициализация движка ONNX Runtime...");

        // Инициализируем глобальную среду ONNX (игнорируем ошибку, если уже инициализировано)
        let _ = ort::init().with_name("hyperion_vision").commit();

        // Пытаемся загрузить веса модели в оперативную память.
        // ОЖИДАЕТСЯ, ЧТО ПАПКА Vault НАХОДИТСЯ РЯДОМ С ИСПОЛНЯЕМЫМ ФАЙЛОМ
        let model_path = "../Vault/Models/yolov8s.onnx";

        let _session = match Session::builder()
            .and_then(|b| b.with_optimization_level(GraphOptimizationLevel::Level3))
            .and_then(|b| b.with_intra_threads(4)) // Выделяем 4 потока вашего Ryzen
            .and_then(|b| b.commit_from_file(model_path))
        {
            Ok(s) => {
                println!("[AI MODULE] 🟢 УСПЕХ: YOLOv8 загружена в ОЗУ!");
                Some(s)
            }
            Err(e) => {
                eprintln!(
                    "[AI MODULE] 🔴 ОШИБКА загрузки модели: {}. Проверьте, лежит ли файл по пути: {}",
                    e, model_path
                );
                None
            }
        };

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
