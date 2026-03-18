// src-tauri/src/anomaly_detector.rs
// Statistical anomaly detection for network traffic
// Uses rolling statistics (mean + std dev) — no ML model file needed
// Upgrade path: replace with ONNX LSTM when model is trained
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use tauri::State;

const WINDOW_SIZE: usize = 60; // observations for rolling baseline
const ANOMALY_THRESHOLD: f32 = 3.0; // standard deviations

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficSample {
    pub source_ip: String,
    pub packet_count: u64,
    pub byte_count: u64,
    pub unique_destinations: u32,
    pub port_scan_score: f32,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnomalyAlert {
    pub source_ip: String,
    pub anomaly_type: String,
    pub severity: String,
    pub score: f32,
    pub baseline_mean: f32,
    pub observed_value: f32,
    pub description: String,
    pub detected_at: String,
}

pub struct AnomalyState {
    pub baselines: Mutex<HashMap<String, IpBaseline>>,
}

#[derive(Clone)]
pub struct IpBaseline {
    pub packet_hist: VecDeque<f32>,
    pub byte_hist: VecDeque<f32>,
    pub dest_hist: VecDeque<f32>,
}

impl IpBaseline {
    fn new() -> Self {
        Self {
            packet_hist: VecDeque::with_capacity(WINDOW_SIZE + 1),
            byte_hist: VecDeque::with_capacity(WINDOW_SIZE + 1),
            dest_hist: VecDeque::with_capacity(WINDOW_SIZE + 1),
        }
    }

    fn push(&mut self, packets: f32, bytes: f32, dests: f32) {
        for (hist, val) in [
            (&mut self.packet_hist, packets),
            (&mut self.byte_hist, bytes),
            (&mut self.dest_hist, dests),
        ] {
            if hist.len() >= WINDOW_SIZE {
                hist.pop_front();
            }
            hist.push_back(val);
        }
    }

    fn zscore(&self, hist: &VecDeque<f32>, val: f32) -> Option<(f32, f32, f32)> {
        if hist.len() < 10 {
            return None;
        } // need at least 10 observations
        let n = hist.len() as f32;
        let mean = hist.iter().sum::<f32>() / n;
        let variance = hist.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n;
        let std_dev = variance.sqrt();
        if std_dev < 1.0 {
            return None;
        } // too stable to detect
        Some(((val - mean) / std_dev, mean, std_dev))
    }
}

impl AnomalyState {
    pub fn new() -> Self {
        Self {
            baselines: Mutex::new(HashMap::new()),
        }
    }
}

#[tauri::command]
pub fn analyze_traffic_sample(
    sample: TrafficSample,
    anomaly_state: State<'_, AnomalyState>,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<AnomalyAlert>, String> {
    let mut baselines = anomaly_state
        .baselines
        .lock()
        .map_err(|_| "lock poisoned")?;
    let baseline = baselines
        .entry(sample.source_ip.clone())
        .or_insert_with(IpBaseline::new);

    let mut alerts = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    // Check packet count anomaly
    if let Some((z, mean, _)) = baseline.zscore(&baseline.packet_hist.clone(), sample.packet_count as f32) {
        if z > ANOMALY_THRESHOLD {
            let atype = if z > 6.0 {
                "flood_attack"
            } else {
                "traffic_spike"
            };
            alerts.push(AnomalyAlert {
                source_ip: sample.source_ip.clone(),
                anomaly_type: atype.to_string(),
                severity: if z > 6.0 { "CRITICAL" } else { "HIGH" }.to_string(),
                score: z,
                baseline_mean: mean,
                observed_value: sample.packet_count as f32,
                description: format!(
                    "{:.0}x normal traffic from {}",
                    z / ANOMALY_THRESHOLD,
                    sample.source_ip
                ),
                detected_at: now.clone(),
            });
        }
    }

    // Check unique destinations (port scan / lateral movement)
    if let Some((z, mean, _)) = baseline.zscore(&baseline.dest_hist.clone(), sample.unique_destinations as f32) {
        if z > ANOMALY_THRESHOLD && sample.unique_destinations > 5 {
            alerts.push(AnomalyAlert {
                source_ip: sample.source_ip.clone(),
                anomaly_type: "port_scan_detected".to_string(),
                severity: "HIGH".to_string(),
                score: z,
                baseline_mean: mean,
                observed_value: sample.unique_destinations as f32,
                description: format!(
                    "{} scans {} unique IPs (baseline: {:.0})",
                    sample.source_ip, sample.unique_destinations, mean
                ),
                detected_at: now.clone(),
            });
        }
    }

    // High port scan score → likely automated scanner
    if sample.port_scan_score > 0.7 {
        alerts.push(AnomalyAlert {
            source_ip: sample.source_ip.clone(),
            anomaly_type: "automated_scanner".to_string(),
            severity: if sample.port_scan_score > 0.9 {
                "CRITICAL"
            } else {
                "HIGH"
            }
            .to_string(),
            score: sample.port_scan_score,
            baseline_mean: 0.0,
            observed_value: sample.port_scan_score,
            description: format!(
                "{} shows automated scanner pattern (score={:.2})",
                sample.source_ip, sample.port_scan_score
            ),
            detected_at: now.clone(),
        });
    }

    // Update baseline AFTER detection
    baseline.push(
        sample.packet_count as f32,
        sample.byte_count as f32,
        sample.unique_destinations as f32,
    );

    if !alerts.is_empty() {
        crate::push_runtime_log(
            &log_state,
            format!("ANOMALY|ip={}|alerts={}", sample.source_ip, alerts.len()),
        );
    }

    Ok(alerts)
}

#[tauri::command]
pub fn get_anomaly_baselines(
    anomaly_state: State<'_, AnomalyState>,
) -> Result<Vec<serde_json::Value>, String> {
    let baselines = anomaly_state.baselines.lock().map_err(|_| "lock")?;
    let result = baselines
        .iter()
        .map(|(ip, b)| {
            let n = b.packet_hist.len() as f32;
            let mean_pkts = if n > 0.0 {
                b.packet_hist.iter().sum::<f32>() / n
            } else {
                0.0
            };
            let mean_dests = if n > 0.0 {
                b.dest_hist.iter().sum::<f32>() / n
            } else {
                0.0
            };
            serde_json::json!({
                "ip": ip,
                "observations": b.packet_hist.len(),
                "mean_packets": (mean_pkts * 10.0).round() / 10.0,
                "mean_dests": (mean_dests * 10.0).round() / 10.0,
            })
        })
        .collect();
    Ok(result)
}

#[tauri::command]
pub fn reset_anomaly_baseline(
    ip: String,
    anomaly_state: State<'_, AnomalyState>,
) -> Result<(), String> {
    let mut baselines = anomaly_state.baselines.lock().map_err(|_| "lock")?;
    baselines.remove(&ip);
    Ok(())
}
