// src-tauri/src/msf_client.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MsfModule {
    pub fullname: String,
    pub name: String,
    pub rank: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MsfSession {
    pub id: u64,
    pub session_type: String,
    pub target_host: String,
    pub info: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MsfExecResult {
    pub job_id: Option<u64>,
    pub uuid: String,
    pub status: String,
}

fn msf_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn msf_authenticate(
    host: String,
    port: u16,
    username: String,
    password: String,
    permit_token: String,
    log_state: State<'_, crate::LogState>,
) -> Result<String, String> {
    if permit_token.trim().len() < 8 {
        return Err("Unauthorized".to_string());
    }

    let c = msf_client()?;
    let resp: serde_json::Value = c
        .post(&format!("http://{}:{}/api/v1/auth/login", host, port))
        .json(&serde_json::json!({"username": username, "password": password}))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    crate::push_runtime_log(
        &log_state,
        format!("MSF_AUTH|{}:{}|permit={}", host, port, &permit_token[..8]),
    );

    resp["data"]["token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No token".to_string())
}

#[tauri::command]
pub async fn msf_search(
    host: String,
    port: u16,
    token: String,
    query: String,
) -> Result<Vec<MsfModule>, String> {
    let c = msf_client()?;
    let resp: serde_json::Value = c
        .get(&format!(
            "http://{}:{}/api/v1/modules/search?query={}",
            host,
            port,
            urlencoding::encode(&query)
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|m| MsfModule {
            fullname: m["fullname"].as_str().unwrap_or("").to_string(),
            name: m["name"].as_str().unwrap_or("").to_string(),
            rank: m["rank"].as_str().unwrap_or("").to_string(),
            description: m["description"].as_str().unwrap_or("").to_string(),
        })
        .collect())
}

#[tauri::command]
pub async fn msf_run_module(
    host: String,
    port: u16,
    token: String,
    module_type: String,
    module_name: String,
    options: HashMap<String, String>,
    permit_token: String,
    log_state: State<'_, crate::LogState>,
) -> Result<MsfExecResult, String> {
    if permit_token.trim().len() < 8 {
        return Err("Unauthorized".to_string());
    }

    let c = msf_client()?;
    let resp: serde_json::Value = c
        .post(&format!(
            "http://{}:{}/api/v1/modules/{}/{}/launch",
            host, port, module_type, module_name
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({"datastore": options}))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    crate::push_runtime_log(
        &log_state,
        format!("MSF_RUN|{}/{}|permit={}", module_type, module_name, &permit_token[..8]),
    );

    Ok(MsfExecResult {
        job_id: resp["data"]["job_id"].as_u64(),
        uuid: resp["data"]["uuid"].as_str().unwrap_or("").to_string(),
        status: if resp["data"]["job_id"].is_number() {
            "launched"
        } else {
            "error"
        }
        .to_string(),
    })
}

#[tauri::command]
pub async fn msf_list_sessions(
    host: String,
    port: u16,
    token: String,
) -> Result<Vec<MsfSession>, String> {
    let c = msf_client()?;
    let resp: serde_json::Value = c
        .get(&format!("http://{}:{}/api/v1/sessions", host, port))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp["data"]
        .as_object()
        .cloned()
        .unwrap_or_default()
        .values()
        .map(|s| MsfSession {
            id: s["id"].as_u64().unwrap_or(0),
            session_type: s["type"].as_str().unwrap_or("").to_string(),
            target_host: s["target_host"].as_str().unwrap_or("").to_string(),
            info: s["info"].as_str().unwrap_or("").to_string(),
        })
        .collect())
}
