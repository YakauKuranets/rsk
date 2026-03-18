use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, Read};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareFinding {
    pub category: String,
    pub finding: String,
    pub severity: String,
}

#[tauri::command]
pub fn analyze_firmware(
    file_path: String,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<FirmwareFinding>, String> {
    let file = File::open(&file_path).map_err(|e| format!("Не удалось открыть файл: {}", e))?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();

    reader
        .by_ref()
        .take(20_000_000)
        .read_to_end(&mut buffer)
        .map_err(|e| e.to_string())?;

    let text_content = String::from_utf8_lossy(&buffer);
    let mut findings = Vec::new();

    crate::push_runtime_log(
        &log_state,
        format!(
            "🔬 РЕВЕРС-ИНЖИНИРИНГ: Анализ дампа прошивки [{}]",
            file_path
        ),
    );

    if text_content.contains("BEGIN RSA PRIVATE KEY")
        || text_content.contains("BEGIN OPENSSH PRIVATE KEY")
    {
        findings.push(FirmwareFinding {
            category: "Hardcoded Secrets".to_string(),
            finding: "Обнаружен вшитый в прошивку RSA/SSH приватный ключ!".to_string(),
            severity: "CRITICAL".to_string(),
        });
    }

    if text_content.contains("root:$1$")
        || text_content.contains("admin:$1$")
        || text_content.contains("root:$6$")
        || text_content.contains("admin:$6$")
    {
        findings.push(FirmwareFinding {
            category: "Credentials".to_string(),
            finding: "Найден хэш пароля суперпользователя (возможна утечка /etc/shadow)"
                .to_string(),
            severity: "CRITICAL".to_string(),
        });
    }

    if text_content.contains("telnetd -l /bin/sh") || text_content.contains("dropbear -r") {
        findings.push(FirmwareFinding {
            category: "Backdoor".to_string(),
            finding: "Обнаружены следы запуска telnetd/dropbear с привязкой к root-шеллу (Бэкдор)"
                .to_string(),
            severity: "CRITICAL".to_string(),
        });
    }

    if text_content.contains("BusyBox v1.1") || text_content.contains("OpenSSL 1.0.1") {
        findings.push(FirmwareFinding {
            category: "Outdated Component".to_string(),
            finding: "Вендор использует критически устаревшие библиотеки (BusyBox/OpenSSL)"
                .to_string(),
            severity: "HIGH".to_string(),
        });
    }

    if text_content.contains("123456") || text_content.contains("admin123") {
        findings.push(FirmwareFinding {
            category: "Weak Defaults".to_string(),
            finding: "В бинарнике открытым текстом фигурируют популярные дефолтные пароли"
                .to_string(),
            severity: "MEDIUM".to_string(),
        });
    }

    if findings.is_empty() {
        crate::push_runtime_log(
            &log_state,
            "✅ Анализ завершен. Очевидных секретов в дампе не найдено.",
        );
    } else {
        crate::push_runtime_log(
            &log_state,
            format!(
                "🚨 Найдено {} потенциальных 0-day векторов в прошивке!",
                findings.len()
            ),
        );
    }

    Ok(findings)
}
