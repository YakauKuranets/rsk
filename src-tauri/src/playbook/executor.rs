use super::parser::{parse_playbook, validate_playbook, Playbook, StepResult, StepStatus};
use super::steps;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybookExecution {
    pub playbook: Playbook,
    pub results: Vec<StepResult>,
    pub status: String,
    pub current_step_index: usize,
    pub pending_approval: Option<String>,
}

pub struct PlaybookExecutionState {
    pub active_execution: Arc<Mutex<Option<PlaybookExecution>>>,
}

fn eval_condition(cond: &str, outputs: &HashMap<String, serde_json::Value>) -> bool {
    let s = cond.trim();
    if let Some((lhs, rhs)) = s.split_once("==") {
        let lhs = lhs.trim();
        let rhs = rhs.trim().trim_matches('"');
        if lhs.ends_with(".status") {
            let step_id = lhs.trim_end_matches(".status");
            return outputs
                .get(step_id)
                .and_then(|v| v.get("status"))
                .and_then(|v| v.as_str())
                .map(|v| v.eq_ignore_ascii_case(rhs))
                .unwrap_or(false);
        }
    }
    if let Some((lhs, rhs)) = s.split_once('>') {
        let lhs = lhs.trim();
        let rhs_num = rhs.trim().parse::<f64>().unwrap_or(0.0);
        let mut parts = lhs.split('.');
        let sid = parts.next().unwrap_or_default();
        let mut cur = outputs.get(sid);
        for p in parts {
            cur = cur.and_then(|c| c.get(p));
        }
        return cur.and_then(|v| v.as_f64()).map(|n| n > rhs_num).unwrap_or(false);
    }
    false
}

fn status_payload(exec: &PlaybookExecution) -> serde_json::Value {
    json!({
        "status": exec.status,
        "currentStepIndex": exec.current_step_index,
        "pendingApproval": exec.pending_approval,
        "steps": exec.results,
        "playbookName": exec.playbook.name,
    })
}

async fn continue_execution(
    exec: &mut PlaybookExecution,
    log_state: &State<'_, crate::LogState>,
    start_from: usize,
) -> Result<(), String> {
    let mut previous_outputs: HashMap<String, serde_json::Value> = HashMap::new();
    for r in &exec.results {
        if matches!(r.status, StepStatus::Completed) {
            previous_outputs.insert(r.step_id.clone(), r.output.clone());
        }
    }

    for idx in start_from..exec.playbook.steps.len() {
        exec.current_step_index = idx;
        let step = exec.playbook.steps[idx].clone();

        if let Some(cond) = &step.condition {
            if !eval_condition(cond, &previous_outputs) {
                exec.results.push(StepResult {
                    step_id: step.id.clone(),
                    step_name: step.name.clone(),
                    module: step.module.clone(),
                    status: StepStatus::Skipped,
                    output: json!({"reason": "condition_not_met"}),
                    duration_ms: 0,
                    error: None,
                });
                continue;
            }
        }

        if step.requires_approval {
            exec.pending_approval = Some(step.id.clone());
            exec.status = "waiting_approval".to_string();
            exec.results.push(StepResult {
                step_id: step.id.clone(),
                step_name: step.name.clone(),
                module: step.module.clone(),
                status: StepStatus::WaitingApproval,
                output: json!({}),
                duration_ms: 0,
                error: None,
            });
            crate::push_runtime_log(log_state, format!("[PLAYBOOK] waiting approval for {}", step.id));
            return Ok(());
        }

        let t0 = Instant::now();
        crate::push_runtime_log(log_state, format!("[PLAYBOOK] running step {} ({})", step.id, step.module));
        match steps::execute_step(&step, &exec.playbook.scope, &exec.playbook.variables, &previous_outputs).await {
            Ok(output) => {
                let elapsed = t0.elapsed().as_millis() as u64;
                previous_outputs.insert(step.id.clone(), output.clone());
                exec.results.push(StepResult {
                    step_id: step.id.clone(),
                    step_name: step.name.clone(),
                    module: step.module.clone(),
                    status: StepStatus::Completed,
                    output,
                    duration_ms: elapsed,
                    error: None,
                });
            }
            Err(err) => {
                exec.status = "failed".to_string();
                exec.results.push(StepResult {
                    step_id: step.id.clone(),
                    step_name: step.name.clone(),
                    module: step.module.clone(),
                    status: StepStatus::Failed,
                    output: json!({}),
                    duration_ms: t0.elapsed().as_millis() as u64,
                    error: Some(err),
                });
                return Ok(());
            }
        }
    }

    exec.status = "completed".to_string();
    exec.pending_approval = None;
    Ok(())
}

