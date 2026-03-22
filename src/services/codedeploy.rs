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
pub struct Application {
    pub name: String,
    pub application_id: String,
    pub create_time: String,
    pub compute_platform: String,
}

#[derive(Debug, Clone)]
pub struct DeploymentGroup {
    pub deployment_group_name: String,
    pub deployment_group_id: String,
    pub application_name: String,
    pub service_role_arn: String,
}

#[derive(Debug, Clone)]
pub struct Deployment {
    pub deployment_id: String,
    pub application_name: String,
    pub deployment_group_name: String,
    pub status: String,
    pub create_time: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CodeDeployState {
    pub applications: DashMap<String, Application>,
    pub deployment_groups: DashMap<String, DeploymentGroup>,
}

impl Default for CodeDeployState {
    fn default() -> Self {
        Self {
            applications: DashMap::new(),
            deployment_groups: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &CodeDeployState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("CodeDeploy_20141006.")
        .unwrap_or(target);

    let result = match action {
        "CreateApplication" => create_application(state, payload),
        "DeleteApplication" => delete_application(state, payload),
        "GetApplication" => get_application(state, payload),
        "ListApplications" => list_applications(state),
        "CreateDeploymentGroup" => create_deployment_group(state, payload),
        "DeleteDeploymentGroup" => delete_deployment_group(state, payload),
        "ListDeploymentGroups" => list_deployment_groups(state, payload),
        "CreateDeployment" => create_deployment(state, payload),
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
    (
        status,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn application_to_json(a: &Application) -> Value {
    json!({
        "applicationName": a.name,
        "applicationId": a.application_id,
        "createTime": a.create_time,
        "computePlatform": a.compute_platform,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_application(state: &CodeDeployState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["applicationName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("applicationName is required".to_string()))?
        .to_string();

    if state.applications.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Application '{}' already exists",
            name
        )));
    }

    let app_id = uuid::Uuid::new_v4().to_string();
    let compute_platform = payload["computePlatform"]
        .as_str()
        .unwrap_or("Server")
        .to_string();

    let app = Application {
        name: name.clone(),
        application_id: app_id.clone(),
        create_time: chrono::Utc::now().to_rfc3339(),
        compute_platform,
    };

    state.applications.insert(name, app);

    Ok(json_response(StatusCode::OK, json!({ "applicationId": app_id })))
}

fn delete_application(state: &CodeDeployState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["applicationName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("applicationName is required".to_string()))?;

    state
        .applications
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Application '{}' not found", name)))?;

    // Also remove associated deployment groups
    state.deployment_groups.retain(|_, dg| dg.application_name != name);

    Ok(json_response(StatusCode::OK, json!({})))
}

fn get_application(state: &CodeDeployState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["applicationName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("applicationName is required".to_string()))?;

    let app = state
        .applications
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Application '{}' not found", name)))?;

    Ok(json_response(StatusCode::OK, json!({ "application": application_to_json(&app) })))
}

fn list_applications(state: &CodeDeployState) -> Result<Response, LawsError> {
    let names: Vec<String> = state
        .applications
        .iter()
        .map(|entry| entry.value().name.clone())
        .collect();

    Ok(json_response(StatusCode::OK, json!({ "applications": names })))
}

fn create_deployment_group(
    state: &CodeDeployState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let app_name = payload["applicationName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("applicationName is required".to_string()))?
        .to_string();

    if !state.applications.contains_key(&app_name) {
        return Err(LawsError::NotFound(format!(
            "Application '{}' not found",
            app_name
        )));
    }

    let dg_name = payload["deploymentGroupName"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("deploymentGroupName is required".to_string())
        })?
        .to_string();

    let key = format!("{app_name}:{dg_name}");
    if state.deployment_groups.contains_key(&key) {
        return Err(LawsError::AlreadyExists(format!(
            "Deployment group '{}' already exists",
            dg_name
        )));
    }

    let dg_id = uuid::Uuid::new_v4().to_string();
    let service_role_arn = payload["serviceRoleArn"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let dg = DeploymentGroup {
        deployment_group_name: dg_name,
        deployment_group_id: dg_id.clone(),
        application_name: app_name,
        service_role_arn,
    };

    state.deployment_groups.insert(key, dg);

    Ok(json_response(StatusCode::OK, json!({ "deploymentGroupId": dg_id })))
}

fn delete_deployment_group(
    state: &CodeDeployState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let app_name = payload["applicationName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("applicationName is required".to_string()))?;

    let dg_name = payload["deploymentGroupName"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("deploymentGroupName is required".to_string())
        })?;

    let key = format!("{app_name}:{dg_name}");
    state
        .deployment_groups
        .remove(&key)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Deployment group '{}' not found", dg_name))
        })?;

    Ok(json_response(StatusCode::OK, json!({})))
}

fn list_deployment_groups(
    state: &CodeDeployState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let app_name = payload["applicationName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("applicationName is required".to_string()))?;

    let groups: Vec<String> = state
        .deployment_groups
        .iter()
        .filter(|entry| entry.value().application_name == app_name)
        .map(|entry| entry.value().deployment_group_name.clone())
        .collect();

    Ok(json_response(StatusCode::OK, json!({
        "applicationName": app_name,
        "deploymentGroups": groups,
    })))
}

fn create_deployment(state: &CodeDeployState, payload: &Value) -> Result<Response, LawsError> {
    let app_name = payload["applicationName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("applicationName is required".to_string()))?
        .to_string();

    if !state.applications.contains_key(&app_name) {
        return Err(LawsError::NotFound(format!(
            "Application '{}' not found",
            app_name
        )));
    }

    let dg_name = payload["deploymentGroupName"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let deployment_id = uuid::Uuid::new_v4().to_string();

    let _deployment = Deployment {
        deployment_id: deployment_id.clone(),
        application_name: app_name,
        deployment_group_name: dg_name,
        status: "Created".to_string(),
        create_time: chrono::Utc::now().to_rfc3339(),
    };

    Ok(json_response(StatusCode::OK, json!({ "deploymentId": deployment_id })))
}
