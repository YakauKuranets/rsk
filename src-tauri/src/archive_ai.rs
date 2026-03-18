use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ort::session::{builder::GraphOptimizationLevel, Session};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::time::{sleep, Duration};

// Глобальное состояние для управления ИИ-воркером
pub struct AiState {
    pub is_running: Arc<AtomicBool>,
    pub model_path: Option<PathBuf>,
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
    login: String,
    pass: String,
) -> Result<(), String> {
    let model_path = state.model_path.clone().ok_or_else(|| {
        "ONNX модель не найдена. Скачайте archive_detector.onnx в ~/.nemesis_vault/models/."
            .to_string()
    })?;

    // Сначала глушим предыдущий процесс (если он был запущен)
    state.is_running.store(false, Ordering::SeqCst);
    sleep(Duration::from_millis(150)).await;

    // Включаем зеленый свет для нового процесса
    state.is_running.store(true, Ordering::SeqCst);
    let is_running_flag = state.is_running.clone();

    println!("[AI MODULE] Запуск анализа архива: {}", playback_uri);

    tokio::spawn(async move {
        let session_result = (|| -> ort::Result<Session> {
            Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(4)?
                .commit_from_file(&model_path)
        })();

        let mut session = match session_result {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "[AI MODULE] 🔴 Ошибка загрузки модели {}: {}",
                    model_path.display(),
                    e
                );
                is_running_flag.store(false, Ordering::SeqCst);
                let _ = app.emit("ai-archive-done", ());
                return;
            }
        };

        println!(
            "[AI MODULE] 🟢 ONNX модель загружена: {}",
            model_path.display()
        );
        println!("[AI MODULE] 🟢 YOLOv8 готова. Запускаем перехват кадров FFmpeg...");

        let auth_uri = playback_uri.replace("rtsp://", &format!("rtsp://{}:{}@", login, pass));

        let mut child = Command::new("ffmpeg")
            .args(&[
                "-rtsp_transport",
                "tcp",
                "-i",
                &auth_uri,
                "-r",
                "2",
                "-f",
                "image2pipe",
                "-pix_fmt",
                "rgb24",
                "-s",
                "640x640",
                "-vcodec",
                "rawvideo",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Не удалось запустить FFmpeg");

        let mut stdout = child.stdout.take().expect("Нет доступа к stdout FFmpeg");
        let frame_size = 640 * 640 * 3;
        let mut buffer = vec![0u8; frame_size];

        let mut current_ms = 0;
        let step_ms = 500;

        while is_running_flag.load(Ordering::SeqCst) {
            match stdout.read_exact(&mut buffer) {
                Ok(_) => {
                    let mut tensor_data = vec![0.0f32; 3 * 640 * 640];
                    for y in 0..640 {
                        for x in 0..640 {
                            let pixel_offset = (y * 640 + x) * 3;
                            let spatial_offset = y * 640 + x;

                            tensor_data[spatial_offset] = buffer[pixel_offset] as f32 / 255.0;
                            tensor_data[640 * 640 + spatial_offset] =
                                buffer[pixel_offset + 1] as f32 / 255.0;
                            tensor_data[2 * 640 * 640 + spatial_offset] =
                                buffer[pixel_offset + 2] as f32 / 255.0;
                        }
                    }

                    let input_tensor =
                        ort::value::Tensor::from_array(([1, 3, 640, 640], tensor_data)).unwrap();

                    if let Ok(outputs) = session.run(ort::inputs!["images" => input_tensor]) {
                        if let Ok((_shape, slice)) = outputs[0].try_extract_tensor::<f32>() {
                            let mut found_person = false;
                            let mut max_conf = 0.0f32;

                            for i in 0..8400 {
                                let person_conf = slice[4 * 8400 + i];
                                if person_conf > 0.65 {
                                    found_person = true;
                                    if person_conf > max_conf {
                                        max_conf = person_conf;
                                    }
                                }
                            }

                            if found_person {
                                println!(
                                    "[AI MODULE] 👤 Найден человек на {} мс (Уверенность: {:.2})",
                                    current_ms, max_conf
                                );
                                let event = AiEvent {
                                    timestamp_ms: current_ms,
                                    class: "person".to_string(),
                                    confidence: max_conf,
                                };
                                let _ = app.emit("ai-archive-event", &event);
                            }
                        }
                    }
                    current_ms += step_ms;
                    if current_ms > duration_ms {
                        break;
                    }
                }
                Err(_) => {
                    println!("[AI MODULE] Конец видеопотока или ошибка чтения.");
                    break;
                }
            }
        }

        let _ = child.kill();
        is_running_flag.store(false, Ordering::SeqCst);
        println!("[AI MODULE] Сканирование остановлено.");
        let _ = app.emit("ai-archive-done", ());
    });

    Ok(())
}

#[tauri::command]
pub fn stop_archive_analysis(state: State<'_, AiState>) {
    println!("[AI MODULE] Получена команда СТОП от интерфейса.");
    state.is_running.store(false, Ordering::SeqCst);
}
