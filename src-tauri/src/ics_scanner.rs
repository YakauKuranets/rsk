use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IcsScanResult {
    pub ip: String,
    pub port: u16,
    pub protocol: String,
    pub device_id: Option<String>,
    pub vendor: Option<String>,
    pub firmware: Option<String>,
    pub writable_registers: bool,
    pub risk: String,
    pub raw_response: String,
}

/// Probe Modbus/TCP — Function Code 0x11 (Report Slave ID)
async fn modbus_probe(ip: &str, port: u16) -> Result<IcsScanResult, String> {
    let addr = format!("{}:{}", ip, port);
    let mut stream = timeout(Duration::from_secs(5), TcpStream::connect(&addr))
        .await
        .map_err(|_| "timeout")?
        .map_err(|e| e.to_string())?;

    // Modbus ADU: transaction=0x0001, protocol=0x0000, len=0x0002, unit=0x01, fc=0x11
    let req: &[u8] = &[0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x01, 0x11];
    stream.write_all(req).await.map_err(|e| e.to_string())?;

    let mut buf = vec![0u8; 256];
    let n = timeout(Duration::from_secs(3), stream.read(&mut buf))
        .await
        .map_err(|_| "read timeout")?
        .map_err(|e| e.to_string())?;

    let raw = hex::encode(&buf[..n]);
    let vendor = if n > 9 {
        String::from_utf8_lossy(&buf[9..n]).trim().to_string()
    } else {
        String::new()
    };

    Ok(IcsScanResult {
        ip: ip.to_string(),
        port,
        protocol: "Modbus/TCP".to_string(),
        device_id: Some(format!("unit_id={}", buf[6])),
        vendor: if vendor.is_empty() {
            None
        } else {
            Some(vendor)
        },
        firmware: None,
        writable_registers: false,
        risk: "HIGH".to_string(),
        raw_response: raw,
    })
}

/// Check if Modbus allows write (FC06 Write Single Register to 40001)
async fn check_modbus_write(ip: &str, port: u16) -> bool {
    let addr = format!("{}:{}", ip, port);
    let Ok(Ok(mut s)) = timeout(Duration::from_secs(4), TcpStream::connect(&addr)).await else {
        return false;
    };

    // FC 06: write register 0x0000 value 0x0000 (safe no-op on most PLCs)
    let req: &[u8] = &[
        0x00, 0x02, 0x00, 0x00, 0x00, 0x06, 0x01, 0x06, 0x00, 0x00, 0x00, 0x00,
    ];
    let _ = s.write_all(req).await;
    let mut buf = [0u8; 12];
    matches!(timeout(Duration::from_secs(2), s.read(&mut buf)).await, Ok(Ok(n)) if n >= 8)
}

/// Probe BACnet/IP (UDP 47808) — Who-Is broadcast
async fn bacnet_probe(ip: &str) -> Result<IcsScanResult, String> {
    use tokio::net::UdpSocket;
    let sock = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| e.to_string())?;
    sock.connect(format!("{}:47808", ip))
        .await
        .map_err(|e| e.to_string())?;
    // BACnet Who-Is: BVLC + NPDU + APDU
    let who_is: &[u8] = &[
        0x81, 0x0a, 0x00, 0x08, 0x01, 0x20, 0xff, 0xff, 0x00, 0xff, 0x10, 0x08,
    ];
    let _ = timeout(Duration::from_secs(2), sock.send(who_is)).await;
    let mut buf = [0u8; 512];
    let n = timeout(Duration::from_secs(3), sock.recv(&mut buf))
        .await
        .map_err(|_| "no BACnet response")?
        .map_err(|e| e.to_string())?;
    Ok(IcsScanResult {
        ip: ip.to_string(),
        port: 47808,
        protocol: "BACnet/IP".to_string(),
        device_id: Some(hex::encode(&buf[..n.min(16)])),
        vendor: None,
        firmware: None,
        writable_registers: false,
        risk: "CRITICAL".to_string(),
        raw_response: hex::encode(&buf[..n]),
    })
}

#[tauri::command]
pub async fn ics_full_scan(
    ip: String,
    permit_token: String,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<IcsScanResult>, String> {
    if permit_token.trim().len() < 8 {
        return Err("Unauthorized: provide valid permit token".to_string());
    }
    crate::push_runtime_log(
        &log_state,
        format!("ICS_SCAN|ip={}|permit={}", ip, &permit_token[..8]),
    );

    let mut results = Vec::new();

    // 1. Modbus TCP port 502
    if let Ok(mut r) = modbus_probe(&ip, 502).await {
        r.writable_registers = check_modbus_write(&ip, 502).await;
        if r.writable_registers {
            r.risk = "CRITICAL".to_string();
        }
        results.push(r);
    }

    // 2. BACnet UDP 47808
    if let Ok(r) = bacnet_probe(&ip).await {
        results.push(r);
    }

    // 3. DNP3 port 20000
    if let Ok(s) = TcpStream::connect(format!("{}:20000", ip)).await {
        drop(s);
        results.push(IcsScanResult {
            ip: ip.clone(),
            port: 20000,
            protocol: "DNP3".to_string(),
            device_id: None,
            vendor: None,
            firmware: None,
            writable_registers: false,
            risk: "HIGH".to_string(),
            raw_response: "port open".to_string(),
        });
    }

    crate::push_runtime_log(
        &log_state,
        format!("ICS_SCAN_DONE|ip={}|found={}", ip, results.len()),
    );
    Ok(results)
}
