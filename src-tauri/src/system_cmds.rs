use serde::Serialize;
use tauri::State;
use tokio::{
    net::TcpStream,
    time::{timeout, Duration},
};

#[derive(Debug, Serialize)]
pub struct PortProbeResult {
    port: u16,
    service: String,
    open: bool,
}

fn guess_service(port: u16) -> &'static str {
    match port {
        21 => "ftp/archive",
        22 => "ssh/sftp",
        80 => "http/admin",
        443 => "https/admin",
        554 => "rtsp/video",
        8080 => "http-alt/admin",
        8443 => "https-alt/admin",
        _ => "unknown",
    }
}

#[tauri::command]
pub async fn scan_host_ports(
    host: String,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<PortProbeResult>, String> {
    let clean_host = crate::normalize_host_for_scan(&host);
    if clean_host.is_empty() {
        return Err("Пустой host для сканирования".into());
    }

    crate::push_runtime_log(&log_state, format!("Port scan started for {}", clean_host));

    let ports = [21u16, 22, 80, 443, 554, 8080, 8443];
    let mut result = Vec::with_capacity(ports.len());

    for port in ports {
        let addr = format!("{}:{}", clean_host, port);
        let open = timeout(Duration::from_millis(900), TcpStream::connect(addr))
            .await
            .is_ok_and(|v| v.is_ok());

        result.push(PortProbeResult {
            port,
            service: guess_service(port).to_string(),
            open,
        });
    }

    let open_count = result.iter().filter(|x| x.open).count();
    crate::push_runtime_log(
        &log_state,
        format!(
            "Port scan finished for {} (open: {})",
            clean_host, open_count
        ),
    );
    Ok(result)
}
