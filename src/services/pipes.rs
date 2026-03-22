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
pub struct Pipe {
    pub name: String,
    pub arn: String,
    pub source: String,
    pub target: String,
    pub description: String,
    pub desired_state: String,
    pub current_state: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct PipesState {
    pub pipes: DashMap<String, Pipe>,
}

impl Default for PipesState {
    fn default() -> Self {
        Self {
            pipes: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<PipesState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/v1/pipes",
            axum::routing::get(list_pipes),
        )
        .route(
            "/v1/pipes/{name}",
            axum::routing::post(create_pipe)
                .get(describe_pipe)
                .delete(delete_pipe)
                .put(update_pipe),
        )
        .route(
            "/v1/pipes/{name}/start",
            axum::routing::post(start_pipe),
        )
        .route(
            "/v1/pipes/{name}/stop",
            axum::routing::post(stop_pipe),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pipe_to_json(p: &Pipe) -> Value {
    json!({
        "Name": p.name,
        "Arn": p.arn,
        "Source": p.source,
        "Target": p.target,
        "Description": p.description,
        "DesiredState": p.desired_state,
        "CurrentState": p.current_state,
        "CreationTime": p.created_at,
    })
}

fn pipe_summary(p: &Pipe) -> Value {
    json!({
        "Name": p.name,
        "Arn": p.arn,
        "Source": p.source,
        "Target": p.target,
        "DesiredState": p.desired_state,
        "CurrentState": p.current_state,
        "CreationTime": p.created_at,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_pipe(
    State(state): State<Arc<PipesState>>,
    Path(name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        if state.pipes.contains_key(&name) {
            return Err(LawsError::AlreadyExists(format!(
                "Pipe '{}' already exists",
                name
            )));
        }

        let source = payload["Source"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing Source".into()))?
            .to_string();

        let target = payload["Target"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing Target".into()))?
            .to_string();

        let description = payload["Description"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let desired_state = payload["DesiredState"]
            .as_str()
            .unwrap_or("RUNNING")
            .to_string();

        let arn = format!(
            "arn:aws:pipes:{REGION}:{ACCOUNT_ID}:pipe/{name}"
        );
        let created_at = chrono::Utc::now().to_rfc3339();

        let pipe = Pipe {
            name: name.clone(),
            arn: arn.clone(),
            source,
            target,
            description,
            desired_state: desired_state.clone(),
            current_state: desired_state,
            created_at: created_at.clone(),
        };

        state.pipes.insert(name.clone(), pipe);

        Ok(rest_json::created(json!({
            "Name": name,
            "Arn": arn,
            "DesiredState": "RUNNING",
            "CurrentState": "RUNNING",
            "CreationTime": created_at,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_pipes(
    State(state): State<Arc<PipesState>>,
) -> Response {
    let pipes: Vec<Value> = state
        .pipes
        .iter()
        .map(|entry| pipe_summary(entry.value()))
        .collect();

    rest_json::ok(json!({ "Pipes": pipes }))
}

async fn describe_pipe(
    State(state): State<Arc<PipesState>>,
    Path(name): Path<String>,
) -> Response {
    match state.pipes.get(&name) {
        Some(p) => rest_json::ok(pipe_to_json(p.value())),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Pipe '{}' not found",
            name
        ))),
    }
}

async fn delete_pipe(
    State(state): State<Arc<PipesState>>,
    Path(name): Path<String>,
) -> Response {
    match state.pipes.remove(&name) {
        Some((_, p)) => rest_json::ok(json!({
            "Name": p.name,
            "Arn": p.arn,
            "DesiredState": "DELETED",
            "CurrentState": "DELETING",
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Pipe '{}' not found",
            name
        ))),
    }
}

async fn start_pipe(
    State(state): State<Arc<PipesState>>,
    Path(name): Path<String>,
) -> Response {
    match state.pipes.get_mut(&name) {
        Some(mut p) => {
            p.desired_state = "RUNNING".to_string();
            p.current_state = "RUNNING".to_string();
            rest_json::ok(json!({
                "Name": p.name,
                "Arn": p.arn,
                "DesiredState": "RUNNING",
                "CurrentState": "RUNNING",
            }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Pipe '{}' not found",
            name
        ))),
    }
}

async fn stop_pipe(
    State(state): State<Arc<PipesState>>,
    Path(name): Path<String>,
) -> Response {
    match state.pipes.get_mut(&name) {
        Some(mut p) => {
            p.desired_state = "STOPPED".to_string();
            p.current_state = "STOPPED".to_string();
            rest_json::ok(json!({
                "Name": p.name,
                "Arn": p.arn,
                "DesiredState": "STOPPED",
                "CurrentState": "STOPPED",
            }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Pipe '{}' not found",
            name
        ))),
    }
}

async fn update_pipe(
    State(state): State<Arc<PipesState>>,
    Path(name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    match state.pipes.get_mut(&name) {
        Some(mut p) => {
            if let Some(desc) = payload["Description"].as_str() {
                p.description = desc.to_string();
            }
            if let Some(target) = payload["Target"].as_str() {
                p.target = target.to_string();
            }
            if let Some(ds) = payload["DesiredState"].as_str() {
                p.desired_state = ds.to_string();
                p.current_state = ds.to_string();
            }
            rest_json::ok(json!({
                "Name": p.name,
                "Arn": p.arn,
                "DesiredState": p.desired_state,
                "CurrentState": p.current_state,
            }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Pipe '{}' not found",
            name
        ))),
    }
}
