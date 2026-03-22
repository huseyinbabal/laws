use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::Json;
use dashmap::DashMap;
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
pub struct Monitor {
    pub monitor_name: String,
    pub monitor_arn: String,
    pub status: String,
    pub max_city_networks_to_monitor: u32,
    pub resources: Vec<String>,
    pub created_at: String,
    pub modified_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct InternetMonitorState {
    pub monitors: DashMap<String, Monitor>,
}

impl Default for InternetMonitorState {
    fn default() -> Self {
        Self {
            monitors: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<InternetMonitorState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/v20210603/Monitors",
            axum::routing::post(create_monitor).get(list_monitors),
        )
        .route(
            "/v20210603/Monitors/{name}",
            axum::routing::get(get_monitor)
                .delete(delete_monitor)
                .patch(update_monitor),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_monitor(
    State(state): State<Arc<InternetMonitorState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("MonitorName")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let max_city = payload
        .get("MaxCityNetworksToMonitor")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as u32;

    let resources: Vec<String> = payload
        .get("Resources")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();

    let arn = format!("arn:aws:internetmonitor:{REGION}:{ACCOUNT_ID}:monitor/{name}");
    let now = chrono::Utc::now().to_rfc3339();

    let monitor = Monitor {
        monitor_name: name.clone(),
        monitor_arn: arn.clone(),
        status: "ACTIVE".into(),
        max_city_networks_to_monitor: max_city,
        resources,
        created_at: now.clone(),
        modified_at: now.clone(),
    };

    state.monitors.insert(name.clone(), monitor);

    rest_json::created(json!({
        "MonitorArn": arn,
        "Status": "ACTIVE"
    }))
}

async fn get_monitor(
    State(state): State<Arc<InternetMonitorState>>,
    Path(name): Path<String>,
) -> Response {
    match state.monitors.get(&name) {
        Some(m) => rest_json::ok(json!({
            "MonitorName": m.monitor_name,
            "MonitorArn": m.monitor_arn,
            "Status": m.status,
            "MaxCityNetworksToMonitor": m.max_city_networks_to_monitor,
            "Resources": m.resources,
            "CreatedAt": m.created_at,
            "ModifiedAt": m.modified_at
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Monitor not found: {name}")))
        }
    }
}

async fn list_monitors(State(state): State<Arc<InternetMonitorState>>) -> Response {
    let monitors: Vec<Value> = state
        .monitors
        .iter()
        .map(|e| {
            let m = e.value();
            json!({
                "MonitorName": m.monitor_name,
                "MonitorArn": m.monitor_arn,
                "Status": m.status
            })
        })
        .collect();

    rest_json::ok(json!({
        "Monitors": monitors
    }))
}

async fn delete_monitor(
    State(state): State<Arc<InternetMonitorState>>,
    Path(name): Path<String>,
) -> Response {
    match state.monitors.remove(&name) {
        Some(_) => rest_json::no_content(),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Monitor not found: {name}")))
        }
    }
}

async fn update_monitor(
    State(state): State<Arc<InternetMonitorState>>,
    Path(name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    match state.monitors.get_mut(&name) {
        Some(mut m) => {
            if let Some(status) = payload.get("Status").and_then(|v| v.as_str()) {
                m.status = status.to_owned();
            }
            if let Some(max_city) = payload
                .get("MaxCityNetworksToMonitor")
                .and_then(|v| v.as_u64())
            {
                m.max_city_networks_to_monitor = max_city as u32;
            }
            if let Some(resources) = payload.get("Resources").and_then(|v| v.as_array()) {
                m.resources = resources
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                    .collect();
            }
            m.modified_at = chrono::Utc::now().to_rfc3339();

            rest_json::ok(json!({
                "MonitorArn": m.monitor_arn,
                "Status": m.status
            }))
        }
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Monitor not found: {name}")))
        }
    }
}
