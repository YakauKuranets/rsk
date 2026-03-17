use serde::Serialize;
use std::time::Duration;
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::process::Command;
use tokio::time::{sleep, timeout};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceMetadata {
    pub ip: String,
    pub device_name: String,
    pub serial_number: Option<String>,
    pub firmware_version: Option<String>,
    pub hardware_model: Option<String>,
    pub mac_address: Option<String>,
    pub uptime: Option<String>,
    pub vendor_confirmed: String,
    pub collection_methods: Vec<String>,
}

#[tauri::command]
pub async fn collect_device_metadata(
    ip: String,
    snmp_community: Option<String>,
    onvif_port: Option<u16>,
    log_state: State<'_, crate::LogState>,
) -> Result<DeviceMetadata, String> {
    crate::push_runtime_log(&log_state, format!("[DEVICE_META] Start {}", ip));

    let mut out = DeviceMetadata {
        ip: ip.clone(),
        device_name: "Unknown device".into(),
        serial_number: None,
        firmware_version: None,
        hardware_model: None,
        mac_address: None,
        uptime: None,
        vendor_confirmed: "unknown".into(),
        collection_methods: Vec::new(),
    };

    if let Ok(http_data) = collect_http_fingerprint(&ip).await {
        if let Some(title) = http_data.title {
            out.device_name = title;
        }
        if let Some(server) = http_data.server {
            if out.vendor_confirmed == "unknown" {
                out.vendor_confirmed = infer_vendor(&server);
            }
        }
        if let Some(hash) = http_data.favicon_hash {
            out.hardware_model
                .get_or_insert(format!("favicon:{}", hash));
        }
        out.collection_methods.push("http_fingerprint".into());
    }

    sleep(Duration::from_millis(180)).await;

    if let Ok(snmp) = collect_snmp_metadata(&ip, snmp_community).await {
        if out.device_name == "Unknown device" {
            if let Some(name) = snmp.sys_name {
                out.device_name = name;
            }
        }
        out.hardware_model = out.hardware_model.or(snmp.model);
        out.firmware_version = out.firmware_version.or(snmp.firmware);
        out.uptime = out.uptime.or(snmp.uptime);
        out.mac_address = out.mac_address.or(snmp.mac);
        out.collection_methods.push("snmp".into());
    }

    sleep(Duration::from_millis(180)).await;

    if let Ok(ssdp_vendor) = collect_ssdp_vendor(&ip).await {
        if out.vendor_confirmed == "unknown" {
            out.vendor_confirmed = ssdp_vendor;
        }
        out.collection_methods.push("ssdp".into());
    }

    sleep(Duration::from_millis(180)).await;

    if let Ok(onvif) = collect_onvif_metadata(&ip, onvif_port.unwrap_or(80)).await {
        out.hardware_model = out.hardware_model.or(onvif.model);
        out.firmware_version = out.firmware_version.or(onvif.firmware);
        out.serial_number = out.serial_number.or(onvif.serial);
        out.vendor_confirmed = if out.vendor_confirmed == "unknown" {
            onvif.manufacturer.unwrap_or_else(|| "unknown".into())
        } else {
            out.vendor_confirmed
        };
        out.collection_methods.push("onvif".into());
    }

    if out.collection_methods.is_empty() {
        return Err("Не удалось собрать метаданные ни одним методом".into());
    }

    crate::push_runtime_log(
        &log_state,
        format!(
            "[DEVICE_META] Done {} via {}",
            out.ip,
            out.collection_methods.join(",")
        ),
    );

    Ok(out)
}

struct HttpFingerprint {
    server: Option<String>,
    title: Option<String>,
    favicon_hash: Option<String>,
}

async fn collect_http_fingerprint(ip: &str) -> Result<HttpFingerprint, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("http://{}/", ip);
    let resp = timeout(Duration::from_secs(6), client.get(&url).send())
        .await
        .map_err(|_| "http timeout".to_string())?
        .map_err(|e| e.to_string())?;

    let server = resp
        .headers()
        .get("server")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let body = resp.text().await.unwrap_or_default();

    let title_re = regex::Regex::new(r"(?is)<title>(.*?)</title>").map_err(|e| e.to_string())?;
    let title = title_re
        .captures(&body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|t| !t.is_empty());

    let fav_url = format!("http://{}/favicon.ico", ip);
    let fav_hash = match timeout(Duration::from_secs(5), client.get(&fav_url).send()).await {
        Ok(Ok(resp)) if resp.status().is_success() => match resp.bytes().await {
            Ok(bytes) => {
                use sha1::{Digest, Sha1};
                let mut hasher = Sha1::new();
                hasher.update(&bytes);
                Some(format!("{:X}", hasher.finalize()))
            }
            Err(_) => None,
        },
        _ => None,
    };

    Ok(HttpFingerprint {
        server,
        title,
        favicon_hash: fav_hash,
    })
}

