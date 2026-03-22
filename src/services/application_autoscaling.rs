use axum::response::{IntoResponse, Response};
use chrono::Utc;
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
pub struct ScalableTarget {
    pub service_namespace: String,
    pub resource_id: String,
    pub scalable_dimension: String,
    pub min_capacity: i64,
    pub max_capacity: i64,
    pub role_arn: String,
    pub creation_time: String,
    pub suspended_state: Value,
}

#[derive(Debug, Clone)]
pub struct ScalingPolicy {
    pub policy_name: String,
    pub policy_arn: String,
    pub service_namespace: String,
    pub resource_id: String,
    pub scalable_dimension: String,
    pub policy_type: String,
    pub target_tracking_config: Value,
    pub creation_time: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ApplicationAutoscalingState {
    pub targets: DashMap<String, ScalableTarget>,
    pub policies: DashMap<String, ScalingPolicy>,
}

impl Default for ApplicationAutoscalingState {
    fn default() -> Self {
        Self {
            targets: DashMap::new(),
            policies: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &ApplicationAutoscalingState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AnyScaleFrontendService.")
        .unwrap_or(target);

    let result = match action {
        "RegisterScalableTarget" => register_scalable_target(state, payload),
        "DeregisterScalableTarget" => deregister_scalable_target(state, payload),
        "DescribeScalableTargets" => describe_scalable_targets(state, payload),
        "PutScalingPolicy" => put_scaling_policy(state, payload),
        "DescribeScalingPolicies" => describe_scaling_policies(state, payload),
        "DeleteScalingPolicy" => delete_scaling_policy(state, payload),
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

fn json_response(status: StatusCode, body: Value) -> Response {
    (status, axum::Json(body)).into_response()
}

fn target_key(service_namespace: &str, resource_id: &str, scalable_dimension: &str) -> String {
    format!("{service_namespace}:{resource_id}:{scalable_dimension}")
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn register_scalable_target(
    state: &ApplicationAutoscalingState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let service_namespace = payload["ServiceNamespace"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ServiceNamespace".into()))?
        .to_string();
    let resource_id = payload["ResourceId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ResourceId".into()))?
        .to_string();
    let scalable_dimension = payload["ScalableDimension"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ScalableDimension".into()))?
        .to_string();

    let min_capacity = payload["MinCapacity"].as_i64().unwrap_or(1);
    let max_capacity = payload["MaxCapacity"].as_i64().unwrap_or(10);
    let role_arn = payload["RoleARN"]
        .as_str()
        .unwrap_or(&format!(
            "arn:aws:iam::{ACCOUNT_ID}:role/aws-service-role/autoscaling"
        ))
        .to_string();
    let now = Utc::now().to_rfc3339();

    let key = target_key(&service_namespace, &resource_id, &scalable_dimension);

    let t = ScalableTarget {
        service_namespace,
        resource_id,
        scalable_dimension,
        min_capacity,
        max_capacity,
        role_arn,
        creation_time: now,
        suspended_state: payload
            .get("SuspendedState")
            .cloned()
            .unwrap_or(json!({
                "DynamicScalingInSuspended": false,
                "DynamicScalingOutSuspended": false,
                "ScheduledScalingSuspended": false,
            })),
    };

    state.targets.insert(key, t);

    Ok(json_response(StatusCode::OK, json!({})))
}

fn deregister_scalable_target(
    state: &ApplicationAutoscalingState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let service_namespace = payload["ServiceNamespace"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ServiceNamespace".into()))?;
    let resource_id = payload["ResourceId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ResourceId".into()))?;
    let scalable_dimension = payload["ScalableDimension"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ScalableDimension".into()))?;

    let key = target_key(service_namespace, resource_id, scalable_dimension);

    state
        .targets
        .remove(&key)
        .ok_or_else(|| LawsError::NotFound(format!("ScalableTarget not found: {key}")))?;

    Ok(json_response(StatusCode::OK, json!({})))
}

fn describe_scalable_targets(
    state: &ApplicationAutoscalingState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let service_namespace = payload["ServiceNamespace"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ServiceNamespace".into()))?;

    let targets: Vec<Value> = state
        .targets
        .iter()
        .filter(|entry| entry.value().service_namespace == service_namespace)
        .map(|entry| {
            let t = entry.value();
            json!({
                "ServiceNamespace": t.service_namespace,
                "ResourceId": t.resource_id,
                "ScalableDimension": t.scalable_dimension,
                "MinCapacity": t.min_capacity,
                "MaxCapacity": t.max_capacity,
                "RoleARN": t.role_arn,
                "CreationTime": t.creation_time,
                "SuspendedState": t.suspended_state,
            })
        })
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "ScalableTargets": targets }),
    ))
}

fn put_scaling_policy(
    state: &ApplicationAutoscalingState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let policy_name = payload["PolicyName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing PolicyName".into()))?
        .to_string();
    let service_namespace = payload["ServiceNamespace"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ServiceNamespace".into()))?
        .to_string();
    let resource_id = payload["ResourceId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ResourceId".into()))?
        .to_string();
    let scalable_dimension = payload["ScalableDimension"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ScalableDimension".into()))?
        .to_string();
    let policy_type = payload["PolicyType"]
        .as_str()
        .unwrap_or("TargetTrackingScaling")
        .to_string();

    let policy_arn = format!(
        "arn:aws:autoscaling:{REGION}:{ACCOUNT_ID}:scalingPolicy:{}:resource/{}/{}:policyName/{}",
        uuid::Uuid::new_v4(),
        service_namespace,
        resource_id,
        policy_name
    );
    let now = Utc::now().to_rfc3339();

    let policy = ScalingPolicy {
        policy_name: policy_name.clone(),
        policy_arn: policy_arn.clone(),
        service_namespace,
        resource_id,
        scalable_dimension,
        policy_type,
        target_tracking_config: payload
            .get("TargetTrackingScalingPolicyConfiguration")
            .cloned()
            .unwrap_or(Value::Null),
        creation_time: now,
    };

    state.policies.insert(policy_name, policy);

    Ok(json_response(
        StatusCode::OK,
        json!({
            "PolicyARN": policy_arn,
            "Alarms": [],
        }),
    ))
}

fn describe_scaling_policies(
    state: &ApplicationAutoscalingState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let service_namespace = payload["ServiceNamespace"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ServiceNamespace".into()))?;

    let policies: Vec<Value> = state
        .policies
        .iter()
        .filter(|entry| entry.value().service_namespace == service_namespace)
        .map(|entry| {
            let p = entry.value();
            json!({
                "PolicyName": p.policy_name,
                "PolicyARN": p.policy_arn,
                "ServiceNamespace": p.service_namespace,
                "ResourceId": p.resource_id,
                "ScalableDimension": p.scalable_dimension,
                "PolicyType": p.policy_type,
                "CreationTime": p.creation_time,
            })
        })
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "ScalingPolicies": policies }),
    ))
}

fn delete_scaling_policy(
    state: &ApplicationAutoscalingState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let policy_name = payload["PolicyName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing PolicyName".into()))?;

    state
        .policies
        .remove(policy_name)
        .ok_or_else(|| {
            LawsError::NotFound(format!("ScalingPolicy not found: {policy_name}"))
        })?;

    Ok(json_response(StatusCode::OK, json!({})))
}
