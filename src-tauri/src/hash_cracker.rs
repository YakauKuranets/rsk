use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HashType {
    NTLM,
    MD5,
    SHA1,
    SHA256,
    SHA512,
    Bcrypt,
    Argon2,
    WPA,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashCrackResult {
    pub hash: String,
    pub hash_type: String,
    pub cracked: bool,
    pub plaintext: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuBenchmark {
    pub device: String,
    pub md5_hps: u64,
    pub ntlm_hps: u64,
    pub wpa_hps: u64,
}

#[tauri::command]
pub fn identify_hash(hash: String) -> String {
    let h = hash.trim();
    match h.len() {
        32 if h.chars().all(|c| c.is_ascii_hexdigit()) => "MD5".to_string(),
        40 if h.chars().all(|c| c.is_ascii_hexdigit()) => "SHA1".to_string(),
        64 if h.chars().all(|c| c.is_ascii_hexdigit()) => "SHA256".to_string(),
        128 if h.chars().all(|c| c.is_ascii_hexdigit()) => "SHA512".to_string(),
        32 if h
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c.is_ascii_uppercase()) =>
        {
            "NTLM".to_string()
        }
        _ if h.starts_with("$2") => "Bcrypt".to_string(),
        _ if h.starts_with("$argon2") => "Argon2".to_string(),
        _ if h.starts_with("$P$") => "PHPass".to_string(),
        60 if h.starts_with("$2b$") || h.starts_with("$2a$") => "Bcrypt".to_string(),
        _ => "Unknown".to_string(),
    }
}

fn hash_type_to_mode(hash_type: &str) -> Option<&'static str> {
    match hash_type {
        "MD5" => Some("0"),
        "SHA1" => Some("100"),
        "SHA256" => Some("1400"),
        "SHA512" => Some("1700"),
        "NTLM" => Some("1000"),
        "WPA" => Some("22000"),
        "Bcrypt" => Some("3200"),
        _ => None,
    }
}

#[tauri::command]
pub async fn crack_hashes(
    hashes: Vec<String>,
    wordlist_path: String,
    hash_type: String,
    permit_token: String,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<HashCrackResult>, String> {
    if permit_token.trim().len() < 8 {
        return Err("Unauthorized: provide valid permit token".to_string());
    }
    let mode = hash_type_to_mode(&hash_type)
        .ok_or_else(|| format!("Unsupported hash type: {}", hash_type))?;

    crate::push_runtime_log(
        &log_state,
        format!(
            "HASH_CRACK|type={}|count={}|permit={}",
            hash_type,
            hashes.len(),
            &permit_token[..8]
        ),
    );

    // Write hashes to temp file
    let hash_file = crate::get_vault_path().join("crack_input.txt");
    tokio::fs::write(&hash_file, hashes.join("\n"))
        .await
        .map_err(|e| e.to_string())?;

    let potfile = crate::get_vault_path().join("crack_output.pot");
    let start = std::time::Instant::now();

    // Run hashcat
    let _status = tokio::time::timeout(
        Duration::from_secs(300),
        tokio::process::Command::new("hashcat")
            .args([
                "-m",
                mode,
                "-a",
                "0",
                hash_file.to_str().unwrap_or(""),
                &wordlist_path,
                "--potfile-path",
                potfile.to_str().unwrap_or(""),
                "--quiet",
                "--status",
                "--machine-readable",
            ])
            .output(),
    )
    .await
    .ok();

    let duration_ms = start.elapsed().as_millis() as u64;

    // Parse potfile for results
    let pot_content = tokio::fs::read_to_string(&potfile)
        .await
        .unwrap_or_default();
    let mut cracked_map = std::collections::HashMap::new();
    for line in pot_content.lines() {
        if let Some(colon) = line.rfind(':') {
            cracked_map.insert(line[..colon].to_string(), line[colon + 1..].to_string());
        }
    }

    let results = hashes
        .iter()
        .map(|h| {
            let cracked = cracked_map.get(h.as_str());
            HashCrackResult {
                hash: h.clone(),
                hash_type: hash_type.clone(),
                cracked: cracked.is_some(),
                plaintext: cracked.cloned(),
                duration_ms,
            }
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub async fn gpu_benchmark() -> Result<Vec<GpuBenchmark>, String> {
    let output = tokio::process::Command::new("hashcat")
        .args(["-b", "--machine-readable", "--quiet"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&output.stdout);
    // Parse "DEVICE_ID:HASH_MODE:H/s" format
    let mut benchmarks: std::collections::HashMap<String, GpuBenchmark> = Default::default();
    for line in text.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 3 {
            continue;
        }
        let dev = parts[0].to_string();
        let mode = parts[1];
        let hps: u64 = parts.last().unwrap_or(&"0").trim().parse().unwrap_or(0);
        let entry = benchmarks.entry(dev.clone()).or_insert(GpuBenchmark {
            device: dev,
            md5_hps: 0,
            ntlm_hps: 0,
            wpa_hps: 0,
        });
        match mode {
            "0" => entry.md5_hps = hps,
            "1000" => entry.ntlm_hps = hps,
            "22000" => entry.wpa_hps = hps,
            _ => {}
        }
    }
    Ok(benchmarks.into_values().collect())
}