struct SnmpMeta {
    sys_name: Option<String>,
    model: Option<String>,
    firmware: Option<String>,
    uptime: Option<String>,
    mac: Option<String>,
}

async fn collect_snmp_metadata(ip: &str, community: Option<String>) -> Result<SnmpMeta, String> {
    let community = community.unwrap_or_else(|| "public".into());
    let target = format!("{}", ip);

    let output = timeout(
        Duration::from_secs(6),
        Command::new("snmpwalk")
            .args([
                "-v2c",
                "-c",
                &community,
                &target,
                "1.3.6.1.2.1.1", // system subtree
            ])
            .output(),
    )
    .await
    .map_err(|_| "snmpwalk timeout".to_string())?
    .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("snmpwalk failed".into());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let extract = |needle: &str| {
        text.lines()
            .find(|l| l.contains(needle))
            .and_then(|l| l.split_once(':'))
            .map(|(_, v)| v.trim().trim_matches('"').to_string())
    };

    Ok(SnmpMeta {
        sys_name: extract("sysName"),
        model: extract("sysDescr"),
        firmware: extract("sysObjectID"),
        uptime: extract("sysUpTime"),
        mac: None,
    })
}

async fn collect_ssdp_vendor(ip: &str) -> Result<String, String> {
    let socket = timeout(Duration::from_secs(3), UdpSocket::bind("0.0.0.0:0"))
        .await
        .map_err(|_| "ssdp bind timeout".to_string())?
        .map_err(|e| e.to_string())?;

    let msg = concat!(
        "M-SEARCH * HTTP/1.1\r\n",
        "HOST: 239.255.255.250:1900\r\n",
        "MAN: \"ssdp:discover\"\r\n",
        "MX: 2\r\n",
        "ST: upnp:rootdevice\r\n\r\n"
    );

    timeout(
        Duration::from_secs(2),
        socket.send_to(msg.as_bytes(), "239.255.255.250:1900"),
    )
    .await
    .map_err(|_| "ssdp send timeout".to_string())?
    .map_err(|e| e.to_string())?;

    let mut buf = [0u8; 2048];
    let (_, src) = timeout(Duration::from_secs(3), socket.recv_from(&mut buf))
        .await
        .map_err(|_| "ssdp recv timeout".to_string())?
        .map_err(|e| e.to_string())?;

    if src.ip().to_string() != ip {
        return Err("ssdp response from another host".into());
    }

    let payload = String::from_utf8_lossy(&buf);
    if payload.to_lowercase().contains("hikvision") {
        return Ok("hikvision".into());
    }
    if payload.to_lowercase().contains("dahua") {
        return Ok("dahua".into());
    }
    Ok("upnp-device".into())
}

struct OnvifMeta {
    manufacturer: Option<String>,
    model: Option<String>,
    firmware: Option<String>,
    serial: Option<String>,
}

async fn collect_onvif_metadata(ip: &str, port: u16) -> Result<OnvifMeta, String> {
    let soap_body = r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\">
  <s:Body>
    <GetDeviceInformation xmlns=\"http://www.onvif.org/ver10/device/wsdl\" />
  </s:Body>
</s:Envelope>"#;

    let addr = format!("{}:{}", ip, port);
    let mut stream = timeout(Duration::from_secs(5), TcpStream::connect(&addr))
        .await
        .map_err(|_| "onvif connect timeout".to_string())?
        .map_err(|e| e.to_string())?;

    let req = format!(
        "POST /onvif/device_service HTTP/1.1\r\nHost: {}\r\nContent-Type: application/soap+xml; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        addr,
        soap_body.len(),
        soap_body
    );

    timeout(Duration::from_secs(4), stream.write_all(req.as_bytes()))
        .await
        .map_err(|_| "onvif write timeout".to_string())?
        .map_err(|e| e.to_string())?;

    let mut data = Vec::new();
    timeout(Duration::from_secs(5), stream.read_to_end(&mut data))
        .await
        .map_err(|_| "onvif read timeout".to_string())?
        .map_err(|e| e.to_string())?;

    let body = String::from_utf8_lossy(&data);
    Ok(OnvifMeta {
        manufacturer: extract_xml_tag(&body, "Manufacturer"),
        model: extract_xml_tag(&body, "Model"),
        firmware: extract_xml_tag(&body, "FirmwareVersion"),
        serial: extract_xml_tag(&body, "SerialNumber"),
    })
}

fn extract_xml_tag(body: &str, tag: &str) -> Option<String> {
    let re = regex::Regex::new(&format!(r"<[^>]*{}[^>]*>(.*?)</[^>]*{}>", tag, tag)).ok()?;
    re.captures(body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn infer_vendor(server: &str) -> String {
    let s = server.to_lowercase();
    if s.contains("hikvision") {
        "hikvision".into()
    } else if s.contains("dahua") {
        "dahua".into()
    } else if s.contains("axis") {
        "axis".into()
    } else {
        "unknown".into()
    }
}
