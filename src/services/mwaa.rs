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
pub struct Environment {
    pub name: String,
    pub arn: String,
    pub status: String,
    pub execution_role_arn: String,
    pub source_bucket_arn: String,
    pub dag_s3_path: String,
    pub airflow_version: String,
    pub environment_class: String,
    pub webserver_url: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct MwaaState {
    pub environments: DashMap<String, Environment>,
}

impl Default for MwaaState {
    fn default() -> Self {
        Self {
            environments: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<MwaaState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/environments",
            axum::routing::get(list_environments),
        )
        .route(
            "/environments/{name}",
            axum::routing::put(create_environment)
                .get(get_environment)
                .delete(delete_environment)
                .patch(update_environment),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn environment_to_json(env: &Environment) -> Value {
    json!({
        "Name": env.name,
        "Arn": env.arn,
        "Status": env.status,
        "ExecutionRoleArn": env.execution_role_arn,
        "SourceBucketArn": env.source_bucket_arn,
        "DagS3Path": env.dag_s3_path,
        "AirflowVersion": env.airflow_version,
        "EnvironmentClass": env.environment_class,
        "WebserverUrl": env.webserver_url,
        "CreatedAt": env.created_at,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_environment(
    State(state): State<Arc<MwaaState>>,
    Path(name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let arn = format!(
            "arn:aws:airflow:{REGION}:{ACCOUNT_ID}:environment/{name}"
        );

        let execution_role_arn = payload["ExecutionRoleArn"]
            .as_str()
            .unwrap_or("arn:aws:iam::000000000000:role/mwaa-role")
            .to_string();

        let source_bucket_arn = payload["SourceBucketArn"]
            .as_str()
            .unwrap_or("arn:aws:s3:::mwaa-bucket")
            .to_string();

        let dag_s3_path = payload["DagS3Path"]
            .as_str()
            .unwrap_or("dags/")
            .to_string();

        let airflow_version = payload["AirflowVersion"]
            .as_str()
            .unwrap_or("2.5.1")
            .to_string();

        let environment_class = payload["EnvironmentClass"]
            .as_str()
            .unwrap_or("mw1.small")
            .to_string();

        let webserver_url = format!(
            "{}.{}.airflow.amazonaws.com",
            uuid::Uuid::new_v4().to_string()[..8].to_string(),
            REGION,
        );

        let created_at = chrono::Utc::now().to_rfc3339();

        let env = Environment {
            name: name.clone(),
            arn: arn.clone(),
            status: "CREATING".to_string(),
            execution_role_arn,
            source_bucket_arn,
            dag_s3_path,
            airflow_version,
            environment_class,
            webserver_url,
            created_at,
        };

        state.environments.insert(name, env);

        Ok(rest_json::ok(json!({ "Arn": arn })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn get_environment(
    State(state): State<Arc<MwaaState>>,
    Path(name): Path<String>,
) -> Response {
    match state.environments.get(&name) {
        Some(env) => rest_json::ok(json!({
            "Environment": environment_to_json(env.value())
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Environment '{}' not found",
            name
        ))),
    }
}

async fn list_environments(
    State(state): State<Arc<MwaaState>>,
) -> Response {
    let envs: Vec<String> = state
        .environments
        .iter()
        .map(|entry| entry.value().name.clone())
        .collect();

    rest_json::ok(json!({ "Environments": envs }))
}

async fn delete_environment(
    State(state): State<Arc<MwaaState>>,
    Path(name): Path<String>,
) -> Response {
    match state.environments.remove(&name) {
        Some(_) => rest_json::ok(json!({})),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Environment '{}' not found",
            name
        ))),
    }
}

async fn update_environment(
    State(state): State<Arc<MwaaState>>,
    Path(name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let mut env = state
            .environments
            .get_mut(&name)
            .ok_or_else(|| LawsError::NotFound(format!("Environment '{}' not found", name)))?;

        if let Some(v) = payload["AirflowVersion"].as_str() {
            env.airflow_version = v.to_string();
        }
        if let Some(v) = payload["EnvironmentClass"].as_str() {
            env.environment_class = v.to_string();
        }
        if let Some(v) = payload["ExecutionRoleArn"].as_str() {
            env.execution_role_arn = v.to_string();
        }
        env.status = "UPDATING".to_string();

        Ok(rest_json::ok(json!({ "Arn": env.arn })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}
