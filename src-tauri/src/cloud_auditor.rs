use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudFinding {
    pub provider: String,
    pub service: String,
    pub resource_id: String,
    pub check_id: String,
    pub severity: String,
    pub description: String,
    pub remediation: String,
}

fn masked_key(key: &str) -> String {
    if key.len() > 4 {
        format!("****{}", &key[key.len() - 4..])
    } else {
        "****".to_string()
    }
}

/// AWS API call with query string auth (simplified SigV4 via pre-signed or session token)
async fn aws_get(
    client: &Client,
    url: &str,
    access_key: &str,
    secret_key: &str,
    region: &str,
    service: &str,
) -> Result<Value, String> {
    let _ = (client, service);
    // Use AWS CLI passthrough for correctness — SigV4 from scratch is error-prone
    let output = tokio::process::Command::new("aws")
        .env("AWS_ACCESS_KEY_ID", access_key)
        .env("AWS_SECRET_ACCESS_KEY", secret_key)
        .env("AWS_DEFAULT_REGION", region)
        .args(url.split_whitespace().collect::<Vec<_>>())
        .args(["--output", "json"])
        .output()
        .await
        .map_err(|e| format!("aws cli: {}", e))?;
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn aws_check_s3(
    access_key: String,
    secret_key: String,
    region: String,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<CloudFinding>, String> {
    crate::push_runtime_log(
        &log_state,
        format!("AWS_S3_CHECK|key={}", masked_key(&access_key)),
    );

    let mut findings = vec![];

    // List buckets via AWS CLI
    let buckets_out = tokio::process::Command::new("aws")
        .env("AWS_ACCESS_KEY_ID", &access_key)
        .env("AWS_SECRET_ACCESS_KEY", &secret_key)
        .env("AWS_DEFAULT_REGION", &region)
        .args(["s3api", "list-buckets", "--output", "json"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let buckets_json: Value = serde_json::from_slice(&buckets_out.stdout).unwrap_or_default();

    let empty = vec![];
    for bucket in buckets_json["Buckets"].as_array().unwrap_or(&empty) {
        let name = bucket["Name"].as_str().unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }

        // Check public access
        let acl_out = tokio::process::Command::new("aws")
            .env("AWS_ACCESS_KEY_ID", &access_key)
            .env("AWS_SECRET_ACCESS_KEY", &secret_key)
            .env("AWS_DEFAULT_REGION", &region)
            .args(["s3api", "get-bucket-acl", "--bucket", &name, "--output", "json"])
            .output()
            .await;
        let acl: Value = match acl_out {
            Ok(output) => serde_json::from_slice(&output.stdout).unwrap_or_default(),
            Err(_) => Value::Null,
        };

        let grants = acl["Grants"].as_array().cloned().unwrap_or_default();
        let public = grants.iter().any(|g| {
            g["Grantee"]["URI"]
                .as_str()
                .map(|u| u.contains("AllUsers") || u.contains("AuthenticatedUsers"))
                .unwrap_or(false)
        });
        if public {
            findings.push(CloudFinding {
                provider: "AWS".to_string(),
                service: "S3".to_string(),
                resource_id: name.clone(),
                check_id: "CIS-2.1".to_string(),
                severity: "CRITICAL".to_string(),
                description: format!("Bucket {} is publicly accessible", name),
                remediation: "Enable S3 Block Public Access. Review and revoke public ACLs."
                    .to_string(),
            });
        }

        // Check encryption
        let enc_out = tokio::process::Command::new("aws")
            .env("AWS_ACCESS_KEY_ID", &access_key)
            .env("AWS_SECRET_ACCESS_KEY", &secret_key)
            .env("AWS_DEFAULT_REGION", &region)
            .args([
                "s3api",
                "get-bucket-encryption",
                "--bucket",
                &name,
                "--output",
                "json",
            ])
            .output()
            .await;
        if !enc_out.map(|output| output.status.success()).unwrap_or(false) {
            findings.push(CloudFinding {
                provider: "AWS".to_string(),
                service: "S3".to_string(),
                resource_id: name,
                check_id: "CIS-2.8".to_string(),
                severity: "HIGH".to_string(),
                description: "Bucket has no server-side encryption".to_string(),
                remediation: "Enable SSE-S3 or SSE-KMS encryption on the bucket.".to_string(),
            });
        }
    }

    Ok(findings)
}

#[tauri::command]
pub async fn aws_check_iam(
    access_key: String,
    secret_key: String,
    region: String,
    log_state: State<'_, crate::LogState>,
) -> Result<Vec<CloudFinding>, String> {
    crate::push_runtime_log(
        &log_state,
        format!("AWS_IAM_CHECK|key={}", masked_key(&access_key)),
    );
    let mut findings = vec![];

    let out = tokio::process::Command::new("aws")
        .env("AWS_ACCESS_KEY_ID", &access_key)
        .env("AWS_SECRET_ACCESS_KEY", &secret_key)
        .env("AWS_DEFAULT_REGION", &region)
        .args(["iam", "list-users", "--output", "json"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let users: Value = serde_json::from_slice(&out.stdout).unwrap_or_default();

    let empty = vec![];
    for user in users["Users"].as_array().unwrap_or(&empty) {
        let username = user["UserName"].as_str().unwrap_or("").to_string();
        // Check MFA
        let mfa_out = tokio::process::Command::new("aws")
            .env("AWS_ACCESS_KEY_ID", &access_key)
            .env("AWS_SECRET_ACCESS_KEY", &secret_key)
            .env("AWS_DEFAULT_REGION", &region)
            .args(["iam", "list-mfa-devices", "--user-name", &username, "--output", "json"])
            .output()
            .await;
        let mfa: Value = match mfa_out {
            Ok(output) => serde_json::from_slice(&output.stdout).unwrap_or_default(),
            Err(_) => Value::Null,
        };
        if mfa["MFADevices"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(true)
        {
            findings.push(CloudFinding {
                provider: "AWS".to_string(),
                service: "IAM".to_string(),
                resource_id: username.clone(),
                check_id: "CIS-1.10".to_string(),
                severity: "HIGH".to_string(),
                description: format!("User {} has no MFA device", username),
                remediation: "Enable MFA for all IAM users with console access.".to_string(),
            });
        }
    }
    Ok(findings)
}
