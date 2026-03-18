// src-tauri/src/telegram_osint.rs
// OSINT via Telegram public channels and groups
// Uses Telegram Bot API (no MTProto — no phone number needed)
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramMessage {
    pub message_id: i64,
    pub text: String,
    pub date: i64,
    pub channel: String,
    pub relevance_score: f32,
    pub matched_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramOsintReport {
    pub query: String,
    pub messages_found: usize,
    pub messages: Vec<TelegramMessage>,
    pub threat_indicators: Vec<String>,
}

fn compute_relevance(text: &str, keywords: &[&str]) -> (f32, Vec<String>) {
    let text_low = text.to_lowercase();
    let matched: Vec<String> = keywords
        .iter()
        .filter(|&&kw| text_low.contains(kw))
        .map(|&kw| kw.to_string())
        .collect();
    let score = if keywords.is_empty() {
        0.0
    } else {
        matched.len() as f32 / keywords.len() as f32
    };
    (score, matched)
}

#[tauri::command]
pub async fn search_telegram_osint(
    vendor: String,
    model: Option<String>,
    bot_token: String,
    channel_usernames: Vec<String>,
    log_state: State<'_, crate::LogState>,
) -> Result<TelegramOsintReport, String> {
    if bot_token.trim().len() < 10 {
        return Err("Invalid Telegram Bot token".to_string());
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    // Build search keywords from vendor/model
    let vendor_low = vendor.to_lowercase();
    let mut keywords: Vec<String> = vec![
        "exploit".into(),
        "cve".into(),
        "vulnerability".into(),
        "0day".into(),
        "rce".into(),
        "bypass".into(),
        "backdoor".into(),
        "credentials".into(),
        "default password".into(),
        vendor_low.clone(),
    ];
    if let Some(m) = &model {
        keywords.push(m.to_lowercase());
    }
    let keyword_refs: Vec<&str> = keywords.iter().map(String::as_str).collect();

    let query = match &model {
        Some(m) => format!("{} {}", vendor, m),
        None => vendor.clone(),
    };

    crate::push_runtime_log(
        &log_state,
        format!("TELEGRAM_OSINT|vendor={}|channels={}", vendor, channel_usernames.len()),
    );

    let mut all_messages = Vec::new();

    for channel in &channel_usernames {
        // Telegram web preview for public channels
        let preview_url = format!("https://t.me/s/{}", channel.trim_start_matches('@'));

        let Ok(Ok(resp)) = tokio::time::timeout(
            Duration::from_secs(10),
            client
                .get(&preview_url)
                .header("User-Agent", "Mozilla/5.0 Hyperion/1.0")
                .send(),
        )
        .await
        else {
            continue;
        };

        let Ok(Ok(html)) = tokio::time::timeout(Duration::from_secs(5), resp.text()).await else {
            continue;
        };

        // Parse Telegram web preview messages
        for msg_block in html.split("tgme_widget_message_text") {
            if msg_block.len() < 50 {
                continue;
            }
            let text = strip_html_tags(msg_block.split("</div>").next().unwrap_or(""));
            if text.is_empty() {
                continue;
            }

            let text_low = text.to_lowercase();
            // Must mention vendor
            if !text_low.contains(&vendor_low) {
                continue;
            }

            let (score, matched) = compute_relevance(&text, &keyword_refs);
            if score < 0.1 {
                continue;
            }

            all_messages.push(TelegramMessage {
                message_id: all_messages.len() as i64,
                text: text.chars().take(500).collect(),
                date: 0,
                channel: channel.clone(),
                relevance_score: score,
                matched_keywords: matched,
            });
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Sort by relevance
    all_messages.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_messages.truncate(20); // top 20

    // Extract threat indicators
    let mut indicators = Vec::new();
    for msg in &all_messages {
        if msg
            .matched_keywords
            .iter()
            .any(|k| k == "0day" || k == "exploit")
        {
            indicators.push(format!("Potential 0day/exploit discussed in @{}", msg.channel));
        }
        if msg
            .matched_keywords
            .iter()
            .any(|k| k == "credentials" || k == "default password")
        {
            indicators.push(format!("Credential discussion in @{}", msg.channel));
        }
    }
    indicators.dedup();

    crate::push_runtime_log(
        &log_state,
        format!(
            "TELEGRAM_OSINT_DONE|found={}|indicators={}",
            all_messages.len(),
            indicators.len()
        ),
    );

    Ok(TelegramOsintReport {
        query,
        messages_found: all_messages.len(),
        messages: all_messages,
        threat_indicators: indicators,
    })
}

fn strip_html_tags(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result.trim().replace("  ", " ").replace("\n\n", "\n")
}
