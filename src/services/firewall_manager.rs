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
pub struct FmsPolicy {
    pub policy_id: String,
    pub policy_name: String,
    pub security_service_type: String,
    pub resource_type: String,
    pub remediation_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct NotificationChannel {
    pub sns_topic_arn: String,
    pub sns_role_name: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct FirewallManagerState {
    pub policies: DashMap<String, FmsPolicy>,
    pub notification_channel: DashMap<String, NotificationChannel>,
}

impl Default for FirewallManagerState {
    fn default() -> Self {
        Self {
            policies: DashMap::new(),
            notification_channel: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &FirewallManagerState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target.strip_prefix("AWSFMS_20180101.").unwrap_or(target);

    let result = match action {
        "PutPolicy" => put_policy(state, payload),
        "GetPolicy" => get_policy(state, payload),
        "ListPolicies" => list_policies(state),
        "DeletePolicy" => delete_policy(state, payload),
        "PutNotificationChannel" => put_notification_channel(state, payload),
        "GetNotificationChannel" => get_notification_channel(state),
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

fn policy_to_json(p: &FmsPolicy) -> Value {
    json!({
        "PolicyId": p.policy_id,
        "PolicyName": p.policy_name,
        "SecurityServiceType": p.security_service_type,
        "ResourceType": p.resource_type,
        "RemediationEnabled": p.remediation_enabled,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn put_policy(state: &FirewallManagerState, payload: &Value) -> Result<Response, LawsError> {
    let policy = payload
        .get("Policy")
        .ok_or_else(|| LawsError::InvalidRequest("Missing Policy".into()))?;

    let policy_name = policy["PolicyName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing PolicyName".into()))?
        .to_string();

    let policy_id = policy["PolicyId"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let security_service = policy["SecurityServicePolicyData"]["Type"]
        .as_str()
        .unwrap_or("WAF")
        .to_string();

    let resource_type = policy["ResourceType"]
        .as_str()
        .unwrap_or("AWS::ElasticLoadBalancingV2::LoadBalancer")
        .to_string();

    let remediation_enabled = policy["RemediationEnabled"].as_bool().unwrap_or(false);

    let fms_policy = FmsPolicy {
        policy_id: policy_id.clone(),
        policy_name,
        security_service_type: security_service,
        resource_type,
        remediation_enabled,
    };

    let resp = policy_to_json(&fms_policy);
    state.policies.insert(policy_id, fms_policy);
    Ok(json_response(
        json!({ "Policy": resp, "PolicyArn": format!("arn:aws:fms:{REGION}:{ACCOUNT_ID}:policy/{}", resp["PolicyId"].as_str().unwrap_or("")) }),
    ))
}

fn get_policy(state: &FirewallManagerState, payload: &Value) -> Result<Response, LawsError> {
    let policy_id = payload["PolicyId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing PolicyId".into()))?;

    let policy = state
        .policies
        .get(policy_id)
        .ok_or_else(|| LawsError::NotFound(format!("Policy not found: {policy_id}")))?;

    let arn = format!("arn:aws:fms:{REGION}:{ACCOUNT_ID}:policy/{policy_id}");
    Ok(json_response(
        json!({ "Policy": policy_to_json(policy.value()), "PolicyArn": arn }),
    ))
}

fn list_policies(state: &FirewallManagerState) -> Result<Response, LawsError> {
    let policies: Vec<Value> = state
        .policies
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "PolicyId": p.policy_id,
                "PolicyName": p.policy_name,
                "ResourceType": p.resource_type,
                "SecurityServiceType": p.security_service_type,
                "RemediationEnabled": p.remediation_enabled,
                "PolicyArn": format!("arn:aws:fms:{REGION}:{ACCOUNT_ID}:policy/{}", p.policy_id),
            })
        })
        .collect();

    Ok(json_response(json!({ "PolicyList": policies })))
}

fn delete_policy(state: &FirewallManagerState, payload: &Value) -> Result<Response, LawsError> {
    let policy_id = payload["PolicyId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing PolicyId".into()))?;

    state
        .policies
        .remove(policy_id)
        .ok_or_else(|| LawsError::NotFound(format!("Policy not found: {policy_id}")))?;

    Ok(json_response(json!({})))
}

fn put_notification_channel(
    state: &FirewallManagerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let sns_topic_arn = payload["SnsTopicArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing SnsTopicArn".into()))?
        .to_string();

    let sns_role_name = payload["SnsRoleName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing SnsRoleName".into()))?
        .to_string();

    let channel = NotificationChannel {
        sns_topic_arn,
        sns_role_name,
    };

    state
        .notification_channel
        .insert("default".to_string(), channel);
    Ok(json_response(json!({})))
}

fn get_notification_channel(state: &FirewallManagerState) -> Result<Response, LawsError> {
    let channel = state
        .notification_channel
        .get("default")
        .ok_or_else(|| LawsError::NotFound("Notification channel not configured".into()))?;

    Ok(json_response(json!({
        "SnsTopicArn": channel.sns_topic_arn,
        "SnsRoleName": channel.sns_role_name,
    })))
}
