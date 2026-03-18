// src-tauri/src/tool_executor.rs
// Unified tool execution API — runs any installed security tool
// Wraps: nmap, nikto, nuclei, amass, john, netcat, hydra, sqlmap
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub tool: String,           // "nmap" | "nikto" | "nuclei" | "hydra" | "sqlmap"
    pub target: String,
    pub args: Vec<String>,      // additional args
    pub timeout_secs: Option<u64>,
    pub permit_token: String,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub tool: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u128,
    pub findings_extracted: Vec<String>,
}


/// Allowed tools whitelist — only these can be executed
const ALLOWED_TOOLS: &[&str] = &[
    "nmap", "nikto", "nuclei", "amass", "gobuster",
    "hydra", "john", "hashcat", "sqlmap", "whatweb",
    "masscan", "rustscan", "ffuf", "feroxbuster",
];


/// Extract high-value lines from tool output
fn extract_findings(tool: &str, output: &str) -> Vec<String> {
    let lines: Vec<&str> = output.lines().collect();
    match tool {
        "nmap" => lines.iter()
            .filter(|l| l.contains("/tcp") || l.contains("/udp"))
            .filter(|l| l.contains("open"))
            .map(|l| format!("[nmap] {}", l.trim()))
            .collect(),
        "nikto" => lines.iter()
            .filter(|l| l.starts_with("+ "))
            .map(|l| format!("[nikto] {}", l.trim_start_matches("+ ")))
            .collect(),
        "nuclei" => lines.iter()
            .filter(|l| l.contains("[") && (l.contains("critical") || l.contains("high")))
            .map(|l| format!("[nuclei] {}", l.trim()))
            .collect(),
        "hydra" => lines.iter()
            .filter(|l| l.contains("[DATA]") && l.contains("found"))
            .map(|l| format!("[hydra] CREDENTIAL: {}", l.trim()))
            .collect(),
        _ => lines.iter().take(50).map(|l| l.to_string()).collect(),
    }
}


#[tauri::command]
pub async fn execute_tool(
    req: ToolRequest,
    log_state: State<'_, crate::LogState>,
) -> Result<ToolResult, String> {
    if req.permit_token.trim().len() < 8 {
        return Err("Unauthorized: provide valid permit token".to_string());
    }


    // Security: only allow whitelisted tools
    let tool_name = req.tool.split('/').last().unwrap_or(&req.tool);
    if !ALLOWED_TOOLS.contains(&tool_name) {
        return Err(format!("Tool '{}' not allowed. Allowed: {:?}", tool_name, ALLOWED_TOOLS));
    }


    // Security: block dangerous flags
    let dangerous = ["--script=exploit", "-iL /etc/", ">/etc/", ";rm ", "&&rm "];
    for arg in &req.args {
        if dangerous.iter().any(|d| arg.contains(d)) {
            return Err(format!("Dangerous argument blocked: {}", arg));
        }
    }


    crate::push_runtime_log(&log_state, format!(
        "TOOL_EXEC|tool={}|target={}|permit={}",
        req.tool, req.target, &req.permit_token[..8]));


    let timeout = req.timeout_secs.unwrap_or(120);
    let t_start = std::time::Instant::now();


    let mut cmd = tokio::process::Command::new(&req.tool);
    cmd.args(&req.args).arg(&req.target);


    let output = tokio::time::timeout(
        Duration::from_secs(timeout),
        cmd.output(),
    ).await
    .map_err(|_| format!("{} timed out after {}s", req.tool, timeout))?
    .map_err(|e| format!("{} not installed or failed: {}", req.tool, e))?;


    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let findings = extract_findings(&req.tool, &stdout);


    crate::push_runtime_log(&log_state, format!(
        "TOOL_DONE|tool={}|exit={}|findings={}",
        req.tool, output.status.code().unwrap_or(-1), findings.len()));


    Ok(ToolResult {
        tool: req.tool,
        exit_code: output.status.code().unwrap_or(-1),
        stdout: stdout.chars().take(50000).collect(), // limit output size
        stderr: stderr.chars().take(5000).collect(),
        duration_ms: t_start.elapsed().as_millis(),
        findings_extracted: findings,
    })
}


#[tauri::command]
pub async fn check_tools_available() -> Vec<serde_json::Value> {
    let mut results = Vec::new();
    for &tool in ALLOWED_TOOLS {
        let available = tokio::process::Command::new("which")
            .arg(tool).output().await
            .map(|o| o.status.success()).unwrap_or(false);
        results.push(serde_json::json!({
            "tool": tool,
            "available": available,
        }));
    }
    results
}

