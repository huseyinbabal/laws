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
pub struct LakeFormationResource {
    pub resource_arn: String,
    pub role_arn: String,
    pub last_modified: f64,
}

#[derive(Debug, Clone)]
pub struct LakeFormationPermission {
    pub id: String,
    pub principal: String,
    pub resource: Value,
    pub permissions: Vec<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct LakeFormationState {
    pub resources: DashMap<String, LakeFormationResource>,
    pub permissions: DashMap<String, LakeFormationPermission>,
}

impl Default for LakeFormationState {
    fn default() -> Self {
        Self {
            resources: DashMap::new(),
            permissions: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &LakeFormationState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AWSLakeFormation.")
        .unwrap_or(target);

    let result = match action {
        "RegisterResource" => register_resource(state, payload),
        "DeregisterResource" => deregister_resource(state, payload),
        "ListResources" => list_resources(state),
        "GrantPermissions" => grant_permissions(state, payload),
        "RevokePermissions" => revoke_permissions(state, payload),
        "ListPermissions" => list_permissions(state),
        "GetDataLakeSettings" => get_data_lake_settings(state),
        "PutDataLakeSettings" => put_data_lake_settings(state, payload),
        other => Err(LawsError::InvalidRequest(format!(
            "unknown action: {other}"
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

fn now_epoch() -> f64 {
    chrono::Utc::now().timestamp() as f64
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn register_resource(
    state: &LakeFormationState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let resource_arn = payload["ResourceArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ResourceArn is required".to_string()))?
        .to_string();

    let role_arn = payload["RoleArn"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if state.resources.contains_key(&resource_arn) {
        return Err(LawsError::AlreadyExists(format!(
            "Resource '{}' already registered",
            resource_arn
        )));
    }

    let resource = LakeFormationResource {
        resource_arn: resource_arn.clone(),
        role_arn,
        last_modified: now_epoch(),
    };

    state.resources.insert(resource_arn, resource);

    Ok(json_response(json!({})))
}

fn deregister_resource(
    state: &LakeFormationState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let resource_arn = payload["ResourceArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ResourceArn is required".to_string()))?;

    state
        .resources
        .remove(resource_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Resource '{}' not found", resource_arn))
        })?;

    Ok(json_response(json!({})))
}

fn list_resources(state: &LakeFormationState) -> Result<Response, LawsError> {
    let items: Vec<Value> = state
        .resources
        .iter()
        .map(|entry| {
            let r = entry.value();
            json!({
                "ResourceArn": r.resource_arn,
                "RoleArn": r.role_arn,
                "LastModified": r.last_modified,
            })
        })
        .collect();

    Ok(json_response(json!({
        "ResourceInfoList": items,
    })))
}

fn grant_permissions(
    state: &LakeFormationState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let principal = payload["Principal"]["DataLakePrincipalIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest(
                "Principal.DataLakePrincipalIdentifier is required".to_string(),
            )
        })?
        .to_string();

    let resource = payload["Resource"].clone();
    let permissions: Vec<String> = payload["Permissions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let id = uuid::Uuid::new_v4().to_string();

    let perm = LakeFormationPermission {
        id: id.clone(),
        principal,
        resource,
        permissions,
    };

    state.permissions.insert(id, perm);

    Ok(json_response(json!({})))
}

fn revoke_permissions(
    state: &LakeFormationState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let principal = payload["Principal"]["DataLakePrincipalIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest(
                "Principal.DataLakePrincipalIdentifier is required".to_string(),
            )
        })?;

    // Remove first matching permission for this principal
    let key = state
        .permissions
        .iter()
        .find(|entry| entry.value().principal == principal)
        .map(|entry| entry.key().clone());

    if let Some(k) = key {
        state.permissions.remove(&k);
    }

    Ok(json_response(json!({})))
}

fn list_permissions(state: &LakeFormationState) -> Result<Response, LawsError> {
    let items: Vec<Value> = state
        .permissions
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "Principal": {
                    "DataLakePrincipalIdentifier": p.principal,
                },
                "Resource": p.resource,
                "Permissions": p.permissions,
            })
        })
        .collect();

    Ok(json_response(json!({
        "PrincipalResourcePermissions": items,
    })))
}

fn get_data_lake_settings(_state: &LakeFormationState) -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "DataLakeSettings": {
            "DataLakeAdmins": [],
            "CreateDatabaseDefaultPermissions": [],
            "CreateTableDefaultPermissions": [],
            "TrustedResourceOwners": [],
        }
    })))
}

fn put_data_lake_settings(
    _state: &LakeFormationState,
    _payload: &Value,
) -> Result<Response, LawsError> {
    Ok(json_response(json!({})))
}
