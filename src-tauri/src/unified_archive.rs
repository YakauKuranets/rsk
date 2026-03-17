use serde::Serialize;
use std::time::Duration;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveRecord {
    pub id: String,
    pub camera_ip: String,
    pub channel: u32,
    pub start_time: String,
    pub end_time: String,
    pub duration_secs: u64,
    pub protocol: String,
    pub playback_uri: Option<String>,
    pub download_uri: Option<String>,
    pub file_size: Option<u64>,
    pub codec: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSearchResult {
    pub camera_ip: String,
    pub protocol_used: String,
    pub records: Vec<ArchiveRecord>,
    pub total_duration_secs: u64,
    pub errors: Vec<String>,
}

#[tauri::command]
pub async fn search_archive_unified(
    camera_ip: String,
    login: String,
    password: String,
    date_from: String,
    date_to: String,
    channel: Option<u32>,
    log_state: State<'_, crate::LogState>,
) -> Result<ArchiveSearchResult, String> {
    let ch = channel.unwrap_or(1);
    let mut errors = Vec::new();

    crate::push_runtime_log(
        &log_state,
        format!(
            "[ARCHIVE] Unified search: {} ({} — {})",
            camera_ip, date_from, date_to
        ),
    );

    crate::push_runtime_log(&log_state, "[ARCHIVE] Trying ISAPI...".to_string());
    match search_via_isapi(&camera_ip, &login, &password, &date_from, &date_to, ch).await {
        Ok(records) if !records.is_empty() => {
            let total = records.iter().map(|r| r.duration_secs).sum();
            return Ok(ArchiveSearchResult {
                camera_ip,
                protocol_used: "ISAPI".into(),
                records,
                total_duration_secs: total,
                errors,
            });
        }
        Ok(_) => errors.push("ISAPI: no records found".into()),
        Err(e) => errors.push(format!("ISAPI: {}", e)),
    }

    crate::push_runtime_log(&log_state, "[ARCHIVE] Trying ONVIF...".to_string());
    match search_via_onvif(&camera_ip, &login, &password, &date_from, &date_to).await {
        Ok(records) if !records.is_empty() => {
            let total = records.iter().map(|r| r.duration_secs).sum();
            return Ok(ArchiveSearchResult {
                camera_ip,
                protocol_used: "ONVIF".into(),
                records,
                total_duration_secs: total,
                errors,
            });
        }
        Ok(_) => errors.push("ONVIF: no records found".into()),
        Err(e) => errors.push(format!("ONVIF: {}", e)),
    }

    crate::push_runtime_log(&log_state, "[ARCHIVE] Trying XM CGI...".to_string());
    match search_via_xm(&camera_ip, &login, &password, &date_from, &date_to, ch).await {
        Ok(records) if !records.is_empty() => {
            let total = records.iter().map(|r| r.duration_secs).sum();
            return Ok(ArchiveSearchResult {
                camera_ip,
                protocol_used: "XM_CGI".into(),
                records,
                total_duration_secs: total,
                errors,
            });
        }
        Ok(_) => errors.push("XM: no records found".into()),
        Err(e) => errors.push(format!("XM: {}", e)),
    }

    crate::push_runtime_log(&log_state, "[ARCHIVE] Trying FTP...".to_string());
    match search_via_ftp(&camera_ip, &login, &password).await {
        Ok(records) if !records.is_empty() => {
            let total = records.iter().map(|r| r.duration_secs).sum();
            return Ok(ArchiveSearchResult {
                camera_ip,
                protocol_used: "FTP".into(),
                records,
                total_duration_secs: total,
                errors,
            });
        }
        Ok(_) => errors.push("FTP: no files found".into()),
        Err(e) => errors.push(format!("FTP: {}", e)),
    }

    Ok(ArchiveSearchResult {
        camera_ip,
        protocol_used: "none".into(),
        records: Vec::new(),
        total_duration_secs: 0,
        errors,
    })
}

#[tauri::command]
pub async fn download_archive_unified(
    record_id: String,
    camera_ip: String,
    login: String,
    password: String,
    protocol: String,
    playback_uri: Option<String>,
    download_uri: Option<String>,
    filename_hint: Option<String>,
    log_state: State<'_, crate::LogState>,
) -> Result<serde_json::Value, String> {
    crate::push_runtime_log(
        &log_state,
        format!("[ARCHIVE] Download via {}: {}", protocol, camera_ip),
    );

    let filename = filename_hint.unwrap_or_else(|| format!("archive_{}.mp4", record_id));

    match protocol.as_str() {
        "ISAPI" => {
            if let Some(uri) = playback_uri.or(download_uri) {
                let save_path = crate::get_vault_path().join("downloads").join(&filename);
                if let Some(parent) = save_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                download_with_digest_auth(&uri, &login, &password, &save_path).await?;

                Ok(serde_json::json!({
                    "status": "done",
                    "savePath": save_path.to_string_lossy(),
                    "protocol": "ISAPI",
                }))
            } else {
                Err("No playback URI for ISAPI download".into())
            }
        }
        "ONVIF" => {
            if let Some(uri) = playback_uri {
                let save_path = crate::get_vault_path().join("downloads").join(&filename);
                if let Some(parent) = save_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                download_with_digest_auth(&uri, &login, &password, &save_path).await?;

                Ok(serde_json::json!({
                    "status": "done",
                    "savePath": save_path.to_string_lossy(),
                    "protocol": "ONVIF",
                }))
            } else {
                Err("No playback URI for ONVIF download".into())
            }
        }
        "FTP" => {
            if let Some(path) = download_uri {
                let save_path = crate::get_vault_path().join("downloads").join(&filename);
                if let Some(parent) = save_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                download_via_ftp_to_file(&camera_ip, &login, &password, &path, &save_path).await?;

                Ok(serde_json::json!({
                    "status": "done",
                    "savePath": save_path.to_string_lossy(),
                    "protocol": "FTP",
                }))
            } else {
                Err("No FTP path for download".into())
            }
        }
        "RTSP_PLAYBACK" => {
            if let Some(uri) = playback_uri {
                let save_path = crate::get_vault_path().join("downloads").join(&filename);
                if let Some(parent) = save_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                capture_rtsp_to_file(&uri, &save_path, 0).await?;

                Ok(serde_json::json!({
                    "status": "done",
                    "savePath": save_path.to_string_lossy(),
                    "protocol": "RTSP_PLAYBACK",
                }))
            } else {
                Err("No RTSP playback URI".into())
            }
        }
        _ => Err(format!("Unsupported protocol: {}", protocol)),
    }
}

async fn search_via_isapi(
    ip: &str,
    login: &str,
    pass: &str,
    from: &str,
    to: &str,
    channel: u32,
) -> Result<Vec<ArchiveRecord>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let search_url = format!("http://{}/ISAPI/ContentMgmt/search", ip);
    let xml_body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CMSearchDescription>
  <searchID>hyperion_{}</searchID>
  <trackList><trackID>{}</trackID></trackList>
  <timeSpanList>
    <timeSpan>
      <startTime>{}</startTime>
      <endTime>{}</endTime>
    </timeSpan>
  </timeSpanList>
  <maxResults>100</maxResults>
  <searchResultPostion>0</searchResultPostion>
  <metadataList><metadataDescriptor>//recordType.meta.std-cgi.com</metadataDescriptor></metadataList>
</CMSearchDescription>"#,
        chrono::Utc::now().timestamp(),
        format!("{:03}01", channel),
        from,
        to
    );

    let resp = client
        .post(&search_url)
        .header("Content-Type", "application/xml")
        .basic_auth(login, Some(pass))
        .body(xml_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("ISAPI HTTP {}", resp.status()));
    }

    let body = resp.text().await.map_err(|e| e.to_string())?;
    parse_isapi_search_results(&body, ip)
}

