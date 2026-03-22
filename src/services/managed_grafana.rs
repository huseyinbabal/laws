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
pub struct Workspace {
    pub id: String,
    pub arn: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub endpoint: String,
    pub authentication: String,
    pub created_at: String,
    pub modified_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ManagedGrafanaState {
    pub workspaces: DashMap<String, Workspace>,
}

impl Default for ManagedGrafanaState {
    fn default() -> Self {
        Self {
            workspaces: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<ManagedGrafanaState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/workspaces",
            axum::routing::post(create_workspace).get(list_workspaces),
        )
        .route(
            "/workspaces/{id}",
            axum::routing::get(describe_workspace)
                .delete(delete_workspace)
                .put(update_workspace),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_workspace(
    State(state): State<Arc<ManagedGrafanaState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("workspaceName")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let description = payload
        .get("workspaceDescription")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let authentication = payload
        .get("authenticationProviders")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("AWS_SSO")
        .to_owned();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:grafana:{REGION}:{ACCOUNT_ID}:/workspaces/{id}");
    let endpoint = format!("{id}.grafana-workspace.{REGION}.amazonaws.com");
    let now = chrono::Utc::now().to_rfc3339();

    let workspace = Workspace {
        id: id.clone(),
        arn: arn.clone(),
        name: name.clone(),
        description: description.clone(),
        status: "ACTIVE".into(),
        endpoint: endpoint.clone(),
        authentication: authentication.clone(),
        created_at: now.clone(),
        modified_at: now.clone(),
    };

    state.workspaces.insert(id.clone(), workspace);

    rest_json::created(json!({
        "workspace": {
            "id": id,
            "arn": arn,
            "name": name,
            "description": description,
            "status": "ACTIVE",
            "endpoint": endpoint,
            "authentication": { "providers": [authentication] },
            "created": now,
            "modified": now
        }
    }))
}

async fn list_workspaces(
    State(state): State<Arc<ManagedGrafanaState>>,
) -> Response {
    let workspaces: Vec<Value> = state
        .workspaces
        .iter()
        .map(|e| {
            let w = e.value();
            json!({
                "id": w.id,
                "arn": w.arn,
                "name": w.name,
                "description": w.description,
                "status": w.status,
                "endpoint": w.endpoint,
                "created": w.created_at,
                "modified": w.modified_at
            })
        })
        .collect();

    rest_json::ok(json!({
        "workspaces": workspaces
    }))
}

async fn describe_workspace(
    State(state): State<Arc<ManagedGrafanaState>>,
    Path(id): Path<String>,
) -> Response {
    match state.workspaces.get(&id) {
        Some(w) => rest_json::ok(json!({
            "workspace": {
                "id": w.id,
                "arn": w.arn,
                "name": w.name,
                "description": w.description,
                "status": w.status,
                "endpoint": w.endpoint,
                "authentication": { "providers": [w.authentication] },
                "created": w.created_at,
                "modified": w.modified_at
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Workspace not found: {id}"
        ))),
    }
}

async fn delete_workspace(
    State(state): State<Arc<ManagedGrafanaState>>,
    Path(id): Path<String>,
) -> Response {
    match state.workspaces.remove(&id) {
        Some((_, w)) => rest_json::ok(json!({
            "workspace": {
                "id": w.id,
                "arn": w.arn,
                "name": w.name,
                "status": "DELETING"
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Workspace not found: {id}"
        ))),
    }
}

async fn update_workspace(
    State(state): State<Arc<ManagedGrafanaState>>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    match state.workspaces.get_mut(&id) {
        Some(mut w) => {
            if let Some(name) = payload.get("workspaceName").and_then(|v| v.as_str()) {
                w.name = name.to_owned();
            }
            if let Some(desc) = payload.get("workspaceDescription").and_then(|v| v.as_str()) {
                w.description = desc.to_owned();
            }
            w.modified_at = chrono::Utc::now().to_rfc3339();

            rest_json::ok(json!({
                "workspace": {
                    "id": w.id,
                    "arn": w.arn,
                    "name": w.name,
                    "description": w.description,
                    "status": w.status,
                    "endpoint": w.endpoint
                }
            }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Workspace not found: {id}"
        ))),
    }
}
