use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignProject {
    pub id: String,
    pub name: String,
    pub client_name: String,
    pub description: Option<String>,
    pub scope: CampaignScope,
    pub status: CampaignStatus,
    pub created_at: String,
    pub updated_at: String,
    pub members: Vec<TeamMember>,
    pub findings: Vec<CampaignFinding>,
    pub timeline: Vec<TimelineEvent>,
    pub notes: Vec<CampaignNote>,
    pub playbook_results: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignScope {
    pub targets: Vec<String>,
    pub excluded: Vec<String>,
    pub mode: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CampaignStatus { Planning, Active, Paused, Completed, Archived }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMember { pub name: String, pub role: String, pub assigned_targets: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignFinding {
    pub id: String,
    pub target_ip: String,
    pub title: String,
    pub severity: String,
    pub category: String,
    pub description: String,
    pub evidence: String,
    pub recommendation: String,
    pub cve_ids: Vec<String>,
    pub cvss_score: Option<f32>,
    pub status: String,
    pub found_by: String,
    pub found_at: String,
    pub module: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent { pub timestamp: String, pub event_type: String, pub description: String, pub actor: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignNote { pub id: String, pub author: String, pub text: String, pub timestamp: String, pub tags: Vec<String> }

fn open_tree() -> Result<sled::Tree, String> {
    let db = sled::open(crate::get_vault_path().join("campaigns_db")).map_err(|e| e.to_string())?;
    db.open_tree("campaigns").map_err(|e| e.to_string())
}

fn save_campaign(c: &CampaignProject) -> Result<(), String> {
    let tree = open_tree()?;
    tree.insert(c.id.as_bytes(), serde_json::to_vec(c).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    tree.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn load_campaign(campaign_id: &str) -> Result<CampaignProject, String> {
    let tree = open_tree()?;
    let raw = tree.get(campaign_id.as_bytes()).map_err(|e| e.to_string())?.ok_or("Campaign not found")?;
    serde_json::from_slice(&raw).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_campaign(name: String, client_name: String, scope_json: String) -> Result<CampaignProject, String> {
    let scope: CampaignScope = serde_json::from_str(&scope_json).map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let id = format!("camp_{}", Utc::now().timestamp_millis());
    let mut c = CampaignProject {
        id,
        name,
        client_name,
        description: None,
        scope,
        status: CampaignStatus::Planning,
        created_at: now.clone(),
        updated_at: now.clone(),
        members: vec![],
        findings: vec![],
        timeline: vec![],
        notes: vec![],
        playbook_results: vec![],
    };
    c.timeline.push(TimelineEvent { timestamp: now, event_type: "campaign_created".into(), description: "Campaign created".into(), actor: "system".into() });
    save_campaign(&c)?;
    Ok(c)
}

#[tauri::command]
pub fn list_campaigns() -> Result<Vec<CampaignProject>, String> {
    let tree = open_tree()?;
    let mut out = vec![];
    for item in tree.iter() {
        let (_, v) = item.map_err(|e| e.to_string())?;
        let c: CampaignProject = serde_json::from_slice(&v).map_err(|e| e.to_string())?;
        out.push(c);
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

#[tauri::command]
pub fn get_campaign(campaign_id: String) -> Result<CampaignProject, String> { load_campaign(&campaign_id) }

#[tauri::command]
pub fn update_campaign_status(campaign_id: String, new_status: String) -> Result<CampaignProject, String> {
    let mut c = load_campaign(&campaign_id)?;
    c.status = match new_status.to_lowercase().as_str() {
        "active" => CampaignStatus::Active,
        "paused" => CampaignStatus::Paused,
        "completed" => CampaignStatus::Completed,
        "archived" => CampaignStatus::Archived,
        _ => CampaignStatus::Planning,
    };
    c.updated_at = Utc::now().to_rfc3339();
    c.timeline.push(TimelineEvent { timestamp: c.updated_at.clone(), event_type: "status_changed".into(), description: format!("Status -> {}", new_status), actor: "operator".into() });
    save_campaign(&c)?;
    Ok(c)
}

#[tauri::command]
pub fn add_campaign_finding(campaign_id: String, finding_json: String) -> Result<CampaignFinding, String> {
    let mut c = load_campaign(&campaign_id)?;
    let v: serde_json::Value = serde_json::from_str(&finding_json).map_err(|e| e.to_string())?;
    let finding = CampaignFinding {
        id: format!("finding_{}", Utc::now().timestamp_millis()),
        target_ip: v["target_ip"].as_str().unwrap_or("unknown").to_string(),
        title: v["title"].as_str().unwrap_or("Untitled").to_string(),
        severity: v["severity"].as_str().unwrap_or("info").to_string(),
        category: v["category"].as_str().unwrap_or("general").to_string(),
        description: v["description"].as_str().unwrap_or("").to_string(),
        evidence: v["evidence"].as_str().unwrap_or("").to_string(),
        recommendation: v["recommendation"].as_str().unwrap_or("").to_string(),
        cve_ids: v["cve_ids"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default(),
        cvss_score: v["cvss_score"].as_f64().map(|x| x as f32),
        status: "open".into(),
        found_by: v["found_by"].as_str().unwrap_or("operator").to_string(),
        found_at: Utc::now().to_rfc3339(),
        module: v["module"].as_str().unwrap_or("manual").to_string(),
    };
    c.findings.push(finding.clone());
    c.updated_at = Utc::now().to_rfc3339();
    c.timeline.push(TimelineEvent { timestamp: c.updated_at.clone(), event_type: "finding_added".into(), description: format!("{} ({})", finding.title, finding.severity), actor: finding.found_by.clone() });
    save_campaign(&c)?;
    Ok(finding)
}

#[tauri::command]
pub fn update_finding_status(campaign_id: String, finding_id: String, new_status: String) -> Result<(), String> {
    let mut c = load_campaign(&campaign_id)?;
    if let Some(f) = c.findings.iter_mut().find(|f| f.id == finding_id) {
        f.status = new_status;
    }
    c.updated_at = Utc::now().to_rfc3339();
    save_campaign(&c)
}

#[tauri::command]
pub fn add_campaign_note(campaign_id: String, author: String, text: String, tags: Vec<String>) -> Result<CampaignNote, String> {
    let mut c = load_campaign(&campaign_id)?;
    let note = CampaignNote { id: format!("note_{}", Utc::now().timestamp_millis()), author: author.clone(), text, timestamp: Utc::now().to_rfc3339(), tags };
    c.notes.push(note.clone());
    c.updated_at = Utc::now().to_rfc3339();
    c.timeline.push(TimelineEvent { timestamp: c.updated_at.clone(), event_type: "note_added".into(), description: "Note added".into(), actor: author });
    save_campaign(&c)?;
    Ok(note)
}

#[tauri::command]
pub fn add_timeline_event(campaign_id: String, event_type: String, description: String, actor: String) -> Result<(), String> {
    let mut c = load_campaign(&campaign_id)?;
    c.timeline.push(TimelineEvent { timestamp: Utc::now().to_rfc3339(), event_type, description, actor });
    c.updated_at = Utc::now().to_rfc3339();
    save_campaign(&c)
}

#[tauri::command]
pub fn import_scan_results(campaign_id: String, module_name: String, results_json: String) -> Result<u32, String> {
    let mut c = load_campaign(&campaign_id)?;
    let parsed: serde_json::Value = serde_json::from_str(&results_json).unwrap_or(json!([]));
    let mut count = 0u32;
    let arr = parsed.as_array().cloned().unwrap_or_else(|| vec![parsed]);
    for item in arr {
        let finding = CampaignFinding {
            id: format!("finding_{}_{}", Utc::now().timestamp_millis(), count),
            target_ip: item.get("ip").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            title: item.get("title").and_then(|v| v.as_str()).unwrap_or("Imported finding").to_string(),
            severity: item.get("severity").and_then(|v| v.as_str()).unwrap_or("info").to_string(),
            category: item.get("category").and_then(|v| v.as_str()).unwrap_or("import").to_string(),
            description: item.to_string(),
            evidence: "Imported from module result".into(),
            recommendation: "Review manually".into(),
            cve_ids: vec![],
            cvss_score: None,
            status: "open".into(),
            found_by: "system".into(),
            found_at: Utc::now().to_rfc3339(),
            module: module_name.clone(),
        };
        c.findings.push(finding);
        count += 1;
    }
    c.timeline.push(TimelineEvent { timestamp: Utc::now().to_rfc3339(), event_type: "playbook_run".into(), description: format!("Imported {} findings from {}", count, module_name), actor: "system".into() });
    c.updated_at = Utc::now().to_rfc3339();
    save_campaign(&c)?;
    Ok(count)
}

#[tauri::command]
pub async fn export_campaign_report(campaign_id: String, format: String, _log_state: State<'_, crate::LogState>) -> Result<String, String> {
    let c = load_campaign(&campaign_id)?;
    let reports_dir = crate::get_vault_path().join("reports");
    std::fs::create_dir_all(&reports_dir).map_err(|e| e.to_string())?;
    let ts = Utc::now().timestamp_millis();
    match format.as_str() {
        "json" => {
            let p = reports_dir.join(format!("campaign_{}_{}.json", c.id, ts));
            std::fs::write(&p, serde_json::to_string_pretty(&c).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            Ok(p.display().to_string())
        }
        "csv" => {
            let p = reports_dir.join(format!("campaign_{}_{}.csv", c.id, ts));
            let mut lines = vec!["severity,target,title,status,module,date".to_string()];
            for f in &c.findings { lines.push(format!("{},{},{},{},{},{}", f.severity, f.target_ip, f.title.replace(','," "), f.status, f.module, f.found_at)); }
            std::fs::write(&p, lines.join("\n")).map_err(|e| e.to_string())?;
            Ok(p.display().to_string())
        }
        _ => {
            let p = reports_dir.join(format!("campaign_{}_{}.md", c.id, ts));
            let mut md = format!("# {}\n\nClient: {}\nStatus: {:?}\n\n## Findings\n\n", c.name, c.client_name, c.status);
            for f in &c.findings { md.push_str(&format!("- **{}** [{}] {} ({})\n", f.severity, f.target_ip, f.title, f.status)); }
            md.push_str("\n## Recommendations\n\n");
            for f in &c.findings { md.push_str(&format!("- {}\n", f.recommendation)); }
            md.push_str("\n## Timeline\n\n");
            for t in &c.timeline { md.push_str(&format!("- {} | {} | {}\n", t.timestamp, t.event_type, t.description)); }
            std::fs::write(&p, md).map_err(|e| e.to_string())?;
            Ok(p.display().to_string())
        }
    }
}