fn parse_isapi_search_results(xml: &str, ip: &str) -> Result<Vec<ArchiveRecord>, String> {
    let mut records = Vec::new();

    let re_item = regex::Regex::new(r"<searchMatchItem>([\s\S]*?)</searchMatchItem>")
        .map_err(|e| e.to_string())?;
    let re_start =
        regex::Regex::new(r"<startTime>([^<]+)</startTime>").map_err(|e| e.to_string())?;
    let re_end = regex::Regex::new(r"<endTime>([^<]+)</endTime>").map_err(|e| e.to_string())?;
    let re_uri =
        regex::Regex::new(r"<playbackURI>([^<]+)</playbackURI>").map_err(|e| e.to_string())?;

    for (idx, cap) in re_item.captures_iter(xml).enumerate() {
        let item = &cap[1];
        let start = re_start
            .captures(item)
            .map(|c| c[1].to_string())
            .unwrap_or_default();
        let end = re_end
            .captures(item)
            .map(|c| c[1].to_string())
            .unwrap_or_default();
        let uri = re_uri.captures(item).map(|c| c[1].to_string());

        let duration = calculate_duration_secs(&start, &end);

        records.push(ArchiveRecord {
            id: format!("isapi_{}_{}", ip.replace('.', "_"), idx),
            camera_ip: ip.to_string(),
            channel: 1,
            start_time: start,
            end_time: end,
            duration_secs: duration,
            protocol: "ISAPI".into(),
            playback_uri: uri.clone(),
            download_uri: uri,
            file_size: None,
            codec: None,
        });
    }

    Ok(records)
}

