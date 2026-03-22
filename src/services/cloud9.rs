use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

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
    pub id: String,
    pub arn: String,
    pub name: String,
    pub description: String,
    pub environment_type: String,
    pub instance_type: String,
    pub status: String,
    pub owner_arn: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct Cloud9State {
    pub environments: DashMap<String, Environment>,
}

impl Default for Cloud9State {
    fn default() -> Self {
        Self {
            environments: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &Cloud9State,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AWSCloud9WorkspaceManagementService.")
        .unwrap_or(target);

    let result = match action {
        "CreateEnvironmentEC2" => create_environment_ec2(state, payload),
        "DeleteEnvironment" => delete_environment(state, payload),
        "DescribeEnvironments" => describe_environments(state, payload),
        "ListEnvironments" => list_environments(state),
        "UpdateEnvironment" => update_environment(state, payload),
        _ => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            action
        ))),
    };

    match result {
        Ok(resp) => resp,
        Err(e) => e.into_response(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn json_response(body: Value) -> Response {
    (
        StatusCode::OK,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn environment_to_json(env: &Environment) -> Value {
    json!({
        "id": env.id,
        "arn": env.arn,
        "name": env.name,
        "description": env.description,
        "type": env.environment_type,
        "instanceType": env.instance_type,
        "status": env.status,
        "ownerArn": env.owner_arn,
        "createdAt": env.created_at,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_environment_ec2(
    state: &Cloud9State,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("name is required".to_string()))?
        .to_string();

    let instance_type = payload["instanceType"]
        .as_str()
        .unwrap_or("t2.micro")
        .to_string();

    let description = payload["description"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let owner_arn = payload["ownerArn"]
        .as_str()
        .unwrap_or(&format!("arn:aws:iam::{ACCOUNT_ID}:root"))
        .to_string();

    let id = uuid::Uuid::new_v4().to_string().replace('-', "")[..32].to_string();
    let arn = format!("arn:aws:cloud9:{REGION}:{ACCOUNT_ID}:environment:{id}");
    let now = chrono::Utc::now().to_rfc3339();

    let env = Environment {
        id: id.clone(),
        arn,
        name,
        description,
        environment_type: "ec2".to_string(),
        instance_type,
        status: "creating".to_string(),
        owner_arn,
        created_at: now,
    };

    state.environments.insert(id.clone(), env);

    Ok(json_response(json!({ "environmentId": id })))
}

fn delete_environment(state: &Cloud9State, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["environmentId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("environmentId is required".to_string()))?;

    state
        .environments
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Environment '{}' not found", id)))?;

    Ok(json_response(json!({})))
}

fn describe_environments(state: &Cloud9State, payload: &Value) -> Result<Response, LawsError> {
    let env_ids = payload["environmentIds"]
        .as_array()
        .ok_or_else(|| LawsError::InvalidRequest("environmentIds is required".to_string()))?;

    let mut environments = Vec::new();
    for id_val in env_ids {
        let id = id_val.as_str().unwrap_or_default();
        if let Some(env) = state.environments.get(id) {
            environments.push(environment_to_json(env.value()));
        }
    }

    Ok(json_response(json!({ "environments": environments })))
}

fn list_environments(state: &Cloud9State) -> Result<Response, LawsError> {
    let ids: Vec<String> = state
        .environments
        .iter()
        .map(|entry| entry.value().id.clone())
        .collect();

    Ok(json_response(json!({ "environmentIds": ids })))
}

fn update_environment(state: &Cloud9State, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["environmentId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("environmentId is required".to_string()))?;

    let mut env = state
        .environments
        .get_mut(id)
        .ok_or_else(|| LawsError::NotFound(format!("Environment '{}' not found", id)))?;

    if let Some(name) = payload["name"].as_str() {
        env.name = name.to_string();
    }
    if let Some(description) = payload["description"].as_str() {
        env.description = description.to_string();
    }

    Ok(json_response(json!({})))
}
