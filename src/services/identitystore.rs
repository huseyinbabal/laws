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
pub struct User {
    pub user_id: String,
    pub identity_store_id: String,
    pub user_name: String,
    pub display_name: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub group_id: String,
    pub identity_store_id: String,
    pub display_name: String,
    pub description: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct IdentityStoreState {
    pub users: DashMap<String, User>,
    pub groups: DashMap<String, Group>,
}

impl Default for IdentityStoreState {
    fn default() -> Self {
        Self {
            users: DashMap::new(),
            groups: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &IdentityStoreState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("AWSIdentityStore.").unwrap_or(target);

    let result = match action {
        "CreateUser" => create_user(state, payload),
        "GetUserId" => get_user_id(state, payload),
        "DescribeUser" => describe_user(state, payload),
        "ListUsers" => list_users(state, payload),
        "CreateGroup" => create_group(state, payload),
        "ListGroups" => list_groups(state, payload),
        "DescribeGroup" => describe_group(state, payload),
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

fn create_user(state: &IdentityStoreState, payload: &Value) -> Result<Response, LawsError> {
    let identity_store_id = payload["IdentityStoreId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing IdentityStoreId".into()))?
        .to_string();

    let user_name = payload["UserName"].as_str().unwrap_or("user").to_string();

    let display_name = payload["DisplayName"]
        .as_str()
        .unwrap_or(&user_name)
        .to_string();

    let email = payload["Emails"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v["Value"].as_str())
        .unwrap_or("")
        .to_string();

    let user_id = uuid::Uuid::new_v4().to_string();

    let user = User {
        user_id: user_id.clone(),
        identity_store_id: identity_store_id.clone(),
        user_name,
        display_name,
        email,
    };

    state.users.insert(user_id.clone(), user);

    Ok(json_response(json!({
        "UserId": user_id,
        "IdentityStoreId": identity_store_id
    })))
}

fn get_user_id(state: &IdentityStoreState, payload: &Value) -> Result<Response, LawsError> {
    let identity_store_id = payload["IdentityStoreId"].as_str().unwrap_or("");

    let filter_value = payload["AlternateIdentifier"]["UniqueAttribute"]["AttributeValue"]
        .as_str()
        .unwrap_or("");

    let user = state
        .users
        .iter()
        .find(|e| {
            let u = e.value();
            u.identity_store_id == identity_store_id && u.user_name == filter_value
        })
        .map(|e| e.value().clone());

    match user {
        Some(u) => Ok(json_response(json!({
            "UserId": u.user_id,
            "IdentityStoreId": u.identity_store_id
        }))),
        None => Err(LawsError::NotFound("User not found".into())),
    }
}

fn describe_user(state: &IdentityStoreState, payload: &Value) -> Result<Response, LawsError> {
    let user_id = payload["UserId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing UserId".into()))?;

    let user = state
        .users
        .get(user_id)
        .ok_or_else(|| LawsError::NotFound(format!("User '{}' not found", user_id)))?;

    Ok(json_response(json!({
        "UserId": user.user_id,
        "IdentityStoreId": user.identity_store_id,
        "UserName": user.user_name,
        "DisplayName": user.display_name,
        "Emails": [{"Value": user.email}]
    })))
}

fn list_users(state: &IdentityStoreState, payload: &Value) -> Result<Response, LawsError> {
    let identity_store_id = payload["IdentityStoreId"].as_str().unwrap_or("");

    let users: Vec<Value> = state
        .users
        .iter()
        .filter(|e| {
            identity_store_id.is_empty() || e.value().identity_store_id == identity_store_id
        })
        .map(|e| {
            let u = e.value();
            json!({
                "UserId": u.user_id,
                "IdentityStoreId": u.identity_store_id,
                "UserName": u.user_name,
                "DisplayName": u.display_name
            })
        })
        .collect();

    Ok(json_response(json!({
        "Users": users
    })))
}

fn create_group(state: &IdentityStoreState, payload: &Value) -> Result<Response, LawsError> {
    let identity_store_id = payload["IdentityStoreId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing IdentityStoreId".into()))?
        .to_string();

    let display_name = payload["DisplayName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DisplayName".into()))?
        .to_string();

    let description = payload["Description"].as_str().unwrap_or("").to_string();

    let group_id = uuid::Uuid::new_v4().to_string();

    let group = Group {
        group_id: group_id.clone(),
        identity_store_id: identity_store_id.clone(),
        display_name,
        description,
    };

    state.groups.insert(group_id.clone(), group);

    Ok(json_response(json!({
        "GroupId": group_id,
        "IdentityStoreId": identity_store_id
    })))
}

fn list_groups(state: &IdentityStoreState, payload: &Value) -> Result<Response, LawsError> {
    let identity_store_id = payload["IdentityStoreId"].as_str().unwrap_or("");

    let groups: Vec<Value> = state
        .groups
        .iter()
        .filter(|e| {
            identity_store_id.is_empty() || e.value().identity_store_id == identity_store_id
        })
        .map(|e| {
            let g = e.value();
            json!({
                "GroupId": g.group_id,
                "IdentityStoreId": g.identity_store_id,
                "DisplayName": g.display_name,
                "Description": g.description
            })
        })
        .collect();

    Ok(json_response(json!({
        "Groups": groups
    })))
}

fn describe_group(state: &IdentityStoreState, payload: &Value) -> Result<Response, LawsError> {
    let group_id = payload["GroupId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing GroupId".into()))?;

    let group = state
        .groups
        .get(group_id)
        .ok_or_else(|| LawsError::NotFound(format!("Group '{}' not found", group_id)))?;

    Ok(json_response(json!({
        "GroupId": group.group_id,
        "IdentityStoreId": group.identity_store_id,
        "DisplayName": group.display_name,
        "Description": group.description
    })))
}
