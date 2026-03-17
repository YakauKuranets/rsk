use std::time::Duration;
use tauri::command;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(serde::Serialize)]
pub struct CameraMetadata {
    pub server_header: String,
    pub session_name: String, // Название устройства/камеры
}

/// Безопасный сбор метаданных через RTSP DESCRIBE
#[command]
pub async fn collect_metadata(ip: String) -> Result<CameraMetadata, String> {
    let addr = format!("{}:554", ip);
    let mut stream = tokio::time::timeout(Duration::from_secs(2), TcpStream::connect(&addr))
        .await
        .map_err(|_| "Таймаут подключения")?
        .map_err(|e| e.to_string())?;

    // Отправляем базовый запрос DESCRIBE без авторизации.
    // Многие камеры отдают базовый SDP или хотя бы заголовок Server даже при ошибке 401.
    let request = format!(
        "DESCRIBE rtsp://{}:554/ RTSP/1.0\r\nCSeq: 1\r\nAccept: application/sdp\r\n\r\n",
        ip
    );
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|e| e.to_string())?;

    let mut buf = [0; 2048];
    let n = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf))
        .await
        .map_err(|_| "Таймаут чтения")?
        .map_err(|e| e.to_string())?;

    let response = String::from_utf8_lossy(&buf[..n]);

    let mut meta = CameraMetadata {
        server_header: "Unknown".into(),
        session_name: "Not specified".into(),
    };

    // Парсим ответ построчно
    for line in response.lines() {
        if line.to_lowercase().starts_with("server:") {
            meta.server_header = line[7..].trim().to_string();
        }
        // В SDP протоколе строка "s=" означает Session Name (обычно это имя камеры)
        if line.starts_with("s=") {
            meta.session_name = line[2..].trim().to_string();
        }
    }

    Ok(meta)
}
