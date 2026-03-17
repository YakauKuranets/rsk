use std::time::Duration;
use tauri::command;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Пассивная проверка портов административного доступа (SSH / Telnet)
#[command]
pub async fn assess_persistence_risk(ip: String) -> Result<String, String> {
    let mut report = format!(
        "[PERSISTENCE_AUDIT] 🔍 Оценка риска закрепления для {}...\n",
        ip
    );
    let mut risk_found = false;

    let ports_to_check = [
        (22, "SSH (Secure Shell)"),
        (23, "Telnet (Незащищенный терминал)"),
    ];

    let check_timeout = Duration::from_millis(1500);

    for (port, service) in ports_to_check {
        let addr = format!("{}:{}", ip, port);

        // Пытаемся установить TCP-соединение
        if let Ok(Ok(mut _stream)) = timeout(check_timeout, TcpStream::connect(&addr)).await {
            report.push_str(&format!(
                "🚨 КРИТИЧЕСКИЙ РИСК: Открыт порт {} ({}).\n",
                port, service
            ));

            if port == 23 {
                report.push_str(
                    "   ⚠️ Telnet передает данные в открытом виде. Крайне высокий риск перехвата!\n",
                );
            }

            report.push_str("   ⚠️ Злоумышленник может использовать найденные RTSP-пароли для получения shell-доступа (RCE) и закрепления в системе (например, через crontab).\n");
            risk_found = true;
        }
    }

    if !risk_found {
        report.push_str(
            "✅ Риск закрепления минимален: порты удаленного управления (SSH/Telnet) закрыты извне.\n",
        );
    }

    Ok(report)
}
