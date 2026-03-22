use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::rest_json;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AppMonitor {
    pub name: String,
    pub id: String,
    pub arn: String,
    pub domain: String,
    pub status: String,
    pub created: String,
    pub last_modified: String,
    pub cw_log_enabled: bool,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct RumState {
    pub app_monitors: DashMap<String, AppMonitor>,
}

impl Default for RumState {
    fn default() -> Self {
        Self {
            app_monitors: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<RumState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/appmonitors",
            post(create_app_monitor).get(list_app_monitors),
        )
        .route(
            "/appmonitors/{name}",
            get(get_app_monitor)
                .delete(delete_app_monitor)
                .put(update_app_monitor),
        )
        .route("/appmonitors/{name}/data", post(put_rum_events))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateAppMonitorRequest {
    #[serde(alias = "Name")]
    name: String,
    #[serde(alias = "Domain")]
    domain: String,
    #[serde(alias = "CwLogEnabled", default)]
    cw_log_enabled: Option<bool>,
}

async fn create_app_monitor(
    State(state): State<Arc<RumState>>,
    Json(req): Json<CreateAppMonitorRequest>,
) -> Response {
    if state.app_monitors.contains_key(&req.name) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "AppMonitor already exists: {}",
            req.name
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:rum:{REGION}:{ACCOUNT_ID}:appmonitor/{name}",
        name = req.name
    );
    let now = Utc::now().to_rfc3339();

    let monitor = AppMonitor {
        name: req.name.clone(),
        id: id.clone(),
        arn,
        domain: req.domain,
        status: "ACTIVE".to_string(),
        created: now.clone(),
        last_modified: now,
        cw_log_enabled: req.cw_log_enabled.unwrap_or(false),
    };

    state.app_monitors.insert(req.name, monitor);

    rest_json::ok(json!({ "Id": id }))
}

async fn list_app_monitors(State(state): State<Arc<RumState>>) -> Response {
    let monitors: Vec<Value> = state
        .app_monitors
        .iter()
        .map(|entry| {
            let m = entry.value();
            json!({
                "Name": m.name,
                "Id": m.id,
                "State": m.status,
                "Created": m.created,
                "LastModified": m.last_modified,
            })
        })
        .collect();

    rest_json::ok(json!({ "AppMonitorSummaries": monitors }))
}

async fn get_app_monitor(State(state): State<Arc<RumState>>, Path(name): Path<String>) -> Response {
    match state.app_monitors.get(&name) {
        Some(m) => rest_json::ok(json!({
            "AppMonitor": {
                "Name": m.name,
                "Id": m.id,
                "Arn": m.arn,
                "Domain": m.domain,
                "State": m.status,
                "Created": m.created,
                "LastModified": m.last_modified,
                "CwLogEnabled": m.cw_log_enabled,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "AppMonitor not found: {name}"
        ))),
    }
}

async fn delete_app_monitor(
    State(state): State<Arc<RumState>>,
    Path(name): Path<String>,
) -> Response {
    match state.app_monitors.remove(&name) {
        Some(_) => rest_json::ok(json!({})),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "AppMonitor not found: {name}"
        ))),
    }
}

#[derive(Deserialize)]
struct UpdateAppMonitorRequest {
    #[serde(alias = "Domain", default)]
    domain: Option<String>,
    #[serde(alias = "CwLogEnabled", default)]
    cw_log_enabled: Option<bool>,
}

async fn update_app_monitor(
    State(state): State<Arc<RumState>>,
    Path(name): Path<String>,
    Json(req): Json<UpdateAppMonitorRequest>,
) -> Response {
    match state.app_monitors.get_mut(&name) {
        Some(mut m) => {
            if let Some(domain) = req.domain {
                m.domain = domain;
            }
            if let Some(cw) = req.cw_log_enabled {
                m.cw_log_enabled = cw;
            }
            m.last_modified = Utc::now().to_rfc3339();
            rest_json::ok(json!({}))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "AppMonitor not found: {name}"
        ))),
    }
}

async fn put_rum_events(
    State(state): State<Arc<RumState>>,
    Path(name): Path<String>,
    Json(_body): Json<Value>,
) -> Response {
    if !state.app_monitors.contains_key(&name) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "AppMonitor not found: {name}"
        )));
    }

    rest_json::ok(json!({}))
}
