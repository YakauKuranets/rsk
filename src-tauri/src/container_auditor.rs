use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: Vec<String>,
    pub privileged: bool,
    pub user: String,
    pub read_only_root: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerFinding {
    pub container_id: String,
    pub check_id: String,
    pub severity: String,
    pub description: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerAuditReport {
    pub containers: Vec<ContainerInfo>,
    pub findings: Vec<ContainerFinding>,
    pub docker_socket_exposed: bool,
    pub privileged_count: usize,
}

fn run(prog: &str, args: &[&str]) -> String {
    Command::new(prog)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

#[tauri::command]
pub fn audit_containers(log_state: State<'_, crate::LogState>) -> Result<ContainerAuditReport, String> {
    crate::push_runtime_log(&log_state, "CONTAINER_AUDIT|start".to_string());

    // docker ps --format json
    let ps_out = run("docker", &["ps", "--format", "{{json .}}"]);
    let mut containers = vec![];
    let mut findings = vec![];

    for line in ps_out.lines() {
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let id = v["ID"].as_str().unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }

        // docker inspect for security details
        let inspect = run(
            "docker",
            &[
                "inspect",
                "--format",
                "{{.HostConfig.Privileged}}|{{.Config.User}}|{{.HostConfig.ReadonlyRootfs}}",
                &id,
            ],
        );
        let parts: Vec<&str> = inspect.trim().splitn(3, '|').collect();
        let privileged = parts.first().map(|s| *s == "true").unwrap_or(false);
        let user = parts.get(1).unwrap_or(&"").trim().to_string();
        let read_only = parts.get(2).map(|s| s.trim() == "true").unwrap_or(false);

        // CIS Benchmark checks
        if privileged {
            findings.push(ContainerFinding {
                container_id: id.clone(),
                check_id: "CIS-5.4".to_string(),
                severity: "CRITICAL".to_string(),
                description: format!("Container {} runs in privileged mode", id),
                remediation: "Remove --privileged flag. Use specific capabilities instead."
                    .to_string(),
            });
        }
        if user.is_empty() || user == "root" || user == "0" {
            findings.push(ContainerFinding {
                container_id: id.clone(),
                check_id: "CIS-4.1".to_string(),
                severity: "HIGH".to_string(),
                description: format!("Container {} runs as root", id),
                remediation: "Add USER directive in Dockerfile. Use non-root UID >= 1000."
                    .to_string(),
            });
        }
        if !read_only {
            findings.push(ContainerFinding {
                container_id: id.clone(),
                check_id: "CIS-5.12".to_string(),
                severity: "MEDIUM".to_string(),
                description: format!("Container {} root filesystem is writable", id),
                remediation: "Add --read-only flag. Mount tmpfs for /tmp if needed.".to_string(),
            });
        }

        // Check for secrets in env
        let env_out = run(
            "docker",
            &[
                "inspect",
                "--format",
                "{{range .Config.Env}}{{println .}}{{end}}",
                &id,
            ],
        );
        let secret_keys = [
            "password",
            "passwd",
            "secret",
            "token",
            "key",
            "api_key",
            "aws_secret",
        ];
        for env_line in env_out.lines() {
            let lower = env_line.to_lowercase();
            if secret_keys
                .iter()
                .any(|k| lower.starts_with(k) || lower.contains(&format!("_{}", k)))
            {
                let masked = env_line.split('=').next().unwrap_or("").to_string() + "=***REDACTED***";
                findings.push(ContainerFinding {
                    container_id: id.clone(),
                    check_id: "CIS-ENV-SECRET".to_string(),
                    severity: "HIGH".to_string(),
                    description: format!("Secret in environment: {}", masked),
                    remediation:
                        "Use Docker secrets or external vault. Never hardcode credentials."
                            .to_string(),
                });
            }
        }

        let ports_str = v["Ports"].as_str().unwrap_or("").to_string();
        containers.push(ContainerInfo {
            id,
            name: v["Names"].as_str().unwrap_or("").to_string(),
            image: v["Image"].as_str().unwrap_or("").to_string(),
            status: v["Status"].as_str().unwrap_or("").to_string(),
            ports: ports_str.split(',').map(|s| s.trim().to_string()).collect(),
            privileged,
            user,
            read_only_root: read_only,
        });
    }

    // Check Docker socket exposure
    let socket_exposed = std::path::Path::new("/var/run/docker.sock").exists();

    let privileged_count = containers.iter().filter(|c| c.privileged).count();
    crate::push_runtime_log(
        &log_state,
        format!(
            "CONTAINER_AUDIT_DONE|containers={}|findings={}",
            containers.len(),
            findings.len()
        ),
    );

    Ok(ContainerAuditReport {
        containers,
        findings,
        docker_socket_exposed: socket_exposed,
        privileged_count,
    })
}
