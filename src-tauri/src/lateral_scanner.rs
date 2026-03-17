use std::time::Duration;

pub async fn check_neighbors(target_ip: &str, known_creds: Vec<String>) -> Result<Option<String>, String> {
    // Guardrail 1: Запрет на сканирование публичных IP (только локальные сети для безопасности)
    if !target_url_is_local(target_ip) {
        return Ok(Some(String::from("Пропуск: Боковое перемещение разрешено только в локальных сетях (RFC 1918)")));
    }

    // Симуляция генерации соседних IP (например, если target 192.168.1.5, соседи .4 и .6)
    let neighbors = generate_close_neighbors(target_ip);
    let mut successful_lateral = Vec::new();

    for neighbor in neighbors {
        // Guardrail 2: Этическая задержка между хостами
        tokio::time::sleep(Duration::from_millis(600)).await;

        for cred in &known_creds {
            // Симуляция попытки входа на соседний хост с известным паролем
            // В реальном коде здесь будет вызов SSH/FTP клиента
            println!("[LateralScanner] 🕷️ Пробую креды '{}' на соседе {}...", cred, neighbor);

            // Имитируем успешный подбор с вероятностью 10% (для тестов)
            if neighbor.ends_with(".1") && cred.contains("admin") {
                successful_lateral.push(format!("Успешный вход на {} с кредами [{}]", neighbor, cred));
            }
        }
    }

    if successful_lateral.is_empty() {
        Ok(None)
    } else {
        Ok(Some(successful_lateral.join(" | ")))
    }
}

// Вспомогательная функция проверки локальной сети (Guardrail)
fn target_url_is_local(ip: &str) -> bool {
    ip.starts_with("192.168.") || ip.starts_with("10.") || ip.starts_with("172.16.")
}

// Вспомогательная функция для генерации +/- 2 IP адресов
fn generate_close_neighbors(ip: &str) -> Vec<String> {
    let mut neighbors = Vec::new();
    if let Some(last_octet_idx) = ip.rfind('.') {
        let prefix = &ip[..last_octet_idx];
        if let Ok(last_octet) = ip[last_octet_idx + 1..].parse::<u8>() {
            if last_octet > 1 {
                neighbors.push(format!("{}.{}", prefix, last_octet - 1));
            }
            if last_octet < 254 {
                neighbors.push(format!("{}.{}", prefix, last_octet + 1));
            }
        }
    }
    neighbors
}
