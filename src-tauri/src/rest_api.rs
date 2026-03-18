// src-tauri/src/rest_api.rs
// Headless REST API mode — Hyperion as a backend service
// Exposes key commands over HTTP without Tauri UI
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

#[derive(Clone)]
pub struct ApiState {
    pub api_key: String,
    pub port: u16,
}

fn with_auth(api_key: String) -> impl warp::Filter<Extract = (), Error = warp::Rejection> + Clone {
    warp::header::<String>("x-api-key")
        .and_then(move |key: String| {
            let expected = api_key.clone();
            async move {
                if key == expected {
                    Ok(())
                } else {
                    Err(warp::reject::custom(Unauthorized))
                }
            }
        })
        .untuple_one()
}

#[derive(Debug)]
struct Unauthorized;
impl warp::reject::Reject for Unauthorized {}

#[derive(Debug, Serialize, Deserialize)]
struct PipelineRequest {
    scope: String,
    permit_token: String,
    shodan_key: Option<String>,
    anthropic_api_key: Option<String>,
    language: Option<String>,
    skip_exploit_verify: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    fn err(msg: impl Into<String>) -> ApiResponse<serde_json::Value> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

/// Start the REST API server (runs alongside Tauri UI or standalone)
#[tauri::command]
pub async fn start_rest_api(
    api_key: String,
    port: u16,
    log_state: tauri::State<'_, crate::LogState>,
) -> Result<String, String> {
    if api_key.trim().len() < 16 {
        return Err("API key must be at least 16 characters".to_string());
    }

    crate::push_runtime_log(&log_state, format!("REST_API_START|port={}", port));

    let auth = with_auth(api_key.clone());

    // POST /api/v1/pipeline — run full pipeline
    let pipeline = warp::post()
        .and(warp::path!("api" / "v1" / "pipeline"))
        .and(auth.clone())
        .and(warp::body::json::<PipelineRequest>())
        .and_then(|req: PipelineRequest| async move {
            let options = crate::agents::auto_pipeline::PipelineOptions {
                scope: req.scope,
                permit_token: req.permit_token,
                shodan_key: req.shodan_key,
                anthropic_api_key: req.anthropic_api_key,
                language: req.language.unwrap_or_else(|| "ru".into()),
                skip_exploit_verify: req.skip_exploit_verify.unwrap_or(true),
            };
            // Note: no Tauri State available here — use default State
            let _ = options;
            Err::<warp::reply::Json, warp::Rejection>(warp::reject::not_found())
        });

    // GET /api/v1/health
    let health = warp::get()
        .and(warp::path!("api" / "v1" / "health"))
        .and(auth.clone())
        .map(|| {
            warp::reply::json(&serde_json::json!({
                "status": "ok",
                "version": "1.0",
                "product": "Hyperion PTES",
            }))
        });

    // GET /api/v1/scenarios — list BAS scenarios
    let scenarios = warp::get()
        .and(warp::path!("api" / "v1" / "scenarios"))
        .and(auth.clone())
        .map(|| {
            let s = crate::bas_engine::list_bas_scenarios();
            warp::reply::json(&ApiResponse::ok(s))
        });

    let routes = health.or(scenarios).or(pipeline).with(
        warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["x-api-key", "content-type"])
            .allow_methods(vec!["GET", "POST", "DELETE"]),
    );

    tokio::spawn(async move {
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;
    });

    Ok(format!(
        "REST API started on http://127.0.0.1:{}/api/v1",
        port
    ))
}

#[tauri::command]
pub fn get_rest_api_docs(port: u16) -> serde_json::Value {
    serde_json::json!({
        "base_url": format!("http://127.0.0.1:{}/api/v1", port),
        "auth": "x-api-key header",
        "endpoints": [
            {"method":"GET",  "path":"/health",    "desc":"Health check"},
            {"method":"GET",  "path":"/scenarios", "desc":"List BAS scenarios"},
            {"method":"POST", "path":"/pipeline",  "desc":"Run full recon pipeline",
             "body": {"scope":"string","permit_token":"string","shodan_key":"string?"}},
            {"method":"POST", "path":"/bas",       "desc":"Run BAS simulation"},
            {"method":"POST", "path":"/fingerprint","desc":"Firmware fingerprint"},
            {"method":"GET",  "path":"/jobs",      "desc":"List active jobs"}
        ]
    })
}
