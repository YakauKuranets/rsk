use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackGraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub severity: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackGraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
    pub label: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackGraph {
    pub nodes: Vec<AttackGraphNode>,
    pub edges: Vec<AttackGraphEdge>,
    pub attack_paths: Vec<AttackPath>,
    pub risk_score: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackPath {
    pub path: Vec<String>,
    pub total_risk: f32,
    pub description: String,
}

#[tauri::command]
pub async fn generate_attack_graph(
    targets_json: String,
    log_state: State<'_, crate::LogState>,
) -> Result<AttackGraph, String> {
    crate::push_runtime_log(&log_state, "[ATTACK_GRAPH] Построение графа...".to_string());

    let targets: Vec<serde_json::Value> =
        serde_json::from_str(&targets_json).map_err(|e| e.to_string())?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for target in &targets {
        let ip = target["ip"].as_str().unwrap_or("unknown");
        let host_id = format!("host_{}", ip.replace('.', "_"));

        let mut host_meta = HashMap::new();
        host_meta.insert("ip".into(), ip.into());

        nodes.push(AttackGraphNode {
            id: host_id.clone(),
            label: ip.to_string(),
            node_type: "host".into(),
            severity: None,
            metadata: host_meta,
        });

        if let Some(ports) = target["openPorts"].as_array() {
            for port_val in ports {
                let port = port_val.as_u64().unwrap_or(0);
                let svc_id = format!("{}_port_{}", host_id, port);
                let mut svc_meta = HashMap::new();
                svc_meta.insert("port".into(), port.to_string());

                nodes.push(AttackGraphNode {
                    id: svc_id.clone(),
                    label: format!(":{}", port),
                    node_type: "service".into(),
                    severity: None,
                    metadata: svc_meta,
                });
                edges.push(AttackGraphEdge {
                    from: host_id.clone(),
                    to: svc_id,
                    edge_type: "exposes".into(),
                    label: format!("port {}", port),
                    weight: 0.5,
                });
            }
        }

        if let Some(vulns) = target["vulnerabilities"].as_array() {
            for vuln in vulns {
                let cve = vuln["cveId"].as_str().unwrap_or("unknown_vuln");
                let cvss = vuln["cvssScore"].as_f64().unwrap_or(0.0) as f32;
                let vuln_id = format!("{}_{}", host_id, cve.replace('-', "_"));

                let mut vuln_meta = HashMap::new();
                vuln_meta.insert("cvss".into(), format!("{:.1}", cvss));

                nodes.push(AttackGraphNode {
                    id: vuln_id.clone(),
                    label: cve.to_string(),
                    node_type: "vulnerability".into(),
                    severity: Some(classify_severity(cvss)),
                    metadata: vuln_meta,
                });
                edges.push(AttackGraphEdge {
                    from: host_id.clone(),
                    to: vuln_id,
                    edge_type: "has_vuln".into(),
                    label: format!("CVSS {:.1}", cvss),
                    weight: (cvss / 10.0).clamp(0.0, 1.0),
                });
            }
        }

        if let Some(creds) = target["credentials"].as_object() {
            let cred_id = format!("{}_cred", host_id);
            let login = creds.get("login").and_then(|v| v.as_str()).unwrap_or("?");

            let mut cred_meta = HashMap::new();
            cred_meta.insert("login".into(), login.into());

            nodes.push(AttackGraphNode {
                id: cred_id.clone(),
                label: format!("creds: {}:***", login),
                node_type: "credential".into(),
                severity: Some("critical".into()),
                metadata: cred_meta,
            });
            edges.push(AttackGraphEdge {
                from: host_id.clone(),
                to: cred_id,
                edge_type: "uses_cred".into(),
                label: "weak/default credentials".into(),
                weight: 0.9,
            });
        }
    }

    dedupe_graph(&mut nodes, &mut edges);
    let attack_paths = find_attack_paths(&nodes);
    let risk_score = calculate_overall_risk(&attack_paths);

    crate::push_runtime_log(
        &log_state,
        format!(
            "[ATTACK_GRAPH] Готово: {} nodes, {} edges, {} paths",
            nodes.len(),
            edges.len(),
            attack_paths.len()
        ),
    );

    Ok(AttackGraph {
        nodes,
        edges,
        attack_paths,
        risk_score,
    })
}

fn dedupe_graph(nodes: &mut Vec<AttackGraphNode>, edges: &mut Vec<AttackGraphEdge>) {
    let mut seen_nodes = HashSet::new();
    nodes.retain(|n| seen_nodes.insert(n.id.clone()));

    let mut seen_edges = HashSet::new();
    edges.retain(|e| seen_edges.insert(format!("{}|{}|{}", e.from, e.to, e.edge_type)));
}

fn classify_severity(cvss: f32) -> String {
    match cvss {
        s if s >= 9.0 => "critical",
        s if s >= 7.0 => "high",
        s if s >= 4.0 => "medium",
        _ => "low",
    }
    .to_string()
}

fn find_attack_paths(nodes: &[AttackGraphNode]) -> Vec<AttackPath> {
    let mut paths = Vec::new();

    let vuln_nodes: Vec<&AttackGraphNode> = nodes
        .iter()
        .filter(|n| n.node_type == "vulnerability")
        .collect();
    let cred_nodes: Vec<&AttackGraphNode> = nodes
        .iter()
        .filter(|n| n.node_type == "credential")
        .collect();

    for vuln in &vuln_nodes {
        for cred in &cred_nodes {
            if vuln.id.split('_').take(2).eq(cred.id.split('_').take(2)) {
                let base = match vuln.severity.as_deref() {
                    Some("critical") => 0.95,
                    Some("high") => 0.8,
                    Some("medium") => 0.6,
                    _ => 0.4,
                };
                paths.push(AttackPath {
                    path: vec![vuln.id.clone(), cred.id.clone()],
                    total_risk: base,
                    description: format!(
                        "Уязвимость {} может привести к компрометации учётных данных",
                        vuln.label
                    ),
                });
            }
        }
    }

    paths
}

fn calculate_overall_risk(paths: &[AttackPath]) -> f32 {
    if paths.is_empty() {
        return 0.0;
    }
    paths.iter().map(|p| p.total_risk).fold(0.0f32, f32::max)
}

// Добавить в конец attack_graph.rs — критические пути через Dijkstra

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CriticalPath {
    pub path: Vec<String>,
    pub total_risk: f32,
    pub mitre_techniques: Vec<String>,
    pub estimated_time_minutes: u32,
    pub description: String,
}

/// Find top-N highest-risk attack paths using weighted graph traversal
fn find_critical_paths_weighted(
    nodes: &[AttackGraphNode],
    edges: &[AttackGraphEdge],
    max_paths: usize,
) -> Vec<CriticalPath> {
    use std::collections::HashMap;

    // Build adjacency list
    let mut adj: HashMap<&str, Vec<(&str, f32)>> = HashMap::new();
    for edge in edges {
        adj.entry(&edge.from).or_default().push((&edge.to, edge.weight));
    }

    // Find entry points (nodes with no incoming edges)
    let has_incoming: std::collections::HashSet<&str> = edges.iter().map(|e| e.to.as_str()).collect();
    let entry_points: Vec<&str> = nodes
        .iter()
        .filter(|n| !has_incoming.contains(n.id.as_str()))
        .map(|n| n.id.as_str())
        .collect();

    // Find exit points (nodes with no outgoing edges — high value targets)
    let has_outgoing: std::collections::HashSet<&str> = edges.iter().map(|e| e.from.as_str()).collect();
    let exit_points: std::collections::HashSet<&str> = nodes
        .iter()
        .filter(|n| !has_outgoing.contains(n.id.as_str()))
        .map(|n| n.id.as_str())
        .collect();

    let mut all_paths: Vec<CriticalPath> = Vec::new();

    // DFS from each entry point to find all paths to exits
    fn dfs<'a>(
        current: &'a str,
        adj: &HashMap<&'a str, Vec<(&'a str, f32)>>,
        exit_points: &std::collections::HashSet<&'a str>,
        visited: &mut Vec<&'a str>,
        current_risk: f32,
        results: &mut Vec<(Vec<String>, f32)>,
    ) {
        visited.push(current);
        if exit_points.contains(current) {
            results.push((visited.iter().map(|s| s.to_string()).collect(), current_risk));
        }
        if let Some(neighbors) = adj.get(current) {
            for &(next, weight) in neighbors {
                if !visited.contains(&next) && visited.len() < 10 {
                    dfs(next, adj, exit_points, visited, current_risk + weight, results);
                }
            }
        }
        visited.pop();
    }

    for entry in &entry_points {
        let mut raw_paths = Vec::new();
        dfs(entry, &adj, &exit_points, &mut vec![], 0.0, &mut raw_paths);
        for (path, risk) in raw_paths {
            let techniques: Vec<String> = path
                .iter()
                .filter_map(|nid| nodes.iter().find(|n| &n.id == nid))
                .filter_map(|n| n.metadata.get("mitre_id").cloned())
                .collect();
            all_paths.push(CriticalPath {
                path: path.clone(),
                total_risk: risk,
                mitre_techniques: techniques,
                estimated_time_minutes: (risk * 10.0) as u32,
                description: format!(
                    "{} → {} ({} steps, risk {:.1})",
                    path.first().cloned().unwrap_or_default(),
                    path.last().cloned().unwrap_or_default(),
                    path.len(),
                    risk
                ),
            });
        }
    }

    all_paths.sort_by(|a, b| {
        b.total_risk
            .partial_cmp(&a.total_risk)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_paths.truncate(max_paths);
    all_paths
}

#[tauri::command]
pub async fn find_critical_attack_paths(
    targets_json: String,
    max_paths: Option<usize>,
    log_state: tauri::State<'_, crate::LogState>,
) -> Result<Vec<CriticalPath>, String> {
    crate::push_runtime_log(&log_state, "CRITICAL_PATHS|start".to_string());
    let graph = generate_attack_graph(targets_json, log_state).await?;
    let paths = find_critical_paths_weighted(&graph.nodes, &graph.edges, max_paths.unwrap_or(10));
    Ok(paths)
}
