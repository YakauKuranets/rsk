use serde::Serialize;
use tauri::State;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceMapping {
    pub standard: String,
    pub requirement_id: String,
    pub requirement_text: String,
    pub status: String,
    pub related_findings: Vec<String>,
}

#[tauri::command]
pub async fn check_compliance(
    findings_json: String,
    standards: Vec<String>,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<ComplianceMapping>, String> {
    crate::push_runtime_log(&log_state, "[COMPLIANCE] start mapping".to_string());

    let findings: Vec<serde_json::Value> =
        serde_json::from_str(&findings_json).map_err(|e| e.to_string())?;
    let standard_set: std::collections::HashSet<String> =
        standards.into_iter().map(|s| s.to_uppercase()).collect();

    let mut out = Vec::new();

    push_if_enabled(
        &standard_set,
        "PCI_DSS",
        build_mapping(
            "PCI_DSS",
            "2.1",
            "Изменение дефолтных паролей и параметров безопасности",
            collect_ids(&findings, |f| {
                contains_any(f, &["default password", "weak/default", "credential"])
            }),
        ),
        &mut out,
    );
    push_if_enabled(
        &standard_set,
        "ISO_27001",
        build_mapping(
            "ISO_27001",
            "A.9.2.4",
            "Управление секретной аутентификационной информацией пользователей",
            collect_ids(&findings, |f| {
                contains_any(f, &["default password", "credential", "weak"])
            }),
        ),
        &mut out,
    );
    push_if_enabled(
        &standard_set,
        "PCI_DSS",
        build_mapping(
            "PCI_DSS",
            "6.2",
            "Обеспечение своевременной установки патчей безопасности",
            collect_ids(&findings, |f| {
                contains_any(f, &["outdated firmware", "firmware", "cve"])
            }),
        ),
        &mut out,
    );
    push_if_enabled(
        &standard_set,
        "ISO_27001",
        build_mapping(
            "ISO_27001",
            "A.12.6.1",
            "Управление техническими уязвимостями",
            collect_ids(&findings, |f| {
                contains_any(f, &["outdated firmware", "cve", "vulnerability"])
            }),
        ),
        &mut out,
    );
    push_if_enabled(
        &standard_set,
        "PCI_DSS",
        build_mapping(
            "PCI_DSS",
            "4.1",
            "Шифрование передачи данных по открытым сетям",
            collect_ids(&findings, |f| {
                contains_any(f, &["unencrypted", "http_basic", "telnet", "ftp"])
            }),
        ),
        &mut out,
    );
    push_if_enabled(
        &standard_set,
        "GDPR",
        build_mapping(
            "GDPR",
            "Art.32",
            "Безопасность обработки персональных данных",
            collect_ids(&findings, |f| {
                contains_any(f, &["unencrypted", "breach", "exposed"])
            }),
        ),
        &mut out,
    );
    push_if_enabled(
        &standard_set,
        "PCI_DSS",
        build_mapping(
            "PCI_DSS",
            "2.2.5",
            "Отключение небезопасных служб и протоколов",
            collect_ids(&findings, |f| {
                contains_any(f, &["open ftp", "ftp", "port 21"])
            }),
        ),
        &mut out,
    );

    sleep(Duration::from_millis(120)).await;
    crate::push_runtime_log(
        &log_state,
        format!("[COMPLIANCE] mapped {} controls", out.len()),
    );
    Ok(out)
}

fn collect_ids(
    findings: &[serde_json::Value],
    pred: impl Fn(&serde_json::Value) -> bool,
) -> Vec<String> {
    findings
        .iter()
        .filter(|f| pred(f))
        .enumerate()
        .map(|(idx, f)| {
            f["id"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("finding_{}", idx + 1))
        })
        .collect()
}

fn contains_any(f: &serde_json::Value, needles: &[&str]) -> bool {
    let text = serde_json::to_string(f).unwrap_or_default().to_lowercase();
    needles.iter().any(|n| text.contains(&n.to_lowercase()))
}

fn build_mapping(
    standard: &str,
    requirement_id: &str,
    requirement_text: &str,
    related_findings: Vec<String>,
) -> ComplianceMapping {
    let status = if related_findings.is_empty() {
        "compliant"
    } else if related_findings.len() > 2 {
        "non_compliant"
    } else {
        "partially"
    };

    ComplianceMapping {
        standard: standard.to_string(),
        requirement_id: requirement_id.to_string(),
        requirement_text: requirement_text.to_string(),
        status: status.to_string(),
        related_findings,
    }
}

fn push_if_enabled(
    set: &std::collections::HashSet<String>,
    standard: &str,
    mapping: ComplianceMapping,
    out: &mut Vec<ComplianceMapping>,
) {
    if set.is_empty() || set.contains(&standard.to_uppercase()) {
        out.push(mapping);
    }
}
