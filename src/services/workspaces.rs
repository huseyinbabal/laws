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
pub struct Workspace {
    pub workspace_id: String,
    pub directory_id: String,
    pub user_name: String,
    pub bundle_id: String,
    pub state: String,
    pub ip_address: String,
}

#[derive(Debug, Clone)]
pub struct WorkspaceDirectory {
    pub directory_id: String,
    pub directory_name: String,
    pub directory_type: String,
    pub state: String,
    pub alias: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct WorkSpacesState {
    pub workspaces: DashMap<String, Workspace>,
    pub directories: DashMap<String, WorkspaceDirectory>,
}

impl Default for WorkSpacesState {
    fn default() -> Self {
        Self {
            workspaces: DashMap::new(),
            directories: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &WorkSpacesState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("WorkspacesService.")
        .unwrap_or(target);

    let result = match action {
        "CreateWorkspaces" => create_workspaces(state, payload),
        "TerminateWorkspaces" => terminate_workspaces(state, payload),
        "DescribeWorkspaces" => describe_workspaces(state, payload),
        "StartWorkspaces" => start_workspaces(state, payload),
        "StopWorkspaces" => stop_workspaces(state, payload),
        "RegisterWorkspaceDirectory" => register_workspace_directory(state, payload),
        "DescribeWorkspaceDirectories" => describe_workspace_directories(state),
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

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_workspaces(
    state: &WorkSpacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let workspaces_arr = payload["Workspaces"]
        .as_array()
        .ok_or_else(|| LawsError::InvalidRequest("Workspaces array is required".to_string()))?;

    let mut pending = Vec::new();
    let mut failed = Vec::new();

    for ws in workspaces_arr {
        let directory_id = match ws["DirectoryId"].as_str() {
            Some(d) => d.to_string(),
            None => {
                failed.push(json!({
                    "ErrorCode": "InvalidParameterValuesException",
                    "ErrorMessage": "Missing DirectoryId",
                }));
                continue;
            }
        };

        let user_name = match ws["UserName"].as_str() {
            Some(u) => u.to_string(),
            None => {
                failed.push(json!({
                    "ErrorCode": "InvalidParameterValuesException",
                    "ErrorMessage": "Missing UserName",
                }));
                continue;
            }
        };

        let bundle_id = ws["BundleId"]
            .as_str()
            .unwrap_or("wsb-default")
            .to_string();

        let workspace_id = format!("ws-{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..12]);

        let workspace = Workspace {
            workspace_id: workspace_id.clone(),
            directory_id: directory_id.clone(),
            user_name: user_name.clone(),
            bundle_id: bundle_id.clone(),
            state: "AVAILABLE".to_string(),
            ip_address: "10.0.0.1".to_string(),
        };

        pending.push(json!({
            "WorkspaceId": workspace_id,
            "DirectoryId": directory_id,
            "UserName": user_name,
            "BundleId": bundle_id,
            "State": "PENDING",
        }));

        state.workspaces.insert(workspace_id, workspace);
    }

    Ok(json_response(json!({
        "PendingRequests": pending,
        "FailedRequests": failed,
    })))
}

fn terminate_workspaces(
    state: &WorkSpacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let requests = payload["TerminateWorkspaceRequests"]
        .as_array()
        .ok_or_else(|| {
            LawsError::InvalidRequest("TerminateWorkspaceRequests is required".to_string())
        })?;

    let mut failed = Vec::new();

    for req in requests {
        let workspace_id = match req["WorkspaceId"].as_str() {
            Some(id) => id,
            None => continue,
        };

        if state.workspaces.remove(workspace_id).is_none() {
            failed.push(json!({
                "WorkspaceId": workspace_id,
                "ErrorCode": "ResourceNotFoundException",
                "ErrorMessage": format!("Workspace '{}' not found", workspace_id),
            }));
        }
    }

    Ok(json_response(json!({
        "FailedRequests": failed,
    })))
}

fn describe_workspaces(
    state: &WorkSpacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let workspace_ids = payload["WorkspaceIds"]
        .as_array();

    let workspaces: Vec<Value> = state
        .workspaces
        .iter()
        .filter(|entry| {
            match workspace_ids {
                Some(ids) => ids.iter().any(|id| id.as_str() == Some(entry.key().as_str())),
                None => true,
            }
        })
        .map(|entry| {
            let ws = entry.value();
            json!({
                "WorkspaceId": ws.workspace_id,
                "DirectoryId": ws.directory_id,
                "UserName": ws.user_name,
                "BundleId": ws.bundle_id,
                "State": ws.state,
                "IpAddress": ws.ip_address,
            })
        })
        .collect();

    Ok(json_response(json!({ "Workspaces": workspaces })))
}

fn start_workspaces(
    state: &WorkSpacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let requests = payload["StartWorkspaceRequests"]
        .as_array()
        .ok_or_else(|| {
            LawsError::InvalidRequest("StartWorkspaceRequests is required".to_string())
        })?;

    let mut failed = Vec::new();

    for req in requests {
        let workspace_id = match req["WorkspaceId"].as_str() {
            Some(id) => id,
            None => continue,
        };

        match state.workspaces.get_mut(workspace_id) {
            Some(mut ws) => {
                ws.state = "AVAILABLE".to_string();
            }
            None => {
                failed.push(json!({
                    "WorkspaceId": workspace_id,
                    "ErrorCode": "ResourceNotFoundException",
                    "ErrorMessage": format!("Workspace '{}' not found", workspace_id),
                }));
            }
        }
    }

    Ok(json_response(json!({
        "FailedRequests": failed,
    })))
}

fn stop_workspaces(
    state: &WorkSpacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let requests = payload["StopWorkspaceRequests"]
        .as_array()
        .ok_or_else(|| {
            LawsError::InvalidRequest("StopWorkspaceRequests is required".to_string())
        })?;

    let mut failed = Vec::new();

    for req in requests {
        let workspace_id = match req["WorkspaceId"].as_str() {
            Some(id) => id,
            None => continue,
        };

        match state.workspaces.get_mut(workspace_id) {
            Some(mut ws) => {
                ws.state = "STOPPED".to_string();
            }
            None => {
                failed.push(json!({
                    "WorkspaceId": workspace_id,
                    "ErrorCode": "ResourceNotFoundException",
                    "ErrorMessage": format!("Workspace '{}' not found", workspace_id),
                }));
            }
        }
    }

    Ok(json_response(json!({
        "FailedRequests": failed,
    })))
}

fn register_workspace_directory(
    state: &WorkSpacesState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let directory_id = payload["DirectoryId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DirectoryId is required".to_string()))?
        .to_string();

    if state.directories.contains_key(&directory_id) {
        return Err(LawsError::AlreadyExists(format!(
            "Directory '{}' already registered",
            directory_id
        )));
    }

    let directory = WorkspaceDirectory {
        directory_id: directory_id.clone(),
        directory_name: payload["DirectoryName"]
            .as_str()
            .unwrap_or(&directory_id)
            .to_string(),
        directory_type: "SIMPLE_AD".to_string(),
        state: "REGISTERED".to_string(),
        alias: directory_id.clone(),
    };

    state.directories.insert(directory_id, directory);

    Ok(json_response(json!({})))
}

fn describe_workspace_directories(
    state: &WorkSpacesState,
) -> Result<Response, LawsError> {
    let directories: Vec<Value> = state
        .directories
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "DirectoryId": d.directory_id,
                "DirectoryName": d.directory_name,
                "DirectoryType": d.directory_type,
                "State": d.state,
                "Alias": d.alias,
            })
        })
        .collect();

    Ok(json_response(json!({ "Directories": directories })))
}
