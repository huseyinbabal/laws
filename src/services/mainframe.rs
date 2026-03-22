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
pub struct Application {
    pub application_id: String,
    pub application_arn: String,
    pub name: String,
    pub description: String,
    pub engine_type: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub environment_id: String,
    pub environment_arn: String,
    pub name: String,
    pub engine_type: String,
    pub instance_type: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct MainframeState {
    pub applications: DashMap<String, Application>,
    pub environments: DashMap<String, Environment>,
}

impl Default for MainframeState {
    fn default() -> Self {
        Self {
            applications: DashMap::new(),
            environments: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<MainframeState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/applications",
            axum::routing::post(create_application).get(list_applications),
        )
        .route(
            "/applications/{id}",
            axum::routing::get(get_application).delete(delete_application),
        )
        .route(
            "/environments",
            axum::routing::post(create_environment).get(list_environments),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_application(
    State(state): State<Arc<MainframeState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let description = payload
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let engine_type = payload
        .get("engineType")
        .and_then(|v| v.as_str())
        .unwrap_or("microfocus")
        .to_owned();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:m2:{REGION}:{ACCOUNT_ID}:app/{id}");
    let now = chrono::Utc::now().to_rfc3339();

    let app = Application {
        application_id: id.clone(),
        application_arn: arn.clone(),
        name: name.clone(),
        description,
        engine_type,
        status: "Created".into(),
        created_at: now,
    };

    state.applications.insert(id.clone(), app);

    rest_json::created(json!({
        "applicationId": id,
        "applicationArn": arn,
        "applicationVersion": 1
    }))
}

async fn list_applications(
    State(state): State<Arc<MainframeState>>,
) -> Response {
    let apps: Vec<Value> = state
        .applications
        .iter()
        .map(|e| {
            let a = e.value();
            json!({
                "applicationId": a.application_id,
                "applicationArn": a.application_arn,
                "name": a.name,
                "engineType": a.engine_type,
                "status": a.status,
                "creationTime": a.created_at
            })
        })
        .collect();

    rest_json::ok(json!({
        "applications": apps
    }))
}

async fn get_application(
    State(state): State<Arc<MainframeState>>,
    Path(id): Path<String>,
) -> Response {
    match state.applications.get(&id) {
        Some(a) => rest_json::ok(json!({
            "applicationId": a.application_id,
            "applicationArn": a.application_arn,
            "name": a.name,
            "description": a.description,
            "engineType": a.engine_type,
            "status": a.status,
            "creationTime": a.created_at
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Application not found: {id}"
        ))),
    }
}

async fn delete_application(
    State(state): State<Arc<MainframeState>>,
    Path(id): Path<String>,
) -> Response {
    match state.applications.remove(&id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Application not found: {id}"
        ))),
    }
}

async fn create_environment(
    State(state): State<Arc<MainframeState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let engine_type = payload
        .get("engineType")
        .and_then(|v| v.as_str())
        .unwrap_or("microfocus")
        .to_owned();

    let instance_type = payload
        .get("instanceType")
        .and_then(|v| v.as_str())
        .unwrap_or("M2.m5.large")
        .to_owned();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:m2:{REGION}:{ACCOUNT_ID}:env/{id}");
    let now = chrono::Utc::now().to_rfc3339();

    let env = Environment {
        environment_id: id.clone(),
        environment_arn: arn.clone(),
        name,
        engine_type,
        instance_type,
        status: "Available".into(),
        created_at: now,
    };

    state.environments.insert(id.clone(), env);

    rest_json::created(json!({
        "environmentId": id
    }))
}

async fn list_environments(
    State(state): State<Arc<MainframeState>>,
) -> Response {
    let envs: Vec<Value> = state
        .environments
        .iter()
        .map(|e| {
            let env = e.value();
            json!({
                "environmentId": env.environment_id,
                "environmentArn": env.environment_arn,
                "name": env.name,
                "engineType": env.engine_type,
                "instanceType": env.instance_type,
                "status": env.status,
                "creationTime": env.created_at
            })
        })
        .collect();

    rest_json::ok(json!({
        "environments": envs
    }))
}
