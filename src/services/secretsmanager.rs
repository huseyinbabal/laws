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
pub struct Secret {
    pub name: String,
    pub arn: String,
    pub secret_string: String,
    pub version_id: String,
    pub created_date: f64,
    pub last_changed_date: f64,
    pub description: Option<String>,
}

pub struct SecretsManagerState {
    pub secrets: MemoryStore<Secret>,
    pub account_id: String,
    pub region: String,
}

impl Default for SecretsManagerState {
    fn default() -> Self {
        Self {
            secrets: MemoryStore::new(),
            account_id: "000000000000".to_string(),
            region: "us-east-1".to_string(),
        }
    }
}

pub fn router(state: Arc<SecretsManagerState>) -> Router {
    Router::new()
        .route("/", post(handle_action))
        .with_state(state)
}

pub async fn handle_request(
    state: &SecretsManagerState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("secretsmanager.").unwrap_or(target);

    let result = match action {
        "CreateSecret" => create_secret(state, payload).await,
        "GetSecretValue" => get_secret_value(state, payload).await,
        "UpdateSecret" => update_secret(state, payload).await,
        "DeleteSecret" => delete_secret(state, payload).await,
        "ListSecrets" => list_secrets(state).await,
        "DescribeSecret" => describe_secret(state, payload).await,
        "RotateSecret" => rotate_secret(state, payload).await,
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
    State(state): State<Arc<SecretsManagerState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Response, LawsError> {
    let target = headers
        .get("X-Amz-Target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let action = target
        .strip_prefix("secretsmanager.")
        .unwrap_or(target);

    let payload: serde_json::Value =
        serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

    match action {
        "CreateSecret" => create_secret(&state, &payload).await,
        "GetSecretValue" => get_secret_value(&state, &payload).await,
        "UpdateSecret" => update_secret(&state, &payload).await,
        "DeleteSecret" => delete_secret(&state, &payload).await,
        "ListSecrets" => list_secrets(&state).await,
        "DescribeSecret" => describe_secret(&state, &payload).await,
        "RotateSecret" => rotate_secret(&state, &payload).await,
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

fn make_arn(state: &SecretsManagerState, name: &str) -> String {
    format!(
        "arn:aws:secretsmanager:{}:{}:secret:{}",
        state.region, state.account_id, name
    )
}

fn resolve_secret_id<'a>(payload: &'a serde_json::Value) -> Result<&'a str, LawsError> {
    payload["SecretId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("SecretId is required".to_string()))
}

/// Look up a secret by SecretId which can be a name or an ARN.
fn find_secret(state: &SecretsManagerState, secret_id: &str) -> Result<Secret, LawsError> {
    // Try by name first
    if let Some(secret) = state.secrets.get(secret_id) {
        return Ok(secret);
    }
    // Try matching by ARN
    for (_, secret) in state.secrets.list() {
        if secret.arn == secret_id {
            return Ok(secret);
        }
    }
    Err(LawsError::NotFound(format!(
        "Secret '{}' not found",
        secret_id
    )))
}

async fn create_secret(
    state: &SecretsManagerState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;
    let secret_string = payload["SecretString"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if state.secrets.contains(name) {
        return Err(LawsError::AlreadyExists(format!(
            "Secret '{}' already exists",
            name
        )));
    }

    let now = chrono::Utc::now().timestamp() as f64;
    let version_id = uuid::Uuid::new_v4().to_string();
    let arn = make_arn(state, name);

    let description = payload["Description"].as_str().map(String::from);

    let secret = Secret {
        name: name.to_string(),
        arn: arn.clone(),
        secret_string,
        version_id: version_id.clone(),
        created_date: now,
        last_changed_date: now,
        description,
    };

    state.secrets.insert(name.to_string(), secret);

    Ok(json_response(serde_json::json!({
        "ARN": arn,
        "Name": name,
        "VersionId": version_id,
    })))
}

async fn get_secret_value(
    state: &SecretsManagerState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let secret_id = resolve_secret_id(payload)?;
    let secret = find_secret(state, secret_id)?;

    Ok(json_response(serde_json::json!({
        "ARN": secret.arn,
        "Name": secret.name,
        "SecretString": secret.secret_string,
        "VersionId": secret.version_id,
        "VersionStages": ["AWSCURRENT"],
        "CreatedDate": secret.created_date,
    })))
}

async fn update_secret(
    state: &SecretsManagerState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let secret_id = resolve_secret_id(payload)?;
    let mut secret = find_secret(state, secret_id)?;

    if let Some(new_value) = payload["SecretString"].as_str() {
        secret.secret_string = new_value.to_string();
    }

    secret.version_id = uuid::Uuid::new_v4().to_string();
    secret.last_changed_date = chrono::Utc::now().timestamp() as f64;

    let arn = secret.arn.clone();
    let name = secret.name.clone();
    let version_id = secret.version_id.clone();

    state.secrets.insert(name.clone(), secret);

    Ok(json_response(serde_json::json!({
        "ARN": arn,
        "Name": name,
        "VersionId": version_id,
    })))
}

async fn delete_secret(
    state: &SecretsManagerState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let secret_id = resolve_secret_id(payload)?;
    let secret = find_secret(state, secret_id)?;
    let name = secret.name.clone();

    state
        .secrets
        .remove(&name)
        .ok_or_else(|| LawsError::NotFound(format!("Secret '{}' not found", secret_id)))?;

    let deletion_date = chrono::Utc::now().timestamp() as f64;

    Ok(json_response(serde_json::json!({
        "ARN": secret.arn,
        "Name": name,
        "DeletionDate": deletion_date,
    })))
}

async fn list_secrets(state: &SecretsManagerState) -> Result<Response, LawsError> {
    let secret_list: Vec<serde_json::Value> = state
        .secrets
        .list_values()
        .iter()
        .map(|s| {
            serde_json::json!({
                "ARN": s.arn,
                "Name": s.name,
                "VersionId": s.version_id,
                "LastChangedDate": s.last_changed_date,
                "CreatedDate": s.created_date,
                "Description": s.description,
            })
        })
        .collect();

    Ok(json_response(serde_json::json!({
        "SecretList": secret_list,
    })))
}

async fn describe_secret(
    state: &SecretsManagerState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let secret_id = resolve_secret_id(payload)?;
    let secret = find_secret(state, secret_id)?;

    Ok(json_response(serde_json::json!({
        "ARN": secret.arn,
        "Name": secret.name,
        "VersionId": secret.version_id,
        "LastChangedDate": secret.last_changed_date,
        "CreatedDate": secret.created_date,
        "Description": secret.description,
        "VersionIdsToStages": {
            secret.version_id: ["AWSCURRENT"],
        },
    })))
}

async fn rotate_secret(
    state: &SecretsManagerState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let secret_id = resolve_secret_id(payload)?;
    let mut secret = find_secret(state, secret_id)?;

    // Simulate rotation by generating a new version ID
    secret.version_id = uuid::Uuid::new_v4().to_string();
    secret.last_changed_date = chrono::Utc::now().timestamp() as f64;

    let arn = secret.arn.clone();
    let name = secret.name.clone();
    let version_id = secret.version_id.clone();

    state.secrets.insert(name.clone(), secret);

    Ok(json_response(serde_json::json!({
        "ARN": arn,
        "Name": name,
        "VersionId": version_id,
    })))
}
