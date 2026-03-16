use reqwest::Client;
use std::time::{Duration, Instant};

/// Интеллектуальный предохранитель (Circuit Breaker)
struct CircuitBreaker {
    max_rtt_ms: u128,
    max_errors: u8,
    current_errors: u8,
}

impl CircuitBreaker {
    fn new() -> Self {
        Self {
            max_rtt_ms: 1500,
            max_errors: 2,
            current_errors: 0,
        }
    }

    fn register_response(&mut self, rtt: u128, status_is_5xx: bool) -> Result<(), &'static str> {
        if status_is_5xx {
            self.current_errors += 1;
        }
        if self.current_errors >= self.max_errors {
            return Err("Слишком много 5xx ошибок");
        }
        if rtt > self.max_rtt_ms {
            return Err("Критическая задержка ответа (RTT)");
        }
        Ok(())
    }
}

/// Безопасный мутационный фаззинг API
#[tauri::command]
pub async fn run_api_fuzzer(ip: String) -> Result<String, String> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(2)) // Жесткий таймаут!
        .build()
        .map_err(|e| e.to_string())?;

    // 1. Быстрый поиск альтернативных веб-портов
    let common_ports = [80, 81, 8080, 8000, 8443];
    let mut target_port = 0;

    for port in common_ports {
        let url = format!("http://{}:{}", ip, port);
        if client.get(&url).send().await.is_ok() {
            target_port = port;
            break;
        }
    }

    if target_port == 0 {
        return Ok(format!(
            "[-] Фаззинг отменен: Веб-интерфейс не найден ни на одном из портов ({:?})",
            common_ports
        ));
    }

    let mut report = format!(
        "[API_FUZZER] 🎯 Найден веб-сервер на порту {}. Начинаем безопасный фаззинг...\n",
        target_port
    );
    let mut breaker = CircuitBreaker::new();

    // 2. Словарь скрытых эндпоинтов для IoT
    let endpoints = [
        "/cgi-bin/snapshot.cgi", // Снимок без пароля
        "/config/get",           // Утечка конфига
        "/onvif/device_service", // Открытый ONVIF
        "/system/deviceInfo",    // Данные о прошивке
    ];

    for ep in endpoints {
        let url = format!("http://{}:{}{}", ip, target_port, ep);
        let start = Instant::now();

        let response = client.get(&url).send().await;
        let rtt = start.elapsed().as_millis();

        match response {
            Ok(resp) => {
                let status = resp.status();

                // Проверка предохранителя
                if let Err(reason) = breaker.register_response(rtt, status.is_server_error()) {
                    report.push_str(&format!(
                        "🛑 ПРЕДОХРАНИТЕЛЬ СРАБОТАЛ: {}. Фаззинг прерван для защиты устройства!\n",
                        reason
                    ));
                    break;
                }

                if status.is_success() && resp.content_length().unwrap_or(0) > 0 {
                    report.push_str(&format!(
                        "🚨 НАЙДЕН ОТКРЫТЫЙ ЭНДПОИНТ: {} (HTTP {})\n",
                        ep, status
                    ));
                }
            }
            Err(_) => {
                // Если соединение рвется, лучше остановиться
                report.push_str("⚠️ Ошибка соединения. Прерываем фаззинг.\n");
                break;
            }
        }
        // Пауза между запросами (Имитация легитимного трафика)
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    Ok(report)
}
