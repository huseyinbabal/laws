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
pub struct Container {
    pub name: String,
    pub arn: String,
    pub endpoint: String,
    pub status: String,
    pub creation_time: String,
    pub policy: Option<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct MediaStoreState {
    pub containers: DashMap<String, Container>,
}

impl Default for MediaStoreState {
    fn default() -> Self {
        Self {
            containers: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &MediaStoreState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("MediaStore_20170901.")
        .unwrap_or(target);

    let result = match action {
        "CreateContainer" => create_container(state, payload),
        "DeleteContainer" => delete_container(state, payload),
        "DescribeContainer" => describe_container(state, payload),
        "ListContainers" => list_containers(state),
        "PutContainerPolicy" => put_container_policy(state, payload),
        "GetContainerPolicy" => get_container_policy(state, payload),
        other => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            other
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

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_container(state: &MediaStoreState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["ContainerName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ContainerName".into()))?
        .to_string();

    if state.containers.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Container '{}' already exists",
            name
        )));
    }

    let arn = format!("arn:aws:mediastore:{REGION}:{ACCOUNT_ID}:container/{name}");
    let endpoint = format!(
        "https://{}.mediastore.{REGION}.amazonaws.com",
        &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]
    );
    let now = chrono::Utc::now().to_rfc3339();

    let container = Container {
        name: name.clone(),
        arn: arn.clone(),
        endpoint: endpoint.clone(),
        status: "ACTIVE".into(),
        creation_time: now.clone(),
        policy: None,
    };

    state.containers.insert(name.clone(), container);

    Ok(json_response(json!({
        "Container": {
            "Name": name,
            "ARN": arn,
            "Endpoint": endpoint,
            "Status": "CREATING",
            "CreationTime": now
        }
    })))
}

fn delete_container(state: &MediaStoreState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["ContainerName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ContainerName".into()))?;

    state
        .containers
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Container '{}' not found", name)))?;

    Ok(json_response(json!({})))
}

fn describe_container(state: &MediaStoreState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["ContainerName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ContainerName".into()))?;

    let c = state
        .containers
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Container '{}' not found", name)))?;

    Ok(json_response(json!({
        "Container": {
            "Name": c.name,
            "ARN": c.arn,
            "Endpoint": c.endpoint,
            "Status": c.status,
            "CreationTime": c.creation_time
        }
    })))
}

fn list_containers(state: &MediaStoreState) -> Result<Response, LawsError> {
    let containers: Vec<Value> = state
        .containers
        .iter()
        .map(|e| {
            let c = e.value();
            json!({
                "Name": c.name,
                "ARN": c.arn,
                "Endpoint": c.endpoint,
                "Status": c.status,
                "CreationTime": c.creation_time
            })
        })
        .collect();

    Ok(json_response(json!({
        "Containers": containers
    })))
}

fn put_container_policy(state: &MediaStoreState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["ContainerName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ContainerName".into()))?;

    let policy = payload["Policy"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Policy".into()))?
        .to_string();

    let mut container = state
        .containers
        .get_mut(name)
        .ok_or_else(|| LawsError::NotFound(format!("Container '{}' not found", name)))?;

    container.policy = Some(policy);

    Ok(json_response(json!({})))
}

fn get_container_policy(state: &MediaStoreState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["ContainerName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ContainerName".into()))?;

    let container = state
        .containers
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Container '{}' not found", name)))?;

    match &container.policy {
        Some(policy) => Ok(json_response(json!({
            "Policy": policy
        }))),
        None => Err(LawsError::NotFound(format!(
            "No policy for container '{}'",
            name
        ))),
    }
}
