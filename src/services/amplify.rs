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
pub struct AmplifyApp {
    pub app_id: String,
    pub app_arn: String,
    pub name: String,
    pub description: String,
    pub repository: String,
    pub platform: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AmplifyBranch {
    pub app_id: String,
    pub branch_name: String,
    pub branch_arn: String,
    pub description: String,
    pub stage: String,
    pub display_name: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AmplifyState {
    pub apps: DashMap<String, AmplifyApp>,
    pub branches: DashMap<String, AmplifyBranch>,
}

impl Default for AmplifyState {
    fn default() -> Self {
        Self {
            apps: DashMap::new(),
            branches: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<AmplifyState>) -> axum::Router {
    axum::Router::new()
        .route("/apps", axum::routing::post(create_app).get(list_apps))
        .route("/apps/{id}", axum::routing::get(get_app).delete(delete_app))
        .route(
            "/apps/{id}/branches",
            axum::routing::post(create_branch).get(list_branches),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_id() -> String {
    uuid::Uuid::new_v4().to_string()[..10].to_string()
}

fn app_to_json(app: &AmplifyApp) -> Value {
    json!({
        "appId": app.app_id,
        "appArn": app.app_arn,
        "name": app.name,
        "description": app.description,
        "repository": app.repository,
        "platform": app.platform,
        "createTime": app.created_at,
    })
}

fn branch_to_json(branch: &AmplifyBranch) -> Value {
    json!({
        "branchName": branch.branch_name,
        "branchArn": branch.branch_arn,
        "description": branch.description,
        "stage": branch.stage,
        "displayName": branch.display_name,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_app(
    State(state): State<Arc<AmplifyState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing name".into()))?
            .to_string();

        let description = payload["description"].as_str().unwrap_or("").to_string();

        let repository = payload["repository"].as_str().unwrap_or("").to_string();

        let platform = payload["platform"].as_str().unwrap_or("WEB").to_string();

        let app_id = random_id();
        let app_arn = format!("arn:aws:amplify:{REGION}:{ACCOUNT_ID}:apps/{app_id}");
        let created_at = chrono::Utc::now().to_rfc3339();

        let app = AmplifyApp {
            app_id: app_id.clone(),
            app_arn,
            name,
            description,
            repository,
            platform,
            created_at,
        };

        let resp = app_to_json(&app);
        state.apps.insert(app_id, app);

        Ok(rest_json::created(json!({ "app": resp })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_apps(State(state): State<Arc<AmplifyState>>) -> Response {
    let apps: Vec<Value> = state
        .apps
        .iter()
        .map(|entry| app_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "apps": apps }))
}

async fn get_app(State(state): State<Arc<AmplifyState>>, Path(id): Path<String>) -> Response {
    match state.apps.get(&id) {
        Some(app) => rest_json::ok(json!({ "app": app_to_json(app.value()) })),
        None => rest_json::error_response(&LawsError::NotFound(format!("App '{}' not found", id))),
    }
}

async fn delete_app(State(state): State<Arc<AmplifyState>>, Path(id): Path<String>) -> Response {
    match state.apps.remove(&id) {
        Some((_, app)) => {
            // Clean up associated branches
            state.branches.retain(|_, b| b.app_id != id);
            rest_json::ok(json!({ "app": app_to_json(&app) }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!("App '{}' not found", id))),
    }
}

async fn create_branch(
    State(state): State<Arc<AmplifyState>>,
    Path(app_id): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        if !state.apps.contains_key(&app_id) {
            return Err(LawsError::NotFound(format!("App '{}' not found", app_id)));
        }

        let branch_name = payload["branchName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing branchName".into()))?
            .to_string();

        let description = payload["description"].as_str().unwrap_or("").to_string();

        let stage = payload["stage"].as_str().unwrap_or("NONE").to_string();

        let display_name = payload["displayName"]
            .as_str()
            .unwrap_or(&branch_name)
            .to_string();

        let branch_arn =
            format!("arn:aws:amplify:{REGION}:{ACCOUNT_ID}:apps/{app_id}/branches/{branch_name}");

        let branch_key = format!("{}:{}", app_id, branch_name);
        if state.branches.contains_key(&branch_key) {
            return Err(LawsError::AlreadyExists(format!(
                "Branch '{}' already exists",
                branch_name
            )));
        }

        let branch = AmplifyBranch {
            app_id: app_id.clone(),
            branch_name,
            branch_arn,
            description,
            stage,
            display_name,
        };

        let resp = branch_to_json(&branch);
        state.branches.insert(branch_key, branch);

        Ok(rest_json::created(json!({ "branch": resp })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_branches(
    State(state): State<Arc<AmplifyState>>,
    Path(app_id): Path<String>,
) -> Response {
    if !state.apps.contains_key(&app_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "App '{}' not found",
            app_id
        )));
    }

    let branches: Vec<Value> = state
        .branches
        .iter()
        .filter(|entry| entry.app_id == app_id)
        .map(|entry| branch_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "branches": branches }))
}
