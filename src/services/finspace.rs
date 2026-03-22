use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, post};
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
pub struct Environment {
    pub environment_id: String,
    pub name: String,
    pub arn: String,
    pub status: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct KxDatabase {
    pub database_name: String,
    pub environment_id: String,
    pub database_arn: String,
    pub description: String,
    pub created_timestamp: String,
    pub last_modified_timestamp: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct FinSpaceState {
    pub environments: DashMap<String, Environment>,
    pub kx_databases: DashMap<String, KxDatabase>,
}

impl Default for FinSpaceState {
    fn default() -> Self {
        Self {
            environments: DashMap::new(),
            kx_databases: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<FinSpaceState>) -> axum::Router {
    axum::Router::new()
        .route("/environment", post(create_environment))
        .route("/environment", get(list_environments))
        .route(
            "/environment/{environment_id}",
            get(get_environment).delete(delete_environment),
        )
        .route(
            "/kx/environments/{environment_id}/databases",
            post(create_kx_database).get(list_kx_databases),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateEnvironmentRequest {
    name: String,
    #[serde(default)]
    description: Option<String>,
}

async fn create_environment(
    State(state): State<Arc<FinSpaceState>>,
    Json(req): Json<CreateEnvironmentRequest>,
) -> Response {
    let environment_id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:finspace:{REGION}:{ACCOUNT_ID}:environment/{environment_id}"
    );
    let now = Utc::now().to_rfc3339();

    let env = Environment {
        environment_id: environment_id.clone(),
        name: req.name.clone(),
        arn: arn.clone(),
        status: "CREATE_REQUESTED".to_string(),
        description: req.description.unwrap_or_default(),
        created_at: now,
    };

    state.environments.insert(environment_id.clone(), env);

    rest_json::created(json!({
        "environmentId": environment_id,
        "environmentArn": arn,
    }))
}

async fn list_environments(State(state): State<Arc<FinSpaceState>>) -> Response {
    let envs: Vec<Value> = state
        .environments
        .iter()
        .map(|entry| {
            let e = entry.value();
            json!({
                "environmentId": e.environment_id,
                "name": e.name,
                "environmentArn": e.arn,
                "status": e.status,
            })
        })
        .collect();

    rest_json::ok(json!({ "environments": envs }))
}

async fn get_environment(
    State(state): State<Arc<FinSpaceState>>,
    Path(environment_id): Path<String>,
) -> Response {
    match state.environments.get(&environment_id) {
        Some(e) => rest_json::ok(json!({
            "environment": {
                "environmentId": e.environment_id,
                "name": e.name,
                "environmentArn": e.arn,
                "status": e.status,
                "description": e.description,
                "createdAt": e.created_at,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Environment not found: {environment_id}"
        ))),
    }
}

async fn delete_environment(
    State(state): State<Arc<FinSpaceState>>,
    Path(environment_id): Path<String>,
) -> Response {
    match state.environments.remove(&environment_id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Environment not found: {environment_id}"
        ))),
    }
}

#[derive(Deserialize)]
struct CreateKxDatabaseRequest {
    #[serde(alias = "databaseName")]
    database_name: String,
    #[serde(default)]
    description: Option<String>,
}

async fn create_kx_database(
    State(state): State<Arc<FinSpaceState>>,
    Path(environment_id): Path<String>,
    Json(req): Json<CreateKxDatabaseRequest>,
) -> Response {
    if !state.environments.contains_key(&environment_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Environment not found: {environment_id}"
        )));
    }

    let key = format!("{environment_id}:{}", req.database_name);
    if state.kx_databases.contains_key(&key) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "Database already exists: {}",
            req.database_name
        )));
    }

    let database_arn = format!(
        "arn:aws:finspace:{REGION}:{ACCOUNT_ID}:kxEnvironment/{environment_id}/kxDatabase/{db}",
        db = req.database_name
    );
    let now = Utc::now().to_rfc3339();

    let db = KxDatabase {
        database_name: req.database_name.clone(),
        environment_id: environment_id.clone(),
        database_arn: database_arn.clone(),
        description: req.description.unwrap_or_default(),
        created_timestamp: now.clone(),
        last_modified_timestamp: now.clone(),
    };

    state.kx_databases.insert(key, db);

    rest_json::created(json!({
        "databaseName": req.database_name,
        "databaseArn": database_arn,
        "environmentId": environment_id,
        "createdTimestamp": now,
    }))
}

async fn list_kx_databases(
    State(state): State<Arc<FinSpaceState>>,
    Path(environment_id): Path<String>,
) -> Response {
    if !state.environments.contains_key(&environment_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Environment not found: {environment_id}"
        )));
    }

    let databases: Vec<Value> = state
        .kx_databases
        .iter()
        .filter(|entry| entry.value().environment_id == environment_id)
        .map(|entry| {
            let db = entry.value();
            json!({
                "databaseName": db.database_name,
                "databaseArn": db.database_arn,
                "createdTimestamp": db.created_timestamp,
                "lastModifiedTimestamp": db.last_modified_timestamp,
            })
        })
        .collect();

    rest_json::ok(json!({ "kxDatabases": databases }))
}
