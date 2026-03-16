use std::sync::Arc;
use std::time::Duration;
use tauri::command;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;

#[derive(serde::Serialize)]
pub struct MassAuditResult {
    pub ip: String,
    pub is_alive: bool,
    pub creds_reused: bool,
    pub rtsp_path: Option<String>,
}

/// Массовая проверка подсети с лимитом параллельности (Safe Assessment)
#[command]
pub async fn run_mass_audit(
    target_ips: Vec<String>,
    known_login: String,
    known_pass: String,
) -> Result<Vec<MassAuditResult>, String> {
    println!(
        "[MASS_AUDIT] 🚀 Запуск пакетной проверки для {} узлов...",
        target_ips.len()
    );

    // Ограничиваем количество одновременных подключений (не более 15),
    // чтобы не вызвать перегрузку сетевого оборудования (DoS)
    let semaphore = Arc::new(Semaphore::new(15));
    let mut tasks = vec![];

    for ip in target_ips {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| e.to_string())?;
        let login = known_login.clone();
        let pass = known_pass.clone();

        let task = tokio::spawn(async move {
            let _permit = permit; // Удерживаем семафор до конца проверки
            let mut result = MassAuditResult {
                ip: ip.clone(),
                is_alive: false,
                creds_reused: false,
                rtsp_path: None,
            };

            // 1. Быстрый пинг порта 554
            let addr = format!("{}:554", ip);
            if tokio::time::timeout(Duration::from_millis(1000), TcpStream::connect(&addr))
                .await
                .is_err()
            {
                return result; // Узел мертв или порт закрыт
            }
            result.is_alive = true;

            // 2. Проверка повторного использования пароля (Credential Reuse)
            // Пытаемся получить профиль по умолчанию для большинства камер
            let test_url = format!(
                "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=1",
                login, pass, ip
            );
            let test_url_hik = format!(
                "rtsp://{}:{}@{}:554/Streaming/Channels/101",
                login, pass, ip
            );

            let ffmpeg = crate::get_ffmpeg_path(); // Используем вашу функцию

            // Быстрый probe
            for url in [&test_url, &test_url_hik] {
                if let Ok(Ok(status)) = tokio::time::timeout(
                    Duration::from_secs(3),
                    tokio::process::Command::new(&ffmpeg)
                        .args(crate::ffmpeg::FfmpegProfiles::probe(url))
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status(),
                )
                .await
                {
                    if status.success() {
                        result.creds_reused = true;
                        result.rtsp_path = Some(url.to_string());

                        // Сохраняем успех в Базу Знаний!
                        let km = crate::knowledge::KnowledgeManager::new();
                        km.save_success(&ip, "auto_detected", url, &login, &pass);
                        break;
                    }
                }
            }
            result
        });
        tasks.push(task);
    }

    let mut final_results = vec![];
    for task in tasks {
        if let Ok(res) = task.await {
            final_results.push(res);
        }
    }

    let reused_count = final_results.iter().filter(|r| r.creds_reused).count();
    println!(
        "[MASS_AUDIT] ✅ Завершено. Найдено {} устройств с дублирующимся паролем.",
        reused_count
    );

    Ok(final_results)
}
