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
pub struct Workspace {
    pub workspace_id: String,
    pub arn: String,
    pub alias: String,
    pub status: String,
    pub created_at: String,
    pub rule_groups_namespaces: DashMap<String, RuleGroupsNamespace>,
}

#[derive(Debug, Clone)]
pub struct RuleGroupsNamespace {
    pub name: String,
    pub arn: String,
    pub data: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AmpState {
    pub workspaces: DashMap<String, Workspace>,
}

impl Default for AmpState {
    fn default() -> Self {
        Self {
            workspaces: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<AmpState>) -> axum::Router {
    axum::Router::new()
        .route("/workspaces", post(create_workspace).get(list_workspaces))
        .route(
            "/workspaces/{workspace_id}",
            get(describe_workspace).delete(delete_workspace),
        )
        .route(
            "/workspaces/{workspace_id}/rulegroupsnamespaces",
            post(create_rule_groups_namespace).get(list_rule_groups_namespaces),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateWorkspaceRequest {
    #[serde(default)]
    alias: Option<String>,
}

async fn create_workspace(
    State(state): State<Arc<AmpState>>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Response {
    let workspace_id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:aps:{REGION}:{ACCOUNT_ID}:workspace/{workspace_id}"
    );
    let now = Utc::now().to_rfc3339();

    let workspace = Workspace {
        workspace_id: workspace_id.clone(),
        arn: arn.clone(),
        alias: req.alias.unwrap_or_default(),
        status: "ACTIVE".to_string(),
        created_at: now.clone(),
        rule_groups_namespaces: DashMap::new(),
    };

    state.workspaces.insert(workspace_id.clone(), workspace);

    rest_json::created(json!({
        "workspaceId": workspace_id,
        "arn": arn,
        "status": { "statusCode": "ACTIVE" },
    }))
}

async fn list_workspaces(State(state): State<Arc<AmpState>>) -> Response {
    let workspaces: Vec<Value> = state
        .workspaces
        .iter()
        .map(|entry| {
            let w = entry.value();
            json!({
                "workspaceId": w.workspace_id,
                "arn": w.arn,
                "alias": w.alias,
                "status": { "statusCode": w.status },
                "createdAt": w.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "workspaces": workspaces }))
}

async fn describe_workspace(
    State(state): State<Arc<AmpState>>,
    Path(workspace_id): Path<String>,
) -> Response {
    match state.workspaces.get(&workspace_id) {
        Some(w) => rest_json::ok(json!({
            "workspace": {
                "workspaceId": w.workspace_id,
                "arn": w.arn,
                "alias": w.alias,
                "status": { "statusCode": w.status },
                "createdAt": w.created_at,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Workspace not found: {workspace_id}"
        ))),
    }
}

async fn delete_workspace(
    State(state): State<Arc<AmpState>>,
    Path(workspace_id): Path<String>,
) -> Response {
    match state.workspaces.remove(&workspace_id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Workspace not found: {workspace_id}"
        ))),
    }
}

#[derive(Deserialize)]
struct CreateRuleGroupsNamespaceRequest {
    name: String,
    #[serde(default)]
    data: Option<String>,
}

async fn create_rule_groups_namespace(
    State(state): State<Arc<AmpState>>,
    Path(workspace_id): Path<String>,
    Json(req): Json<CreateRuleGroupsNamespaceRequest>,
) -> Response {
    let workspace = match state.workspaces.get(&workspace_id) {
        Some(w) => w,
        None => {
            return rest_json::error_response(&LawsError::NotFound(format!(
                "Workspace not found: {workspace_id}"
            )));
        }
    };

    if workspace.rule_groups_namespaces.contains_key(&req.name) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "Rule groups namespace already exists: {}",
            req.name
        )));
    }

    let arn = format!(
        "arn:aws:aps:{REGION}:{ACCOUNT_ID}:rulegroupsnamespace/{workspace_id}/{name}",
        name = req.name
    );
    let now = Utc::now().to_rfc3339();

    let ns = RuleGroupsNamespace {
        name: req.name.clone(),
        arn: arn.clone(),
        data: req.data.unwrap_or_default(),
        status: "ACTIVE".to_string(),
        created_at: now,
    };

    let resp = json!({
        "name": ns.name,
        "arn": ns.arn,
        "status": { "statusCode": "ACTIVE" },
    });

    workspace.rule_groups_namespaces.insert(req.name, ns);

    rest_json::created(resp)
}

async fn list_rule_groups_namespaces(
    State(state): State<Arc<AmpState>>,
    Path(workspace_id): Path<String>,
) -> Response {
    let workspace = match state.workspaces.get(&workspace_id) {
        Some(w) => w,
        None => {
            return rest_json::error_response(&LawsError::NotFound(format!(
                "Workspace not found: {workspace_id}"
            )));
        }
    };

    let namespaces: Vec<Value> = workspace
        .rule_groups_namespaces
        .iter()
        .map(|entry| {
            let ns = entry.value();
            json!({
                "name": ns.name,
                "arn": ns.arn,
                "status": { "statusCode": ns.status },
                "createdAt": ns.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "ruleGroupsNamespaces": namespaces }))
}
