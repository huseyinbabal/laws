use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use rand::Rng;
use rand::distributions::Alphanumeric;
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
pub struct UserPool {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub status: String,
    pub creation_date: String,
}

#[derive(Debug, Clone)]
pub struct UserPoolClient {
    pub client_id: String,
    pub client_name: String,
    pub user_pool_id: String,
}

#[derive(Debug, Clone)]
pub struct CognitoUser {
    pub username: String,
    pub user_pool_id: String,
    pub status: String,
    pub enabled: bool,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CognitoState {
    pub user_pools: DashMap<String, UserPool>,
    pub user_pool_clients: DashMap<String, UserPoolClient>,
    pub users: DashMap<String, CognitoUser>,
}

impl Default for CognitoState {
    fn default() -> Self {
        Self {
            user_pools: DashMap::new(),
            user_pool_clients: DashMap::new(),
            users: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &CognitoState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target
        .strip_prefix("AWSCognitoIdentityProviderService.")
        .unwrap_or(target);

    let result = match action {
        "CreateUserPool" => create_user_pool(state, payload),
        "DeleteUserPool" => delete_user_pool(state, payload),
        "ListUserPools" => list_user_pools(state, payload),
        "DescribeUserPool" => describe_user_pool(state, payload),
        "CreateUserPoolClient" => create_user_pool_client(state, payload),
        "AdminCreateUser" => admin_create_user(state, payload),
        "AdminDeleteUser" => admin_delete_user(state, payload),
        "AdminGetUser" => admin_get_user(state, payload),
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

fn generate_pool_id() -> String {
    let random_part: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(9)
        .map(char::from)
        .collect::<String>();
    format!("{REGION}_{random_part}")
}

fn generate_client_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(26)
        .map(char::from)
        .collect::<String>()
}

fn user_pool_to_json(pool: &UserPool) -> Value {
    json!({
        "Id": pool.id,
        "Name": pool.name,
        "Arn": pool.arn,
        "Status": pool.status,
        "CreationDate": pool.creation_date
    })
}

fn user_pool_client_to_json(client: &UserPoolClient) -> Value {
    json!({
        "ClientId": client.client_id,
        "ClientName": client.client_name,
        "UserPoolId": client.user_pool_id
    })
}

fn user_to_json(user: &CognitoUser) -> Value {
    json!({
        "Username": user.username,
        "UserPoolId": user.user_pool_id,
        "UserStatus": user.status,
        "Enabled": user.enabled,
        "UserCreateDate": user.created_at
    })
}

/// Composite key for users: "{user_pool_id}#{username}"
fn user_key(user_pool_id: &str, username: &str) -> String {
    format!("{user_pool_id}#{username}")
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_user_pool(state: &CognitoState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["PoolName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("PoolName is required".to_string()))?
        .to_string();

    let id = generate_pool_id();
    let arn = format!("arn:aws:cognito-idp:{REGION}:{ACCOUNT_ID}:userpool/{id}");
    let now = chrono::Utc::now().to_rfc3339();

    let pool = UserPool {
        id: id.clone(),
        name,
        arn,
        status: "Enabled".to_string(),
        creation_date: now,
    };

    let resp = user_pool_to_json(&pool);
    state.user_pools.insert(id, pool);

    Ok(json_response(json!({ "UserPool": resp })))
}

fn delete_user_pool(state: &CognitoState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["UserPoolId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("UserPoolId is required".to_string()))?;

    state
        .user_pools
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("User pool '{}' not found", id)))?;

    Ok(json_response(json!({})))
}

fn list_user_pools(state: &CognitoState, _payload: &Value) -> Result<Response, LawsError> {
    let pools: Vec<Value> = state
        .user_pools
        .iter()
        .map(|entry| user_pool_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "UserPools": pools })))
}

fn describe_user_pool(state: &CognitoState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["UserPoolId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("UserPoolId is required".to_string()))?;

    let pool = state
        .user_pools
        .get(id)
        .ok_or_else(|| LawsError::NotFound(format!("User pool '{}' not found", id)))?;

    Ok(json_response(json!({ "UserPool": user_pool_to_json(&pool) })))
}

fn create_user_pool_client(state: &CognitoState, payload: &Value) -> Result<Response, LawsError> {
    let user_pool_id = payload["UserPoolId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("UserPoolId is required".to_string()))?
        .to_string();

    if !state.user_pools.contains_key(&user_pool_id) {
        return Err(LawsError::NotFound(format!(
            "User pool '{}' not found",
            user_pool_id
        )));
    }

    let client_name = payload["ClientName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClientName is required".to_string()))?
        .to_string();

    let client_id = generate_client_id();

    let client = UserPoolClient {
        client_id: client_id.clone(),
        client_name,
        user_pool_id,
    };

    let resp = user_pool_client_to_json(&client);
    state.user_pool_clients.insert(client_id, client);

    Ok(json_response(json!({ "UserPoolClient": resp })))
}

fn admin_create_user(state: &CognitoState, payload: &Value) -> Result<Response, LawsError> {
    let user_pool_id = payload["UserPoolId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("UserPoolId is required".to_string()))?
        .to_string();

    if !state.user_pools.contains_key(&user_pool_id) {
        return Err(LawsError::NotFound(format!(
            "User pool '{}' not found",
            user_pool_id
        )));
    }

    let username = payload["Username"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Username is required".to_string()))?
        .to_string();

    let key = user_key(&user_pool_id, &username);
    if state.users.contains_key(&key) {
        return Err(LawsError::AlreadyExists(format!(
            "User '{}' already exists in pool '{}'",
            username, user_pool_id
        )));
    }

    let now = chrono::Utc::now().to_rfc3339();

    let user = CognitoUser {
        username: username.clone(),
        user_pool_id,
        status: "CONFIRMED".to_string(),
        enabled: true,
        created_at: now,
    };

    let resp = user_to_json(&user);
    state.users.insert(key, user);

    Ok(json_response(json!({ "User": resp })))
}

fn admin_delete_user(state: &CognitoState, payload: &Value) -> Result<Response, LawsError> {
    let user_pool_id = payload["UserPoolId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("UserPoolId is required".to_string()))?;
    let username = payload["Username"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Username is required".to_string()))?;

    let key = user_key(user_pool_id, username);
    state
        .users
        .remove(&key)
        .ok_or_else(|| LawsError::NotFound(format!("User '{}' not found", username)))?;

    Ok(json_response(json!({})))
}

fn admin_get_user(state: &CognitoState, payload: &Value) -> Result<Response, LawsError> {
    let user_pool_id = payload["UserPoolId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("UserPoolId is required".to_string()))?;
    let username = payload["Username"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Username is required".to_string()))?;

    let key = user_key(user_pool_id, username);
    let user = state
        .users
        .get(&key)
        .ok_or_else(|| LawsError::NotFound(format!("User '{}' not found", username)))?;

    Ok(json_response(user_to_json(&user)))
}

fn list_users(state: &CognitoState, payload: &Value) -> Result<Response, LawsError> {
    let user_pool_id = payload["UserPoolId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("UserPoolId is required".to_string()))?;

    if !state.user_pools.contains_key(user_pool_id) {
        return Err(LawsError::NotFound(format!(
            "User pool '{}' not found",
            user_pool_id
        )));
    }

    let users: Vec<Value> = state
        .users
        .iter()
        .filter(|entry| entry.value().user_pool_id == user_pool_id)
        .map(|entry| user_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "Users": users })))
}