#[tauri::command]
pub async fn start_playbook(
    yaml_content: String,
    log_state: State<'_, crate::LogState>,
    exec_state: State<'_, PlaybookExecutionState>,
) -> Result<serde_json::Value, String> {
    let pb = parse_playbook(&yaml_content)?;
    if let Err(errors) = validate_playbook(&pb) {
        return Err(format!("Validation errors: {}", errors.join("; ")));
    }

    let mut exec = PlaybookExecution {
        playbook: pb,
        results: Vec::new(),
        status: "running".to_string(),
        current_step_index: 0,
        pending_approval: None,
    };

    continue_execution(&mut exec, &log_state, 0).await?;
    let payload = status_payload(&exec);

    let mut guard = exec_state.active_execution.lock().map_err(|_| "playbook state lock poisoned")?;
    *guard = Some(exec);

    Ok(payload)
}

#[tauri::command]
pub async fn approve_playbook_step(
    step_id: String,
    approved: bool,
    log_state: State<'_, crate::LogState>,
    exec_state: State<'_, PlaybookExecutionState>,
) -> Result<serde_json::Value, String> {
    let mut guard = exec_state.active_execution.lock().map_err(|_| "playbook state lock poisoned")?;
    let Some(exec) = guard.as_mut() else { return Err("No active playbook execution".into()); };

    if exec.pending_approval.as_deref() != Some(step_id.as_str()) {
        return Err("Requested step is not pending approval".into());
    }

    let idx = exec.current_step_index;
    let step = exec.playbook.steps.get(idx).cloned().ok_or("Invalid step index")?;

    if let Some(last) = exec.results.last_mut() {
        if last.step_id == step_id {
            if !approved {
                last.status = StepStatus::Cancelled;
                last.output = json!({"approved": false});
            } else {
                let t0 = Instant::now();
                let mut previous_outputs = HashMap::new();
                for r in &exec.results {
                    if matches!(r.status, StepStatus::Completed) {
                        previous_outputs.insert(r.step_id.clone(), r.output.clone());
                    }
                }
                match steps::execute_step(&step, &exec.playbook.scope, &exec.playbook.variables, &previous_outputs).await {
                    Ok(output) => {
                        last.status = StepStatus::Completed;
                        last.output = output;
                        last.duration_ms = t0.elapsed().as_millis() as u64;
                    }
                    Err(e) => {
                        last.status = StepStatus::Failed;
                        last.error = Some(e);
                        exec.status = "failed".to_string();
                    }
                }
            }
        }
    }

    exec.pending_approval = None;
    if exec.status != "failed" {
        exec.status = "running".to_string();
        continue_execution(exec, &log_state, idx + 1).await?;
    }

    Ok(status_payload(exec))
}

#[tauri::command]
pub fn get_playbook_status(
    exec_state: State<'_, PlaybookExecutionState>,
) -> Result<serde_json::Value, String> {
    let guard = exec_state.active_execution.lock().map_err(|_| "playbook state lock poisoned")?;
    let Some(exec) = guard.as_ref() else { return Ok(json!({"status": "idle", "steps": []})); };
    Ok(status_payload(exec))
}
