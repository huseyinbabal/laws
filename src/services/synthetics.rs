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
pub struct Canary {
    pub name: String,
    pub arn: String,
    pub artifact_s3_location: String,
    pub runtime_version: String,
    pub status: String,
    pub created_at: String,
    pub handler: String,
    pub schedule_expression: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct SyntheticsState {
    pub canaries: DashMap<String, Canary>,
}

impl Default for SyntheticsState {
    fn default() -> Self {
        Self {
            canaries: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<SyntheticsState>) -> axum::Router {
    axum::Router::new()
        .route("/canary", post(create_canary))
        .route("/canary/{name}", get(get_canary).delete(delete_canary))
        .route("/canaries", get(describe_canaries))
        .route("/canary/{name}/start", post(start_canary))
        .route("/canary/{name}/stop", post(stop_canary))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateCanaryRequest {
    #[serde(alias = "Name")]
    name: String,
    #[serde(alias = "ArtifactS3Location", default)]
    artifact_s3_location: Option<String>,
    #[serde(alias = "RuntimeVersion", default)]
    runtime_version: Option<String>,
    #[serde(alias = "Handler", default)]
    handler: Option<String>,
    #[serde(alias = "Schedule", default)]
    schedule: Option<CanaryScheduleInput>,
}

#[derive(Deserialize)]
struct CanaryScheduleInput {
    #[serde(alias = "Expression", default)]
    expression: Option<String>,
}

async fn create_canary(
    State(state): State<Arc<SyntheticsState>>,
    Json(req): Json<CreateCanaryRequest>,
) -> Response {
    if state.canaries.contains_key(&req.name) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "Canary already exists: {}",
            req.name
        )));
    }

    let arn = format!(
        "arn:aws:synthetics:{REGION}:{ACCOUNT_ID}:canary:{name}",
        name = req.name
    );
    let now = Utc::now().to_rfc3339();

    let canary = Canary {
        name: req.name.clone(),
        arn,
        artifact_s3_location: req
            .artifact_s3_location
            .unwrap_or_else(|| format!("s3://cw-syn-results-{ACCOUNT_ID}-{REGION}")),
        runtime_version: req
            .runtime_version
            .unwrap_or_else(|| "syn-nodejs-puppeteer-6.0".into()),
        status: "READY".to_string(),
        created_at: now,
        handler: req.handler.unwrap_or_else(|| "index.handler".into()),
        schedule_expression: req
            .schedule
            .and_then(|s| s.expression)
            .unwrap_or_else(|| "rate(5 minutes)".into()),
    };

    let resp = canary_to_json(&canary);
    state.canaries.insert(req.name, canary);

    rest_json::created(json!({ "Canary": resp }))
}

async fn get_canary(
    State(state): State<Arc<SyntheticsState>>,
    Path(name): Path<String>,
) -> Response {
    match state.canaries.get(&name) {
        Some(c) => rest_json::ok(json!({ "Canary": canary_to_json(c.value()) })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Canary not found: {name}")))
        }
    }
}

async fn describe_canaries(State(state): State<Arc<SyntheticsState>>) -> Response {
    let canaries: Vec<Value> = state
        .canaries
        .iter()
        .map(|entry| canary_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "Canaries": canaries }))
}

async fn delete_canary(
    State(state): State<Arc<SyntheticsState>>,
    Path(name): Path<String>,
) -> Response {
    match state.canaries.remove(&name) {
        Some(_) => rest_json::ok(json!({})),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Canary not found: {name}")))
        }
    }
}

async fn start_canary(
    State(state): State<Arc<SyntheticsState>>,
    Path(name): Path<String>,
) -> Response {
    match state.canaries.get_mut(&name) {
        Some(mut c) => {
            c.status = "RUNNING".to_string();
            rest_json::ok(json!({}))
        }
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Canary not found: {name}")))
        }
    }
}

async fn stop_canary(
    State(state): State<Arc<SyntheticsState>>,
    Path(name): Path<String>,
) -> Response {
    match state.canaries.get_mut(&name) {
        Some(mut c) => {
            c.status = "STOPPED".to_string();
            rest_json::ok(json!({}))
        }
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Canary not found: {name}")))
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn canary_to_json(c: &Canary) -> Value {
    json!({
        "Name": c.name,
        "Arn": c.arn,
        "ArtifactS3Location": c.artifact_s3_location,
        "RuntimeVersion": c.runtime_version,
        "Status": { "State": c.status },
        "Timeline": { "Created": c.created_at },
        "Handler": c.handler,
        "Schedule": { "Expression": c.schedule_expression },
    })
}
