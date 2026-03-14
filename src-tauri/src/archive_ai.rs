use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ndarray::Array;
use ort::session::{builder::GraphOptimizationLevel, Session};
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
        // Загружаем сессию ONNX (ИИ)
        let model_path = "../Vault/Models/yolov8s.onnx";
        let session_result = (|| -> ort::Result<Session> {
            Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(4)?
                .commit_from_file(model_path)
        })();

        let mut session = match session_result {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[AI MODULE] 🔴 Ошибка загрузки модели: {}", e);
                return;
            }
        };

        println!("[AI MODULE] 🟢 YOLOv8 готова. Запускаем перехват кадров FFmpeg...");

        // Запускаем FFmpeg в фоне для вытягивания сырых кадров
        let mut child = Command::new("ffmpeg")
            .args(&[
                "-i",
                &playback_uri,
                "-r",
                "2", // Анализируем 2 кадра в секунду
                "-f",
                "image2pipe",
                "-pix_fmt",
                "rgb24",
                "-s",
                "640x640", // YOLOv8 ожидает именно 640x640
                "-vcodec",
                "rawvideo",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // Скрываем спам от FFmpeg
            .spawn()
            .expect("Не удалось запустить FFmpeg");

        let mut stdout = child.stdout.take().expect("Нет доступа к stdout FFmpeg");
        let frame_size = 640 * 640 * 3; // Размер сырого RGB кадра
        let mut buffer = vec![0u8; frame_size];

        let mut current_ms = 0;
        let step_ms = 500; // 2 FPS = шаг в 500 миллисекунд

        // Читаем поток кадров, пока пользователь не нажмет СТОП
        while is_running_flag.load(Ordering::SeqCst) {
            match stdout.read_exact(&mut buffer) {
                Ok(_) => {
                    // Конвертируем RGB-массив в тензор [1, 3, 640, 640] и нормализуем (0.0 - 1.0)
                    let mut tensor = Array::zeros((1, 3, 640, 640));
                    for y in 0..640 {
                        for x in 0..640 {
                            let offset = (y * 640 + x) * 3;
                            tensor[[0, 0, y, x]] = buffer[offset] as f32 / 255.0; // R
                            tensor[[0, 1, y, x]] = buffer[offset + 1] as f32 / 255.0; // G
                            tensor[[0, 2, y, x]] = buffer[offset + 2] as f32 / 255.0; // B
                        }
                    }

                    // Скармливаем кадр нейросети (убрали .unwrap(), так как макрос теперь возвращает готовый Vec)
                    if let Ok(outputs) = session.run(ort::inputs!["images" => tensor.view()]) {
                        // В ort v2 try_extract_tensor возвращает кортеж (Shape, &[f32])
                        if let Ok((_shape, slice)) = outputs[0].try_extract_tensor::<f32>() {
                            let mut found_person = false;
                            let mut max_conf = 0.0f32;

                            // YOLOv8 возвращает плоский массив.
                            // Матрица имеет форму [1, 84, 8400].
                            // Нам нужна строка с индексом 4 (вероятность класса 'person').
                            // Чтобы найти её в плоском массиве &[f32], используем смещение: 4 * 8400
                            for i in 0..8400 {
                                let person_conf = slice[4 * 8400 + i];
                                if person_conf > 0.65 { // Порог уверенности 65%
                                    found_person = true;
                                    if person_conf > max_conf { max_conf = person_conf; }
                                }
                            }

                            // Если реально нашли человека - отправляем метку во фронтенд!
                            if found_person {
                                println!("[AI MODULE] 👤 Найден человек на {} мс (Уверенность: {:.2})", current_ms, max_conf);
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

        let _ = child.kill(); // Убиваем FFmpeg, если пользователь нажал СТОП
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
