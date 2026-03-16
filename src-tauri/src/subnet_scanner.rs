use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;
use tauri::command;
use tokio::net::TcpStream;

/// Асинхронная разведка соседей по подсети /24 (Поиск связанных камер)
#[command]
pub async fn scan_neighborhood(ip: String) -> Result<String, String> {
    let ip_addr = IpAddr::from_str(&ip).map_err(|_| "Неверный формат IP")?;

    // Извлекаем базу сети (первые 3 октета)
    let base_ip = match ip_addr {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            format!("{}.{}.{}.", octets[0], octets[1], octets[2])
        }
        _ => return Ok("Поддерживается только IPv4".into()),
    };

    let mut report = format!(
        "[SUBNET_SCAN] 🗺️ Разведка соседей в подсети {}0/24...\n",
        base_ip
    );
    let mut tasks = vec![];

    // Генерируем 254 асинхронные задачи на проверку RTSP-порта
    for i in 1..=254 {
        let target_ip = format!("{}{}", base_ip, i);
        if target_ip == ip {
            continue;
        } // Пропускаем саму цель

        // Запускаем легкий tokio::spawn для каждого IP
        let task = tokio::spawn(async move {
            let timeout_duration = Duration::from_millis(800); // 800 мс таймаут
            if let Ok(Ok(_)) = tokio::time::timeout(
                timeout_duration,
                TcpStream::connect((&target_ip as &str, 554)),
            )
            .await
            {
                Some(target_ip)
            } else {
                None
            }
        });
        tasks.push(task);
    }

    // Ждем завершения всех микро-сканов
    let mut found = 0;
    for task in tasks {
        if let Ok(Some(live_ip)) = task.await {
            report.push_str(&format!(
                "   📡 Найден соседний узел (RTSP 554 открыт): {}\n",
                live_ip
            ));
            found += 1;
        }
    }

    if found == 0 {
        report.push_str("   🕸️ Соседние камеры не обнаружены (узел изолирован).\n");
    } else {
        report.push_str(&format!(
            "✅ Найдено {} потенциальных целей в той же подсети.\n",
            found
        ));
    }

    Ok(report)
}
