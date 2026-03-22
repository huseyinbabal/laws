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
pub struct Directory {
    pub directory_id: String,
    pub name: String,
    pub short_name: String,
    pub size: String,
    pub directory_type: String,
    pub stage: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DirectoryServiceState {
    pub directories: DashMap<String, Directory>,
}

impl Default for DirectoryServiceState {
    fn default() -> Self {
        Self {
            directories: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &DirectoryServiceState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("DirectoryService_20150416.")
        .unwrap_or(target);

    let result = match action {
        "CreateDirectory" => create_directory(state, payload),
        "DeleteDirectory" => delete_directory(state, payload),
        "DescribeDirectories" => describe_directories(state, payload),
        "ConnectDirectory" => connect_directory(state, payload),
        "CreateMicrosoftAD" => create_microsoft_ad(state, payload),
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

fn directory_to_json(d: &Directory) -> Value {
    json!({
        "DirectoryId": d.directory_id,
        "Name": d.name,
        "ShortName": d.short_name,
        "Size": d.size,
        "Type": d.directory_type,
        "Stage": d.stage,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_directory(state: &DirectoryServiceState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let short_name = payload["ShortName"].as_str().unwrap_or("").to_string();

    let size = payload["Size"].as_str().unwrap_or("Small").to_string();

    let dir_id = format!("d-{}", &uuid::Uuid::new_v4().to_string()[..10]);

    let directory = Directory {
        directory_id: dir_id.clone(),
        name,
        short_name,
        size,
        directory_type: "SimpleAD".to_string(),
        stage: "Active".to_string(),
    };

    state.directories.insert(dir_id.clone(), directory);
    Ok(json_response(json!({ "DirectoryId": dir_id })))
}

fn delete_directory(state: &DirectoryServiceState, payload: &Value) -> Result<Response, LawsError> {
    let directory_id = payload["DirectoryId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DirectoryId".into()))?;

    state
        .directories
        .remove(directory_id)
        .ok_or_else(|| LawsError::NotFound(format!("Directory not found: {directory_id}")))?;

    Ok(json_response(json!({ "DirectoryId": directory_id })))
}

fn describe_directories(
    state: &DirectoryServiceState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let directories: Vec<Value> = if let Some(ids) = payload["DirectoryIds"].as_array() {
        ids.iter()
            .filter_map(|id| id.as_str())
            .filter_map(|id| {
                state
                    .directories
                    .get(id)
                    .map(|d| directory_to_json(d.value()))
            })
            .collect()
    } else {
        state
            .directories
            .iter()
            .map(|entry| directory_to_json(entry.value()))
            .collect()
    };

    Ok(json_response(
        json!({ "DirectoryDescriptions": directories }),
    ))
}

fn connect_directory(
    state: &DirectoryServiceState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let short_name = payload["ShortName"].as_str().unwrap_or("").to_string();

    let size = payload["Size"].as_str().unwrap_or("Small").to_string();

    let dir_id = format!("d-{}", &uuid::Uuid::new_v4().to_string()[..10]);

    let directory = Directory {
        directory_id: dir_id.clone(),
        name,
        short_name,
        size,
        directory_type: "ADConnector".to_string(),
        stage: "Active".to_string(),
    };

    state.directories.insert(dir_id.clone(), directory);
    Ok(json_response(json!({ "DirectoryId": dir_id })))
}

fn create_microsoft_ad(
    state: &DirectoryServiceState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let short_name = payload["ShortName"].as_str().unwrap_or("").to_string();

    let edition = payload["Edition"]
        .as_str()
        .unwrap_or("Standard")
        .to_string();

    let dir_id = format!("d-{}", &uuid::Uuid::new_v4().to_string()[..10]);

    let directory = Directory {
        directory_id: dir_id.clone(),
        name,
        short_name,
        size: edition,
        directory_type: "MicrosoftAD".to_string(),
        stage: "Active".to_string(),
    };

    state.directories.insert(dir_id.clone(), directory);
    Ok(json_response(json!({ "DirectoryId": dir_id })))
}
