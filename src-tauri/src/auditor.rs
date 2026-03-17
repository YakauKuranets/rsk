use std::time::Duration;
use tauri::command;
use tokio::time::sleep;

/// Интеллектуальная генерация словаря на основе контекста
fn generate_smart_dict(vendor: &str, osint_context: Option<&str>) -> Vec<(String, String)> {
    let mut dict = Vec::new();

    // 1. Дефолтные креды вендора (самые вероятные)
    match vendor {
        "hikvision" => {
            dict.push(("admin".into(), "12345".into()));
            dict.push(("admin".into(), "123456".into()));
            dict.push(("admin".into(), "admin".into()));
            dict.push(("admin".into(), "123456789abc".into()));
        }
        "xmeye" => {
            dict.push(("admin".into(), "".into())); // Часто пароль пустой
            dict.push(("admin".into(), "admin".into()));
            dict.push(("admin".into(), "123456".into()));
        }
        _ => {
            dict.push(("admin".into(), "12345".into()));
            dict.push(("admin".into(), "admin".into()));
            dict.push(("admin".into(), "".into()));
        }
    }

    // 2. OSINT-мутации (если передали контекст, например "mvd" или "2024")
    if let Some(ctx) = osint_context {
        dict.push(("admin".into(), ctx.to_string()));
        dict.push(("admin".into(), format!("{}123", ctx)));
        dict.push(("admin".into(), format!("{}2024", ctx)));
    }

    dict
}

/// Адаптивный Аудитор Паролей
#[command]
pub async fn adaptive_credential_audit(
    ip: String,
    vendor: String,
    osint_context: Option<String>,
) -> Result<Option<(String, String)>, String> {
    let km = crate::knowledge::KnowledgeManager::new();
    let history = km.load_all();

    // 🧠 ЭТАП 0: Проверяем память. Если мы уже знаем пароль, не шумим!
    if let Some(exp) = history.get(&ip) {
        if !exp.login.is_empty() {
            println!(
                "[AUDITOR] ⚡ Кэш-хит! Известные креды для {}: {} / {}",
                ip, exp.login, exp.pass
            );
            return Ok(Some((exp.login.clone(), exp.pass.clone())));
        }
    }

    let dict = generate_smart_dict(&vendor, osint_context.as_deref());
    let mut base_delay = 500; // Стартовая пауза между попытками: 500мс
    let ffmpeg = crate::get_ffmpeg_path();

    println!(
        "[AUDITOR] 🛡️ Начинаем адаптивный аудит {} (Вендор: {}). Размер словаря: {}",
        ip,
        vendor,
        dict.len()
    );

    // 🕵️‍♂️ ЭТАП 1: Перебор с Backoff-задержками
    for (login, pass) in dict {
        // Формируем легкий путь для проверки (саб-стрим)
        let test_url = if vendor == "hikvision" {
            format!(
                "rtsp://{}:{}@{}:554/Streaming/Channels/102",
                login, pass, ip
            )
        } else {
            format!(
                "rtsp://{}:{}@{}:554/cam/realmonitor?channel=1&subtype=1",
                login, pass, ip
            )
        };

        let start_time = std::time::Instant::now();

        // Быстрый отстрел через FFmpeg зонд
        let s = std::process::Command::new(&ffmpeg)
            .args(crate::ffmpeg::FfmpegProfiles::probe(&test_url))
            .status();

        let rtt = start_time.elapsed().as_millis();

        if let Ok(status) = s {
            if status.success() {
                println!(
                    "[AUDITOR] 🎯 УСПЕХ! Подобраны креды для {}: {} / {}",
                    ip, login, pass
                );
                // Сохраняем в базу знаний
                km.save_success(&ip, &vendor, &test_url, &login, &pass);

                // 🛡️ НОВОЕ: Отправляем найденный пароль на проверку утечек
                if !pass.is_empty() {
                    match crate::breach_analyzer::check_password_breach(pass.clone()).await {
                        Ok(report) => println!("[BREACH_DATA] {}", report),
                        Err(e) => println!("[BREACH_DATA] Ошибка проверки: {}", e),
                    }
                }

                return Ok(Some((login, pass)));
            }
        }

        // 🛡️ Адаптивная логика (Circuit Breaker & Evasion):
        // Если камера отвечает слишком долго (RTT > 2000мс), она захлебывается.
        // Увеличиваем паузу, чтобы не вызвать DoS.
        if rtt > 2000 {
            base_delay += 1000;
            println!(
                "[AUDITOR] ⚠️ Камера {} тормозит (RTT {}ms). Увеличиваем паузу до {}ms",
                ip, rtt, base_delay
            );
        } else {
            // Легкая рандомизация (Jitter), чтобы сбить с толку анализаторы трафика
            base_delay = 500 + (rtt as u64 % 300);
        }

        println!(
            "[AUDITOR] Неудача ({} / {}). Этический лимит 300ms + adaptive {}ms...",
            login, pass, base_delay
        );

        // Safe-Rate Guardrail: минимальная пауза между попытками брутфорса.
        sleep(Duration::from_millis(300)).await;

        // Сохраняем адаптивный backoff поверх безопасного минимума.
        if base_delay > 300 {
            sleep(Duration::from_millis(base_delay - 300)).await;
        }
    }

    println!("[AUDITOR] 🛑 Аудит завершен. Подходящих кредов в базовом словаре не найдено.");
    Ok(None)
}
