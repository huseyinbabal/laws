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
pub struct TransferServer {
    pub server_id: String,
    pub arn: String,
    pub endpoint_type: String,
    pub identity_provider_type: String,
    pub state: String,
    pub protocols: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TransferUser {
    pub server_id: String,
    pub user_name: String,
    pub arn: String,
    pub role: String,
    pub home_directory: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct TransferState {
    pub servers: DashMap<String, TransferServer>,
    pub users: DashMap<String, TransferUser>,
}

impl Default for TransferState {
    fn default() -> Self {
        Self {
            servers: DashMap::new(),
            users: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &TransferState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("TransferService.").unwrap_or(target);

    let result = match action {
        "CreateServer" => create_server(state, payload),
        "DeleteServer" => delete_server(state, payload),
        "DescribeServer" => describe_server(state, payload),
        "ListServers" => list_servers(state),
        "CreateUser" => create_user(state, payload),
        "DeleteUser" => delete_user(state, payload),
        "ListUsers" => list_users(state, payload),
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

fn create_server(state: &TransferState, payload: &Value) -> Result<Response, LawsError> {
    let server_id = format!(
        "s-{}",
        uuid::Uuid::new_v4().to_string().replace("-", "")[..17].to_string()
    );

    let endpoint_type = payload["EndpointType"]
        .as_str()
        .unwrap_or("PUBLIC")
        .to_string();

    let identity_provider_type = payload["IdentityProviderType"]
        .as_str()
        .unwrap_or("SERVICE_MANAGED")
        .to_string();

    let protocols: Vec<String> = payload["Protocols"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_else(|| vec!["SFTP".to_string()]);

    let arn = format!("arn:aws:transfer:{REGION}:{ACCOUNT_ID}:server/{server_id}");

    let server = TransferServer {
        server_id: server_id.clone(),
        arn: arn.clone(),
        endpoint_type,
        identity_provider_type,
        state: "ONLINE".to_string(),
        protocols,
    };

    state.servers.insert(server_id.clone(), server);

    Ok(json_response(json!({
        "ServerId": server_id,
    })))
}

fn delete_server(state: &TransferState, payload: &Value) -> Result<Response, LawsError> {
    let server_id = payload["ServerId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ServerId is required".to_string()))?;

    state
        .servers
        .remove(server_id)
        .ok_or_else(|| LawsError::NotFound(format!("Server '{}' not found", server_id)))?;

    // Remove associated users
    state.users.retain(|_, u| u.server_id != server_id);

    Ok(json_response(json!({})))
}

fn describe_server(state: &TransferState, payload: &Value) -> Result<Response, LawsError> {
    let server_id = payload["ServerId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ServerId is required".to_string()))?;

    let server = state
        .servers
        .get(server_id)
        .ok_or_else(|| LawsError::NotFound(format!("Server '{}' not found", server_id)))?;

    Ok(json_response(json!({
        "Server": {
            "ServerId": server.server_id,
            "Arn": server.arn,
            "EndpointType": server.endpoint_type,
            "IdentityProviderType": server.identity_provider_type,
            "State": server.state,
            "Protocols": server.protocols,
            "UserCount": state.users.iter().filter(|u| u.server_id == server_id).count(),
        }
    })))
}

fn list_servers(state: &TransferState) -> Result<Response, LawsError> {
    let servers: Vec<Value> = state
        .servers
        .iter()
        .map(|entry| {
            let s = entry.value();
            json!({
                "ServerId": s.server_id,
                "Arn": s.arn,
                "EndpointType": s.endpoint_type,
                "IdentityProviderType": s.identity_provider_type,
                "State": s.state,
            })
        })
        .collect();

    Ok(json_response(json!({ "Servers": servers })))
}

fn create_user(state: &TransferState, payload: &Value) -> Result<Response, LawsError> {
    let server_id = payload["ServerId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ServerId is required".to_string()))?
        .to_string();

    let user_name = payload["UserName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("UserName is required".to_string()))?
        .to_string();

    let role = payload["Role"].as_str().unwrap_or("").to_string();

    let home_directory = payload["HomeDirectory"].as_str().unwrap_or("/").to_string();

    if !state.servers.contains_key(&server_id) {
        return Err(LawsError::NotFound(format!(
            "Server '{}' not found",
            server_id
        )));
    }

    let user_key = format!("{}:{}", server_id, user_name);
    if state.users.contains_key(&user_key) {
        return Err(LawsError::AlreadyExists(format!(
            "User '{}' already exists",
            user_name
        )));
    }

    let arn = format!("arn:aws:transfer:{REGION}:{ACCOUNT_ID}:user/{server_id}/{user_name}");

    let user = TransferUser {
        server_id: server_id.clone(),
        user_name: user_name.clone(),
        arn,
        role,
        home_directory,
    };

    state.users.insert(user_key, user);

    Ok(json_response(json!({
        "ServerId": server_id,
        "UserName": user_name,
    })))
}

fn delete_user(state: &TransferState, payload: &Value) -> Result<Response, LawsError> {
    let server_id = payload["ServerId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ServerId is required".to_string()))?;

    let user_name = payload["UserName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("UserName is required".to_string()))?;

    let user_key = format!("{}:{}", server_id, user_name);
    state
        .users
        .remove(&user_key)
        .ok_or_else(|| LawsError::NotFound(format!("User '{}' not found", user_name)))?;

    Ok(json_response(json!({})))
}

fn list_users(state: &TransferState, payload: &Value) -> Result<Response, LawsError> {
    let server_id = payload["ServerId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ServerId is required".to_string()))?;

    if !state.servers.contains_key(server_id) {
        return Err(LawsError::NotFound(format!(
            "Server '{}' not found",
            server_id
        )));
    }

    let users: Vec<Value> = state
        .users
        .iter()
        .filter(|entry| entry.server_id == server_id)
        .map(|entry| {
            let u = entry.value();
            json!({
                "UserName": u.user_name,
                "Arn": u.arn,
                "HomeDirectory": u.home_directory,
                "Role": u.role,
            })
        })
        .collect();

    Ok(json_response(json!({
        "ServerId": server_id,
        "Users": users,
    })))
}
