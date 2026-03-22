use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PermissionSet {
    pub permission_set_arn: String,
    pub name: String,
    pub description: String,
    pub session_duration: String,
    pub created_date: String,
}

#[derive(Debug, Clone)]
pub struct AccountAssignment {
    pub permission_set_arn: String,
    pub principal_id: String,
    pub principal_type: String,
    pub target_id: String,
    pub target_type: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct SsoState {
    pub permission_sets: DashMap<String, PermissionSet>,
    pub account_assignments: DashMap<String, AccountAssignment>,
}

impl Default for SsoState {
    fn default() -> Self {
        Self {
            permission_sets: DashMap::new(),
            account_assignments: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &SsoState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("SWBExternalService.")
        .unwrap_or(target);

    let result = match action {
        "CreatePermissionSet" => create_permission_set(state, payload),
        "DeletePermissionSet" => delete_permission_set(state, payload),
        "ListPermissionSets" => list_permission_sets(state, payload),
        "DescribePermissionSet" => describe_permission_set(state, payload),
        "CreateAccountAssignment" => create_account_assignment(state, payload),
        "ListAccountAssignments" => list_account_assignments(state, payload),
        other => Err(LawsError::InvalidRequest(format!("unknown action: {other}"))),
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
    (StatusCode::OK, [("Content-Type", "application/x-amz-json-1.1")], serde_json::to_string(&body).unwrap_or_default()).into_response()
}

fn require_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, LawsError> {
    body.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest(format!("missing required field: {field}")))
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_permission_set(state: &SsoState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?.to_owned();
    let instance_arn = require_str(body, "InstanceArn")?;
    let description = body.get("Description").and_then(|v| v.as_str()).unwrap_or("").to_owned();
    let session_duration = body.get("SessionDuration").and_then(|v| v.as_str()).unwrap_or("PT1H").to_owned();
    let ps_id = uuid::Uuid::new_v4().to_string();
    let permission_set_arn = format!("{instance_arn}/ps-{ps_id}");
    let created_date = chrono::Utc::now().to_rfc3339();

    let ps = PermissionSet {
        permission_set_arn: permission_set_arn.clone(),
        name: name.clone(),
        description: description.clone(),
        session_duration: session_duration.clone(),
        created_date: created_date.clone(),
    };

    state.permission_sets.insert(permission_set_arn.clone(), ps);

    Ok(json_response(json!({
        "PermissionSet": {
            "PermissionSetArn": permission_set_arn,
            "Name": name,
            "Description": description,
            "SessionDuration": session_duration,
            "CreatedDate": created_date
        }
    })))
}

fn delete_permission_set(state: &SsoState, body: &Value) -> Result<Response, LawsError> {
    let permission_set_arn = require_str(body, "PermissionSetArn")?;
    state.permission_sets.remove(permission_set_arn)
        .ok_or_else(|| LawsError::NotFound(format!("permission set not found: {permission_set_arn}")))?;

    Ok(json_response(json!({})))
}

fn list_permission_sets(state: &SsoState, _body: &Value) -> Result<Response, LawsError> {
    let arns: Vec<String> = state.permission_sets.iter()
        .map(|entry| entry.key().clone())
        .collect();

    Ok(json_response(json!({
        "PermissionSets": arns
    })))
}

fn describe_permission_set(state: &SsoState, body: &Value) -> Result<Response, LawsError> {
    let permission_set_arn = require_str(body, "PermissionSetArn")?;

    let ps = state.permission_sets.get(permission_set_arn)
        .ok_or_else(|| LawsError::NotFound(format!("permission set not found: {permission_set_arn}")))?;

    Ok(json_response(json!({
        "PermissionSet": {
            "PermissionSetArn": ps.permission_set_arn,
            "Name": ps.name,
            "Description": ps.description,
            "SessionDuration": ps.session_duration,
            "CreatedDate": ps.created_date
        }
    })))
}

fn create_account_assignment(state: &SsoState, body: &Value) -> Result<Response, LawsError> {
    let permission_set_arn = require_str(body, "PermissionSetArn")?.to_owned();
    let principal_id = require_str(body, "PrincipalId")?.to_owned();
    let principal_type = require_str(body, "PrincipalType")?.to_owned();
    let target_id = require_str(body, "TargetId")?.to_owned();
    let target_type = body.get("TargetType").and_then(|v| v.as_str()).unwrap_or("AWS_ACCOUNT").to_owned();

    let key = format!("{permission_set_arn}:{principal_id}:{target_id}");

    let assignment = AccountAssignment {
        permission_set_arn: permission_set_arn.clone(),
        principal_id: principal_id.clone(),
        principal_type: principal_type.clone(),
        target_id: target_id.clone(),
        target_type: target_type.clone(),
    };

    state.account_assignments.insert(key, assignment);

    Ok(json_response(json!({
        "AccountAssignmentCreationStatus": {
            "Status": "SUCCEEDED",
            "PermissionSetArn": permission_set_arn,
            "PrincipalId": principal_id,
            "PrincipalType": principal_type,
            "TargetId": target_id,
            "TargetType": target_type
        }
    })))
}

fn list_account_assignments(state: &SsoState, body: &Value) -> Result<Response, LawsError> {
    let _instance_arn = require_str(body, "InstanceArn")?;
    let permission_set_arn = require_str(body, "PermissionSetArn")?;
    let account_id = require_str(body, "AccountId")?;

    let assignments: Vec<Value> = state.account_assignments.iter()
        .filter(|entry| {
            let a = entry.value();
            a.permission_set_arn == permission_set_arn && a.target_id == account_id
        })
        .map(|entry| {
            let a = entry.value();
            json!({
                "PermissionSetArn": a.permission_set_arn,
                "PrincipalId": a.principal_id,
                "PrincipalType": a.principal_type,
                "TargetId": a.target_id,
                "TargetType": a.target_type,
                "AccountId": a.target_id
            })
        })
        .collect();

    Ok(json_response(json!({
        "AccountAssignments": assignments
    })))
}
