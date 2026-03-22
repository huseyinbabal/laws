use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::LawsError;
use crate::storage::mem::MemoryStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsmParameter {
    pub name: String,
    pub value: String,
    pub param_type: String,
    pub version: u64,
    pub last_modified_date: f64,
    pub arn: String,
}

pub struct SsmState {
    pub parameters: MemoryStore<SsmParameter>,
    pub account_id: String,
    pub region: String,
}

impl Default for SsmState {
    fn default() -> Self {
        Self {
            parameters: MemoryStore::new(),
            account_id: "000000000000".to_string(),
            region: "us-east-1".to_string(),
        }
    }
}

pub fn router(state: Arc<SsmState>) -> Router {
    Router::new()
        .route("/", post(handle_action))
        .with_state(state)
}

pub async fn handle_request(
    state: &SsmState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("AmazonSSM.").unwrap_or(target);

    let result = match action {
        "PutParameter" => put_parameter(state, payload).await,
        "GetParameter" => get_parameter(state, payload).await,
        "GetParametersByPath" => get_parameters_by_path(state, payload).await,
        "DeleteParameter" => delete_parameter(state, payload).await,
        "DescribeParameters" => describe_parameters(state).await,
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

async fn handle_action(
    State(state): State<Arc<SsmState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Response, LawsError> {
    let target = headers
        .get("X-Amz-Target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let action = target.strip_prefix("AmazonSSM.").unwrap_or(target);

    let payload: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

    match action {
        "PutParameter" => put_parameter(&state, &payload).await,
        "GetParameter" => get_parameter(&state, &payload).await,
        "GetParametersByPath" => get_parameters_by_path(&state, &payload).await,
        "DeleteParameter" => delete_parameter(&state, &payload).await,
        "DescribeParameters" => describe_parameters(&state).await,
        _ => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            action
        ))),
    }
}

fn json_response(body: serde_json::Value) -> Response {
    (
        StatusCode::OK,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn make_arn(state: &SsmState, name: &str) -> String {
    format!(
        "arn:aws:ssm:{}:{}:parameter{}",
        state.region, state.account_id, name
    )
}

fn format_parameter(p: &SsmParameter) -> serde_json::Value {
    serde_json::json!({
        "Name": p.name,
        "Type": p.param_type,
        "Value": p.value,
        "Version": p.version,
        "LastModifiedDate": p.last_modified_date,
        "ARN": p.arn,
    })
}

async fn put_parameter(
    state: &SsmState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;
    let value = payload["Value"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Value is required".to_string()))?;
    let param_type = payload["Type"].as_str().unwrap_or("String");
    let overwrite = payload["Overwrite"].as_bool().unwrap_or(false);

    // Validate type
    match param_type {
        "String" | "SecureString" | "StringList" => {}
        _ => {
            return Err(LawsError::InvalidRequest(format!(
                "Invalid Type: {}. Must be String, SecureString, or StringList",
                param_type
            )));
        }
    }

    let now = chrono::Utc::now().timestamp() as f64;

    let version = if let Some(existing) = state.parameters.get(name) {
        if !overwrite {
            return Err(LawsError::AlreadyExists(format!(
                "Parameter '{}' already exists. Use Overwrite to update.",
                name
            )));
        }
        existing.version + 1
    } else {
        1
    };

    let arn = make_arn(state, name);

    let param = SsmParameter {
        name: name.to_string(),
        value: value.to_string(),
        param_type: param_type.to_string(),
        version,
        last_modified_date: now,
        arn,
    };

    state.parameters.insert(name.to_string(), param);

    Ok(json_response(serde_json::json!({
        "Version": version,
        "Tier": "Standard",
    })))
}

async fn get_parameter(
    state: &SsmState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    let param = state
        .parameters
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Parameter '{}' not found", name)))?;

    Ok(json_response(serde_json::json!({
        "Parameter": format_parameter(&param),
    })))
}

async fn get_parameters_by_path(
    state: &SsmState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let path = payload["Path"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Path is required".to_string()))?;
    let recursive = payload["Recursive"].as_bool().unwrap_or(false);

    let prefix = if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{}/", path)
    };

    let params: Vec<serde_json::Value> = state
        .parameters
        .list_values()
        .iter()
        .filter(|p| {
            if !p.name.starts_with(&prefix) {
                return false;
            }
            if !recursive {
                // Non-recursive: only direct children (no additional '/' after the prefix)
                let remainder = &p.name[prefix.len()..];
                !remainder.contains('/')
            } else {
                true
            }
        })
        .map(|p| format_parameter(p))
        .collect();

    Ok(json_response(serde_json::json!({
        "Parameters": params,
    })))
}

async fn delete_parameter(
    state: &SsmState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    state
        .parameters
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Parameter '{}' not found", name)))?;

    Ok(json_response(serde_json::json!({})))
}

async fn describe_parameters(state: &SsmState) -> Result<Response, LawsError> {
    let params: Vec<serde_json::Value> = state
        .parameters
        .list_values()
        .iter()
        .map(|p| {
            serde_json::json!({
                "Name": p.name,
                "Type": p.param_type,
                "Version": p.version,
                "LastModifiedDate": p.last_modified_date,
                "Tier": "Standard",
                "Description": "",
            })
        })
        .collect();

    Ok(json_response(serde_json::json!({
        "Parameters": params,
    })))
}