fn calculate_duration_secs(start: &str, end: &str) -> u64 {
    let parse = |s: &str| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ").ok();
    match (parse(start), parse(end)) {
        (Some(s), Some(e)) => (e - s).num_seconds().max(0) as u64,
        _ => 0,
    }
}

async fn search_via_onvif(
    _ip: &str,
    _login: &str,
    _pass: &str,
    _from: &str,
    _to: &str,
) -> Result<Vec<ArchiveRecord>, String> {
    Err("ONVIF search: delegate to existing search_onvif_recordings".into())
}

async fn search_via_xm(
    _ip: &str,
    _login: &str,
    _pass: &str,
    _from: &str,
    _to: &str,
    _channel: u32,
) -> Result<Vec<ArchiveRecord>, String> {
    Err("XM search: delegate to existing search_xm_recordings".into())
}

async fn search_via_ftp(ip: &str, login: &str, pass: &str) -> Result<Vec<ArchiveRecord>, String> {
    use suppaftp::FtpStream;

    let mut ftp =
        FtpStream::connect(format!("{}:21", ip)).map_err(|e| format!("FTP connect: {}", e))?;

    ftp.login(login, pass)
        .map_err(|e| format!("FTP login: {}", e))?;

    let list = ftp.nlst(None).map_err(|e| format!("FTP list: {}", e))?;

    let _ = ftp.quit();

    let video_extensions = [".mp4", ".mkv", ".avi", ".h264", ".265"];
    let records: Vec<ArchiveRecord> = list
        .iter()
        .filter(|f| {
            let lower = f.to_lowercase();
            video_extensions.iter().any(|ext| lower.ends_with(ext))
        })
        .enumerate()
        .map(|(idx, filename)| ArchiveRecord {
            id: format!("ftp_{}_{}", ip.replace('.', "_"), idx),
            camera_ip: ip.to_string(),
            channel: 1,
            start_time: String::new(),
            end_time: String::new(),
            duration_secs: 0,
            protocol: "FTP".into(),
            playback_uri: None,
            download_uri: Some(format!("/{}", filename)),
            file_size: None,
            codec: None,
        })
        .collect();

    Ok(records)
}

async fn download_with_digest_auth(
    url: &str,
    login: &str,
    pass: &str,
    save_path: &std::path::Path,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(url)
        .basic_auth(login, Some(pass))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    std::fs::write(save_path, &bytes).map_err(|e| e.to_string())?;

    Ok(())
}

async fn download_via_ftp_to_file(
    ip: &str,
    login: &str,
    pass: &str,
    remote_path: &str,
    save_path: &std::path::Path,
) -> Result<(), String> {
    use std::io::Cursor;
    use suppaftp::FtpStream;

    let mut ftp = FtpStream::connect(format!("{}:21", ip)).map_err(|e| format!("FTP: {}", e))?;
    ftp.login(login, pass)
        .map_err(|e| format!("FTP login: {}", e))?;

    let cursor = ftp
        .retr_as_buffer(remote_path)
        .map_err(|e| format!("FTP retr error: {}", e))?;
    let bytes = cursor.into_inner();

    let mut reader = Cursor::new(bytes);
    let mut file = std::fs::File::create(save_path).map_err(|e| format!("File create: {}", e))?;

    std::io::copy(&mut reader, &mut file).map_err(|e| format!("File write: {}", e))?;

    let _ = ftp.quit();
    Ok(())
}

async fn capture_rtsp_to_file(
    rtsp_url: &str,
    save_path: &std::path::Path,
    duration_secs: u64,
) -> Result<(), String> {
    let ffmpeg = crate::get_ffmpeg_path();
    let dur = if duration_secs == 0 {
        300
    } else {
        duration_secs
    };

    let status = tokio::process::Command::new(&ffmpeg)
        .args([
            "-rtsp_transport",
            "tcp",
            "-i",
            rtsp_url,
            "-t",
            &dur.to_string(),
            "-c",
            "copy",
            "-y",
            save_path.to_str().unwrap_or("output.mp4"),
        ])
        .status()
        .await
        .map_err(|e| format!("FFmpeg: {}", e))?;

    if !status.success() {
        return Err("FFmpeg capture failed".into());
    }
    Ok(())
}
