use super::parser::{PlaybookScope, PlaybookStep};
use ipnetwork::IpNetwork;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::IpAddr;
use tokio::time::{timeout, Duration};

fn substitute_string(
    src: &str,
    variables: &HashMap<String, String>,
    previous_outputs: &HashMap<String, Value>,
) -> String {
    let mut out = src.to_string();
    for (k, v) in variables {
        out = out.replace(&format!("${{{}}}", k), v);
    }

    // Simple `${step.field}` / `${step.path.to.value}`
    let re = regex::Regex::new(r"\$\{([a-zA-Z0-9_]+)\.([a-zA-Z0-9_\.\[\]]+)\}").unwrap();
    re.replace_all(&out, |caps: &regex::Captures| {
        let step = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        let path = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
        let Some(val) = previous_outputs.get(step) else {
            return "".to_string();
        };
        let mut cur = val;
        for part in path.split('.') {
            if let Some((key, idx)) = part.split_once('[') {
                let idx = idx.trim_end_matches(']').parse::<usize>().unwrap_or(0);
                cur = cur
                    .get(key)
                    .and_then(|x| x.get(idx))
                    .unwrap_or(&Value::Null);
            } else {
                cur = cur.get(part).unwrap_or(&Value::Null);
            }
        }
        if let Some(s) = cur.as_str() {
            s.to_string()
        } else {
            cur.to_string()
        }
    })
    .to_string()
}

fn yaml_to_json_with_subst(
    value: &serde_yaml::Value,
    variables: &HashMap<String, String>,
    previous_outputs: &HashMap<String, Value>,
) -> Value {
    let mut v = serde_json::to_value(value).unwrap_or(Value::Null);
    fn walk(v: &mut Value, vars: &HashMap<String, String>, prev: &HashMap<String, Value>) {
        match v {
            Value::String(s) => *s = substitute_string(s, vars, prev),
            Value::Array(arr) => arr.iter_mut().for_each(|x| walk(x, vars, prev)),
            Value::Object(map) => map.values_mut().for_each(|x| walk(x, vars, prev)),
            _ => {}
        }
    }
    walk(&mut v, variables, previous_outputs);
    v
}

fn ip_allowed(target: &str, scope: &PlaybookScope) -> Result<(), String> {
    let ip: IpAddr = target
        .parse()
        .map_err(|_| format!("Invalid target IP '{}'", target))?;

    let mut included = false;
    for rule in &scope.targets {
        if let Ok(net) = rule.parse::<IpNetwork>() {
            if net.contains(ip) {
                included = true;
                break;
            }
        } else if rule == target {
            included = true;
            break;
        }
    }
    if !included {
        return Err(format!("Target '{}' is outside scope.targets", target));
    }

    for rule in &scope.excluded {
        if let Ok(net) = rule.parse::<IpNetwork>() {
            if net.contains(ip) {
                return Err(format!("Target '{}' is in scope.excluded", target));
            }
        } else if rule == target {
            return Err(format!("Target '{}' is in scope.excluded", target));
        }
    }

    Ok(())
}

pub async fn execute_step(
    step: &PlaybookStep,
    scope: &PlaybookScope,
    variables: &HashMap<String, String>,
    previous_outputs: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let params_yaml = serde_yaml::to_value(&step.params).map_err(|e| e.to_string())?;
    let params = yaml_to_json_with_subst(&params_yaml, variables, previous_outputs);

    let target = params
        .get("target")
        .and_then(|v| v.as_str())
        .or_else(|| params.get("host").and_then(|v| v.as_str()))
        .or_else(|| params.get("ip").and_then(|v| v.as_str()))
        .or_else(|| params.get("camera_ip").and_then(|v| v.as_str()));

    if let Some(t) = target {
        ip_allowed(t, scope)?;
    }

    let timeout_secs = step.timeout_secs.unwrap_or(120);
    timeout(Duration::from_secs(timeout_secs), async move {
        let output = match step.module.as_str() {
            "camera_scan" => json!({"module":"camera_scan","called":"camera_discovery::unified_camera_scan","params":params}),
            "port_scan" => json!({"module":"port_scan","called":"system_cmds::scan_host_ports","params":params}),
            "credential_audit" => json!({"module":"credential_audit","called":"credential_auditor::advanced_credential_audit","params":params}),
            "vuln_scan" => json!({"module":"vuln_scan","called":"vuln_verifier::verify_vulnerability","params":params}),
            "spider" => json!({"module":"spider","called":"spider::spider_full_scan","params":params}),
            "archive_search" => json!({"module":"archive_search","called":"unified_archive::search_archive_unified","params":params}),
            "security_headers" => json!({"module":"security_headers","called":"analyze_security_headers","params":params}),
            "metadata_collect" => {
                let ip = params.get("ip").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let real = crate::metadata_extractor::collect_metadata(ip).await?;
                serde_json::to_value(real).map_err(|e| e.to_string())?
            }
            "mass_audit" => json!({"module":"mass_audit","called":"mass_auditor::run_mass_audit","params":params}),
            "compliance_check" => json!({"module":"compliance_check","called":"compliance_checker::check_compliance","params":params}),
            "asset_discovery" => json!({"module":"asset_discovery","called":"asset_discovery::discover_external_assets","params":params}),
            "report_generate" => json!({"module":"report_generate","called":"report_export::export_report_json","params":params}),
            "archive_download" => json!({"module":"archive_download","called":"unified_archive::download_archive_unified","params":params}),
            other => return Err(format!("Unsupported module: {other}")),
        };
        Ok(output)
    })
    .await
    .map_err(|_| format!("Step '{}' timed out", step.id))?
}
