// src-tauri/src/firmware_intelligence.rs
// Deep firmware fingerprinting → automatic CVE matching
// No ML model needed — pattern matching + NVD database
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareFingerprint {
    pub ip: String,
    pub vendor: String,
    pub model: Option<String>,
    pub firmware_version: Option<String>,
    pub build_date: Option<String>,
    pub detection_method: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareIntelReport {
    pub fingerprint: FirmwareFingerprint,
    pub matched_cves: Vec<crate::vuln_db_updater::VulnDbEntry>,
    pub risk_score: f32,
    pub recommendations: Vec<String>,
}

/// Probe HTTP headers and HTML for vendor/firmware clues
async fn probe_http(ip: &str, client: &Client) -> Option<FirmwareFingerprint> {
    let url = format!("http://{}/", ip);
    let Ok(Ok(resp)) = tokio::time::timeout(Duration::from_secs(5), client.get(&url).send()).await else {
        return None;
    };

    let server = resp
        .headers()
        .get("server")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let www_auth = resp
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    let Ok(Ok(body)) = tokio::time::timeout(Duration::from_secs(3), resp.text()).await else {
        return None;
    };
    let body_low = body.to_lowercase();

    // Vendor detection rules
    let (vendor, model, confidence) = if body_low.contains("hikvision") || server.contains("hikvision") {
        let model = extract_between(&body, "DS-", "\"");
        ("Hikvision", model, 0.95)
    } else if body_low.contains("dahua") || www_auth.contains("dahua") {
        let model = extract_between(&body, "IPC-", "\"");
        ("Dahua", model, 0.95)
    } else if body_low.contains("axis") && body_low.contains("camera") {
        let model = extract_between(&body, "AXIS ", "<");
        ("Axis", model, 0.88)
    } else if body_low.contains("bosch") && body_low.contains("camera") {
        ("Bosch", None, 0.80)
    } else if body_low.contains("hanwha") || body_low.contains("samsung techwin") {
        ("Hanwha", None, 0.85)
    } else if server.contains("uc-httpd") || server.contains("mini_httpd") {
        ("Generic IoT (uc-httpd)", None, 0.60)
    } else if body_low.contains("onvif") {
        ("ONVIF-compatible (unknown vendor)", None, 0.40)
    } else {
        return None;
    };

    // Firmware version extraction
    let fw_ver = extract_firmware_version(&body);
    let build = extract_between(&body, "Build ", "\"").or_else(|| extract_between(&body, "build:", "\""));

    Some(FirmwareFingerprint {
        ip: ip.to_string(),
        vendor: vendor.to_string(),
        model,
        firmware_version: fw_ver,
        build_date: build,
        detection_method: "http_fingerprint".to_string(),
        confidence,
    })
}

fn extract_between(s: &str, after: &str, before: &str) -> Option<String> {
    let start = s.find(after)? + after.len();
    let end = s[start..]
        .find(before)
        .map(|e| start + e)
        .unwrap_or(start + 32);
    let result = s[start..end.min(s.len())].trim().to_string();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn extract_firmware_version(body: &str) -> Option<String> {
    // Common patterns: "V5.4.2", "firmware_version: 2.800.0000", "V2.800.0000"
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"[Vv]?(\d+\.\d+\.\d+(?:\.\d+)?)").expect("static"));
    re.find(body).map(|m| m.as_str().to_string())
}

/// Probe ONVIF GetDeviceInformation for additional details
async fn probe_onvif(ip: &str, client: &Client) -> Option<FirmwareFingerprint> {
    let soap = r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
        <s:Body><GetDeviceInformation
          xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body>
    </s:Envelope>"#;

    let url = format!("http://{}/onvif/device_service", ip);
    let Ok(Ok(resp)) = tokio::time::timeout(
        Duration::from_secs(5),
        client
            .post(&url)
            .header("Content-Type", "application/soap+xml")
            .body(soap)
            .send(),
    )
    .await
    else {
        return None;
    };

    let Ok(Ok(body)) = tokio::time::timeout(Duration::from_secs(3), resp.text()).await else {
        return None;
    };

    let manufacturer = extract_xml_tag(&body, "Manufacturer");
    let model = extract_xml_tag(&body, "Model");
    let fw_ver = extract_xml_tag(&body, "FirmwareVersion");

    manufacturer.as_ref()?;

    Some(FirmwareFingerprint {
        ip: ip.to_string(),
        vendor: manufacturer.unwrap_or_else(|| "Unknown".to_string()),
        model,
        firmware_version: fw_ver,
        build_date: None,
        detection_method: "onvif".to_string(),
        confidence: 0.98,
    })
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close).map(|e| start + e)?;
    Some(xml[start..end].trim().to_string())
}

#[tauri::command]
pub async fn fingerprint_device(
    ip: String,
    log_state: State<'_, crate::LogState>,
) -> Result<FirmwareIntelReport, String> {
    crate::push_runtime_log(&log_state, format!("FIRMWARE_FP|ip={}", ip));

    let client = Client::builder()
        .timeout(Duration::from_secs(8))
        // device client: self-signed certs on local network hardware
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    // Try ONVIF first (most reliable), fall back to HTTP
    let fingerprint = if let Some(fp) = probe_onvif(&ip, &client).await {
        fp
    } else if let Some(fp) = probe_http(&ip, &client).await {
        fp
    } else {
        return Err(format!("Could not fingerprint {}", ip));
    };

    // Match against local CVE database
    let matched_cves = crate::vuln_db_updater::query_local_vuln_db(
        fingerprint.vendor.clone(),
        fingerprint.firmware_version.clone().unwrap_or_default(),
    )
    .await
    .unwrap_or_default();

    // Calculate risk score
    let max_cvss = matched_cves
        .iter()
        .filter_map(|c| c.cvss_v31)
        .fold(0.0f32, f32::max);
    let kev_count = matched_cves.iter().filter(|c| c.in_kev).count() as f32;
    let risk_score = (max_cvss + kev_count * 1.5).min(10.0);

    // Generate recommendations
    let mut recs = Vec::new();
    if !matched_cves.is_empty() {
        recs.push(format!(
            "Update {} firmware — {} CVEs found",
            fingerprint.vendor,
            matched_cves.len()
        ));
    }
    if matched_cves.iter().any(|c| c.in_kev) {
        recs.push("CRITICAL: Device has actively exploited CVEs (CISA KEV). Isolate immediately.".to_string());
    }
    if fingerprint
        .firmware_version
        .as_deref()
        .map(|v| v.starts_with("1.") || v.starts_with("2."))
        .unwrap_or(false)
    {
        recs.push("Firmware version appears outdated. Check vendor release notes.".to_string());
    }

    crate::push_runtime_log(
        &log_state,
        format!(
            "FIRMWARE_FP_DONE|ip={}|vendor={}|cves={}|risk={:.1}",
            ip,
            fingerprint.vendor,
            matched_cves.len(),
            risk_score
        ),
    );

    Ok(FirmwareIntelReport {
        fingerprint,
        matched_cves,
        risk_score,
        recommendations: recs,
    })
}

#[tauri::command]
pub async fn bulk_fingerprint(
    ips: Vec<String>,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<FirmwareIntelReport>, String> {
    let mut results = Vec::new();
    for ip in ips {
        match fingerprint_device(ip, log_state.clone()).await {
            Ok(r) => results.push(r),
            Err(e) => crate::push_runtime_log(&log_state, format!("FIRMWARE_FP_SKIP|err={}", e)),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
    Ok(results)
}
