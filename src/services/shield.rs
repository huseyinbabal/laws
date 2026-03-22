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
pub struct Protection {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub resource_arn: String,
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub start_time: String,
    pub time_commitment_in_seconds: u64,
    pub auto_renew: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ShieldState {
    pub protections: DashMap<String, Protection>,
    pub subscription: DashMap<String, Subscription>,
}

impl Default for ShieldState {
    fn default() -> Self {
        Self {
            protections: DashMap::new(),
            subscription: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &ShieldState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("AWSShield_20160616.").unwrap_or(target);

    let result = match action {
        "CreateProtection" => create_protection(state, payload),
        "DeleteProtection" => delete_protection(state, payload),
        "ListProtections" => list_protections(state),
        "DescribeProtection" => describe_protection(state, payload),
        "CreateSubscription" => create_subscription(state),
        "DescribeSubscription" => describe_subscription(state),
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

fn create_protection(state: &ShieldState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let resource_arn = payload["ResourceArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ResourceArn".into()))?
        .to_string();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:shield:{REGION}:{ACCOUNT_ID}:protection/{id}");

    let protection = Protection {
        id: id.clone(),
        name,
        arn,
        resource_arn,
    };

    state.protections.insert(id.clone(), protection);

    Ok(json_response(json!({
        "ProtectionId": id
    })))
}

fn delete_protection(state: &ShieldState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["ProtectionId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ProtectionId".into()))?;

    state
        .protections
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Protection '{}' not found", id)))?;

    Ok(json_response(json!({})))
}

fn list_protections(state: &ShieldState) -> Result<Response, LawsError> {
    let protections: Vec<Value> = state
        .protections
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "Id": p.id,
                "Name": p.name,
                "ProtectionArn": p.arn,
                "ResourceArn": p.resource_arn,
            })
        })
        .collect();

    Ok(json_response(json!({
        "Protections": protections
    })))
}

fn describe_protection(state: &ShieldState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["ProtectionId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ProtectionId".into()))?;

    let protection = state
        .protections
        .get(id)
        .ok_or_else(|| LawsError::NotFound(format!("Protection '{}' not found", id)))?;

    Ok(json_response(json!({
        "Protection": {
            "Id": protection.id,
            "Name": protection.name,
            "ProtectionArn": protection.arn,
            "ResourceArn": protection.resource_arn,
        }
    })))
}

fn create_subscription(state: &ShieldState) -> Result<Response, LawsError> {
    let now = chrono::Utc::now().to_rfc3339();

    let subscription = Subscription {
        start_time: now,
        time_commitment_in_seconds: 31536000,
        auto_renew: "ENABLED".to_string(),
    };

    state
        .subscription
        .insert("default".to_string(), subscription);

    Ok(json_response(json!({})))
}

fn describe_subscription(state: &ShieldState) -> Result<Response, LawsError> {
    match state.subscription.get("default") {
        Some(sub) => Ok(json_response(json!({
            "Subscription": {
                "StartTime": sub.start_time,
                "TimeCommitmentInSeconds": sub.time_commitment_in_seconds,
                "AutoRenew": sub.auto_renew,
            }
        }))),
        None => Err(LawsError::NotFound("No subscription found".to_string())),
    }
}
