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
pub struct Component {
    pub arn: String,
    pub name: String,
    pub version: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct CoreDevice {
    pub thing_name: String,
    pub status: String,
    pub last_status_update: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct GreengrassState {
    pub components: DashMap<String, Component>,
    pub core_devices: DashMap<String, CoreDevice>,
}

impl Default for GreengrassState {
    fn default() -> Self {
        Self {
            components: DashMap::new(),
            core_devices: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<GreengrassState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/greengrass/v2/components",
            axum::routing::post(create_component_version).get(list_components),
        )
        .route(
            "/greengrass/v2/components/{arn}",
            axum::routing::get(get_component).delete(delete_component),
        )
        .route(
            "/greengrass/v2/coreDevices",
            axum::routing::get(list_core_devices),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_component_version(
    State(state): State<Arc<GreengrassState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("componentName")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let version = payload
        .get("componentVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_owned();

    let arn =
        format!("arn:aws:greengrass:{REGION}:{ACCOUNT_ID}:components:{name}:versions:{version}");
    let now = chrono::Utc::now().to_rfc3339();

    let component = Component {
        arn: arn.clone(),
        name: name.clone(),
        version: version.clone(),
        status: "DEPLOYABLE".into(),
        created_at: now.clone(),
    };

    state.components.insert(arn.clone(), component);

    rest_json::created(json!({
        "arn": arn,
        "componentName": name,
        "componentVersion": version,
        "creationTimestamp": now,
        "status": { "componentState": "DEPLOYABLE" }
    }))
}

async fn list_components(State(state): State<Arc<GreengrassState>>) -> Response {
    let components: Vec<Value> = state
        .components
        .iter()
        .map(|e| {
            let c = e.value();
            json!({
                "arn": c.arn,
                "componentName": c.name,
                "latestVersion": {
                    "arn": c.arn,
                    "componentVersion": c.version,
                    "creationTimestamp": c.created_at
                }
            })
        })
        .collect();

    rest_json::ok(json!({
        "components": components
    }))
}

async fn get_component(
    State(state): State<Arc<GreengrassState>>,
    Path(arn): Path<String>,
) -> Response {
    match state.components.get(&arn) {
        Some(c) => rest_json::ok(json!({
            "arn": c.arn,
            "componentName": c.name,
            "componentVersion": c.version,
            "creationTimestamp": c.created_at,
            "status": { "componentState": c.status }
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Component not found: {arn}")))
        }
    }
}

async fn delete_component(
    State(state): State<Arc<GreengrassState>>,
    Path(arn): Path<String>,
) -> Response {
    match state.components.remove(&arn) {
        Some(_) => rest_json::no_content(),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Component not found: {arn}")))
        }
    }
}

async fn list_core_devices(State(state): State<Arc<GreengrassState>>) -> Response {
    let devices: Vec<Value> = state
        .core_devices
        .iter()
        .map(|e| {
            let d = e.value();
            json!({
                "coreDeviceThingName": d.thing_name,
                "status": d.status,
                "lastStatusUpdateTimestamp": d.last_status_update
            })
        })
        .collect();

    rest_json::ok(json!({
        "coreDevices": devices
    }))
}
