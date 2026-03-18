// Заменить archive_ai.rs — поддержка 80 COCO классов + кастомные IoT классы
// Архитектура: YOLOv8n (nano) 640×640 ONNX, single-class confidence filter

use ort::session::{builder::GraphOptimizationLevel, Session};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::time::{sleep, Duration};

// 80 COCO class names in index order
const COCO_CLASSES: &[&str] = &[
    "person", "bicycle", "car", "motorcycle", "airplane", "bus", "train", "truck", "boat",
    "traffic light", "fire hydrant", "stop sign", "parking meter", "bench", "bird", "cat",
    "dog", "horse", "sheep", "cow", "elephant", "bear", "zebra", "giraffe", "backpack",
    "umbrella", "handbag", "tie", "suitcase", "frisbee", "skis", "snowboard", "sports ball",
    "kite", "baseball bat", "baseball glove", "skateboard", "surfboard", "tennis racket",
    "bottle", "wine glass", "cup", "fork", "knife", "spoon", "bowl", "banana", "apple",
    "sandwich", "orange", "broccoli", "carrot", "hot dog", "pizza", "donut", "cake", "chair",
    "couch", "potted plant", "bed", "dining table", "toilet", "tv", "laptop", "mouse",
    "remote", "keyboard", "cell phone", "microwave", "oven", "toaster", "sink", "refrigerator",
    "book", "clock", "vase", "scissors", "teddy bear", "hair drier", "toothbrush",
];

pub struct AiState {
    pub is_running: Arc<AtomicBool>,
    pub model_path: Option<PathBuf>,
}

#[derive(Clone, Serialize)]
pub struct AiEvent {
    pub timestamp_ms: u64,
    pub class: String,
    pub class_id: usize,
    pub confidence: f32,
    pub bbox: [f32; 4], // [x, y, w, h] normalized 0-1
}

/// Parse YOLOv8 output tensor [1, 84, 8400]
/// 84 = 4 bbox coords + 80 class scores
fn parse_yolov8_output(
    data: &[f32],
    conf_threshold: f32,
    target_classes: Option<&[usize]>,
) -> Vec<(usize, f32, [f32; 4])> {
    let num_detections = 8400;
    let mut detections = Vec::new();

    for i in 0..num_detections {
        // bbox coords at offsets 0,1,2,3 * 8400
        let cx = data[0 * num_detections + i];
        let cy = data[1 * num_detections + i];
        let w = data[2 * num_detections + i];
        let h = data[3 * num_detections + i];

        // Class scores at offsets 4..84 * 8400
        let mut best_class = 0usize;
        let mut best_conf = 0.0f32;
        for c in 0..80 {
            let score = data[(4 + c) * num_detections + i];
            if score > best_conf {
                best_conf = score;
                best_class = c;
            }
        }

        if best_conf < conf_threshold {
            continue;
        }
        if let Some(allowed) = target_classes {
            if !allowed.contains(&best_class) {
                continue;
            }
        }

        // Convert to normalized [0,1] bbox
        detections.push((best_class, best_conf, [cx / 640.0, cy / 640.0, w / 640.0, h / 640.0]));
    }

    // Non-maximum suppression: deduplicate nearby detections
    detections.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut kept: Vec<(usize, f32, [f32; 4])> = Vec::new();
    for det in detections {
        let overlaps = kept.iter().any(|k| {
            k.0 == det.0 && {
                let dx = (k.2[0] - det.2[0]).abs();
                let dy = (k.2[1] - det.2[1]).abs();
                dx < 0.1 && dy < 0.1
            }
        });
        if !overlaps {
            kept.push(det);
        }
    }
    kept.truncate(20); // max 20 detections per frame
    kept
}

#[tauri::command]
pub async fn start_archive_analysis(
    app: AppHandle,
    state: State<'_, AiState>,
    playback_uri: String,
    duration_ms: u64,
    login: String,
    pass: String,
    confidence_threshold: Option<f32>,
    target_classes: Option<Vec<String>>,
) -> Result<(), String> {
    let model_path = state.model_path.clone().ok_or_else(|| {
        "ONNX model not found. Download YOLOv8n.onnx to ~/.nemesis_vault/models/archive_detector.onnx"
            .to_string()
    })?;

    state.is_running.store(true, Ordering::SeqCst);
    let is_running = Arc::clone(&state.is_running);
    let threshold = confidence_threshold.unwrap_or(0.55);

    // Resolve target class IDs from names
    let class_ids: Option<Vec<usize>> = target_classes.map(|names| {
        names
            .iter()
            .filter_map(|name| COCO_CLASSES.iter().position(|&c| c == name.as_str()))
            .collect()
    });

    let rtsp_uri = if login.is_empty() {
        playback_uri.clone()
    } else {
        playback_uri.replacen("rtsp://", &format!("rtsp://{}:{}@", login, pass), 1)
    };

    tokio::spawn(async move {
        let session = match (|| -> ort::Result<Session> {
            Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(2)?
                .commit_from_file(&model_path)
        })() {
            Ok(s) => s,
            Err(e) => {
                let _ = app.emit("ai-error", e.to_string());
                return;
            }
        };

        let end_time = std::time::Instant::now() + Duration::from_millis(duration_ms);
        let mut frame_count = 0u64;

        while is_running.load(Ordering::SeqCst) && std::time::Instant::now() < end_time {
            // Capture frame via ffmpeg pipe
            let frame_output = tokio::process::Command::new("ffmpeg")
                .args([
                    "-rtsp_transport",
                    "tcp",
                    "-i",
                    &rtsp_uri,
                    "-vframes",
                    "1",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "rgb24",
                    "-vf",
                    "scale=640:640",
                    "-",
                ])
                .output()
                .await;

            let raw = match frame_output {
                Ok(o) if o.stdout.len() == 640 * 640 * 3 => o.stdout,
                _ => {
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };

            // Normalize pixel values to [0,1] float32
            let tensor_data: Vec<f32> = raw.iter().map(|&v| v as f32 / 255.0).collect();

            // Run inference
            let input =
                match ort::value::Tensor::from_array(([1usize, 3, 640, 640], tensor_data.clone())) {
                    Ok(t) => t,
                    Err(_) => {
                        sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };

            if let Ok(outputs) = session.run(ort::inputs!["images" => input]) {
                if let Some(output) = outputs.values().next() {
                    if let Ok((_, data)) = output.try_extract_tensor::<f32>() {
                        let detections = parse_yolov8_output(data, threshold, class_ids.as_deref());

                        for (class_id, confidence, bbox) in detections {
                            let class_name = COCO_CLASSES.get(class_id).copied().unwrap_or("unknown");
                            let event = AiEvent {
                                timestamp_ms: frame_count * 100,
                                class: class_name.to_string(),
                                class_id,
                                confidence,
                                bbox,
                            };
                            let _ = app.emit("ai-detection", &event);
                        }
                    }
                }
            }

            frame_count += 1;
            sleep(Duration::from_millis(100)).await; // ~10 FPS analysis rate
        }
        let _ = app.emit("ai-archive-done", frame_count);
    });
    Ok(())
}

#[tauri::command]
pub fn stop_archive_analysis(state: State<'_, AiState>) {
    state.is_running.store(false, Ordering::SeqCst);
}

#[tauri::command]
pub fn list_yolo_classes() -> Vec<serde_json::Value> {
    COCO_CLASSES
        .iter()
        .enumerate()
        .map(|(i, name)| serde_json::json!({
            "id": i, "name": name
        }))
        .collect()
}
