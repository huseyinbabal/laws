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
pub struct ResourceShare {
    pub resource_share_arn: String,
    pub name: String,
    pub status: String,
    pub resource_arns: Vec<String>,
    pub principals: Vec<String>,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct RamState {
    pub resource_shares: DashMap<String, ResourceShare>,
}

impl Default for RamState {
    fn default() -> Self {
        Self {
            resource_shares: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &RamState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AmazonResourceSharing.")
        .unwrap_or(target);

    let result = match action {
        "CreateResourceShare" => create_resource_share(state, payload),
        "DeleteResourceShare" => delete_resource_share(state, payload),
        "GetResourceShares" => get_resource_shares(state, payload),
        "UpdateResourceShare" => update_resource_share(state, payload),
        "AssociateResourceShare" => associate_resource_share(state, payload),
        "DisassociateResourceShare" => disassociate_resource_share(state, payload),
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

fn require_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, LawsError> {
    body.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest(format!("missing required field: {field}")))
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_resource_share(state: &RamState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?.to_owned();
    let share_id = uuid::Uuid::new_v4().to_string();
    let resource_share_arn = format!("arn:aws:ram:{REGION}:{ACCOUNT_ID}:resource-share/{share_id}");
    let created_at = chrono::Utc::now().to_rfc3339();

    let resource_arns: Vec<String> = body
        .get("ResourceArns")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();

    let principals: Vec<String> = body
        .get("Principals")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();

    let share = ResourceShare {
        resource_share_arn: resource_share_arn.clone(),
        name: name.clone(),
        status: "ACTIVE".into(),
        resource_arns: resource_arns.clone(),
        principals: principals.clone(),
        created_at: created_at.clone(),
    };

    state
        .resource_shares
        .insert(resource_share_arn.clone(), share);

    Ok(json_response(json!({
        "resourceShare": {
            "resourceShareArn": resource_share_arn,
            "name": name,
            "status": "ACTIVE",
            "createdAt": created_at,
            "owningAccountId": ACCOUNT_ID
        }
    })))
}

fn delete_resource_share(state: &RamState, body: &Value) -> Result<Response, LawsError> {
    let resource_share_arn = require_str(body, "ResourceShareArn")?;
    let removed = state
        .resource_shares
        .remove(resource_share_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("resource share not found: {resource_share_arn}"))
        })?;

    let s = removed.1;
    Ok(json_response(json!({
        "resourceShare": {
            "resourceShareArn": s.resource_share_arn,
            "name": s.name,
            "status": "DELETED"
        }
    })))
}

fn get_resource_shares(state: &RamState, body: &Value) -> Result<Response, LawsError> {
    let _resource_owner = require_str(body, "ResourceOwner")?;

    let shares: Vec<Value> = state
        .resource_shares
        .iter()
        .map(|entry| {
            let s = entry.value();
            json!({
                "resourceShareArn": s.resource_share_arn,
                "name": s.name,
                "status": s.status,
                "createdAt": s.created_at,
                "owningAccountId": ACCOUNT_ID
            })
        })
        .collect();

    Ok(json_response(json!({
        "resourceShares": shares
    })))
}

fn update_resource_share(state: &RamState, body: &Value) -> Result<Response, LawsError> {
    let resource_share_arn = require_str(body, "ResourceShareArn")?;

    let mut share = state
        .resource_shares
        .get_mut(resource_share_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("resource share not found: {resource_share_arn}"))
        })?;

    if let Some(name) = body.get("Name").and_then(|v| v.as_str()) {
        share.name = name.to_owned();
    }

    Ok(json_response(json!({
        "resourceShare": {
            "resourceShareArn": share.resource_share_arn,
            "name": share.name,
            "status": share.status,
            "createdAt": share.created_at,
            "owningAccountId": ACCOUNT_ID
        }
    })))
}

fn associate_resource_share(state: &RamState, body: &Value) -> Result<Response, LawsError> {
    let resource_share_arn = require_str(body, "ResourceShareArn")?;

    let mut share = state
        .resource_shares
        .get_mut(resource_share_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("resource share not found: {resource_share_arn}"))
        })?;

    let mut associations = Vec::new();

    if let Some(arns) = body.get("ResourceArns").and_then(|v| v.as_array()) {
        for arn in arns {
            if let Some(a) = arn.as_str() {
                share.resource_arns.push(a.to_owned());
                associations.push(json!({
                    "resourceShareArn": resource_share_arn,
                    "associatedEntity": a,
                    "associationType": "RESOURCE",
                    "status": "ASSOCIATED"
                }));
            }
        }
    }

    if let Some(principals) = body.get("Principals").and_then(|v| v.as_array()) {
        for p in principals {
            if let Some(principal) = p.as_str() {
                share.principals.push(principal.to_owned());
                associations.push(json!({
                    "resourceShareArn": resource_share_arn,
                    "associatedEntity": principal,
                    "associationType": "PRINCIPAL",
                    "status": "ASSOCIATED"
                }));
            }
        }
    }

    Ok(json_response(json!({
        "resourceShareAssociations": associations
    })))
}

fn disassociate_resource_share(state: &RamState, body: &Value) -> Result<Response, LawsError> {
    let resource_share_arn = require_str(body, "ResourceShareArn")?;

    let mut share = state
        .resource_shares
        .get_mut(resource_share_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("resource share not found: {resource_share_arn}"))
        })?;

    let mut associations = Vec::new();

    if let Some(arns) = body.get("ResourceArns").and_then(|v| v.as_array()) {
        for arn in arns {
            if let Some(a) = arn.as_str() {
                share.resource_arns.retain(|r| r != a);
                associations.push(json!({
                    "resourceShareArn": resource_share_arn,
                    "associatedEntity": a,
                    "associationType": "RESOURCE",
                    "status": "DISASSOCIATED"
                }));
            }
        }
    }

    if let Some(principals) = body.get("Principals").and_then(|v| v.as_array()) {
        for p in principals {
            if let Some(principal) = p.as_str() {
                share.principals.retain(|pr| pr != principal);
                associations.push(json!({
                    "resourceShareArn": resource_share_arn,
                    "associatedEntity": principal,
                    "associationType": "PRINCIPAL",
                    "status": "DISASSOCIATED"
                }));
            }
        }
    }

    Ok(json_response(json!({
        "resourceShareAssociations": associations
    })))
}
