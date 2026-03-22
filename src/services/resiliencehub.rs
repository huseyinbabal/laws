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
pub struct ResilienceApp {
    pub app_arn: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub policy_arn: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ResiliencyPolicy {
    pub policy_arn: String,
    pub policy_name: String,
    pub tier: String,
    pub estimated_cost_tier: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ResilienceHubState {
    pub apps: DashMap<String, ResilienceApp>,
    pub policies: DashMap<String, ResiliencyPolicy>,
}

impl Default for ResilienceHubState {
    fn default() -> Self {
        Self {
            apps: DashMap::new(),
            policies: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &ResilienceHubState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("AwsResilienceHub.").unwrap_or(target);

    let result = match action {
        "CreateApp" => create_app(state, payload),
        "DeleteApp" => delete_app(state, payload),
        "DescribeApp" => describe_app(state, payload),
        "ListApps" => list_apps(state),
        "CreateResiliencyPolicy" => create_resiliency_policy(state, payload),
        "ListResiliencyPolicies" => list_resiliency_policies(state),
        "StartAppAssessment" => start_app_assessment(state, payload),
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

fn create_app(state: &ResilienceHubState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing name".into()))?
        .to_string();

    let description = payload["description"].as_str().unwrap_or("").to_string();

    let policy_arn = payload["policyArn"].as_str().unwrap_or("").to_string();

    let app_arn = format!(
        "arn:aws:resiliencehub:{REGION}:{ACCOUNT_ID}:app/{}",
        uuid::Uuid::new_v4()
    );
    let created_at = chrono::Utc::now().to_rfc3339();

    let app = ResilienceApp {
        app_arn: app_arn.clone(),
        name: name.clone(),
        description: description.clone(),
        status: "Active".to_string(),
        policy_arn: policy_arn.clone(),
        created_at: created_at.clone(),
    };

    state.apps.insert(app_arn.clone(), app);

    Ok(json_response(json!({
        "app": {
            "appArn": app_arn,
            "name": name,
            "description": description,
            "status": "Active",
            "policyArn": policy_arn,
            "creationTime": created_at,
        }
    })))
}

fn delete_app(state: &ResilienceHubState, payload: &Value) -> Result<Response, LawsError> {
    let app_arn = payload["appArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing appArn".into()))?;

    state
        .apps
        .remove(app_arn)
        .ok_or_else(|| LawsError::NotFound(format!("App '{}' not found", app_arn)))?;

    Ok(json_response(json!({
        "appArn": app_arn,
    })))
}

fn describe_app(state: &ResilienceHubState, payload: &Value) -> Result<Response, LawsError> {
    let app_arn = payload["appArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing appArn".into()))?;

    let app = state
        .apps
        .get(app_arn)
        .ok_or_else(|| LawsError::NotFound(format!("App '{}' not found", app_arn)))?;

    Ok(json_response(json!({
        "app": {
            "appArn": app.app_arn,
            "name": app.name,
            "description": app.description,
            "status": app.status,
            "policyArn": app.policy_arn,
            "creationTime": app.created_at,
        }
    })))
}

fn list_apps(state: &ResilienceHubState) -> Result<Response, LawsError> {
    let apps: Vec<Value> = state
        .apps
        .iter()
        .map(|entry| {
            let a = entry.value();
            json!({
                "appArn": a.app_arn,
                "name": a.name,
                "description": a.description,
                "status": a.status,
                "creationTime": a.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({
        "appSummaries": apps,
    })))
}

fn create_resiliency_policy(
    state: &ResilienceHubState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let policy_name = payload["policyName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing policyName".into()))?
        .to_string();

    let tier = payload["tier"]
        .as_str()
        .unwrap_or("MissionCritical")
        .to_string();

    let estimated_cost_tier = payload["estimatedCostTier"]
        .as_str()
        .unwrap_or("L1")
        .to_string();

    let policy_arn = format!(
        "arn:aws:resiliencehub:{REGION}:{ACCOUNT_ID}:resiliency-policy/{}",
        uuid::Uuid::new_v4()
    );
    let created_at = chrono::Utc::now().to_rfc3339();

    let policy = ResiliencyPolicy {
        policy_arn: policy_arn.clone(),
        policy_name: policy_name.clone(),
        tier: tier.clone(),
        estimated_cost_tier: estimated_cost_tier.clone(),
        created_at: created_at.clone(),
    };

    state.policies.insert(policy_arn.clone(), policy);

    Ok(json_response(json!({
        "policy": {
            "policyArn": policy_arn,
            "policyName": policy_name,
            "tier": tier,
            "estimatedCostTier": estimated_cost_tier,
            "creationTime": created_at,
        }
    })))
}

fn list_resiliency_policies(state: &ResilienceHubState) -> Result<Response, LawsError> {
    let policies: Vec<Value> = state
        .policies
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "policyArn": p.policy_arn,
                "policyName": p.policy_name,
                "tier": p.tier,
                "estimatedCostTier": p.estimated_cost_tier,
                "creationTime": p.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({
        "resiliencyPolicies": policies,
    })))
}

fn start_app_assessment(
    state: &ResilienceHubState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let app_arn = payload["appArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing appArn".into()))?;

    let assessment_name = payload["assessmentName"]
        .as_str()
        .unwrap_or("assessment")
        .to_string();

    if !state.apps.contains_key(app_arn) {
        return Err(LawsError::NotFound(format!("App '{}' not found", app_arn)));
    }

    let assessment_arn = format!(
        "arn:aws:resiliencehub:{REGION}:{ACCOUNT_ID}:app-assessment/{}",
        uuid::Uuid::new_v4()
    );

    Ok(json_response(json!({
        "assessment": {
            "assessmentArn": assessment_arn,
            "assessmentName": assessment_name,
            "appArn": app_arn,
            "assessmentStatus": "InProgress",
            "startTime": chrono::Utc::now().to_rfc3339(),
        }
    })))
}
