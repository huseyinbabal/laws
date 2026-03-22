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
pub struct EnvironmentTemplate {
    pub arn: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ProtonService {
    pub arn: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub created_at: String,
    pub template_name: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ProtonState {
    pub environments: DashMap<String, EnvironmentTemplate>,
    pub services: DashMap<String, ProtonService>,
}

impl Default for ProtonState {
    fn default() -> Self {
        Self {
            environments: DashMap::new(),
            services: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &ProtonState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AwsProton20200720.")
        .unwrap_or(target);

    let result = match action {
        "CreateEnvironmentTemplate" => create_environment_template(state, payload),
        "DeleteEnvironmentTemplate" => delete_environment_template(state, payload),
        "ListEnvironmentTemplates" => list_environment_templates(state),
        "CreateService" => create_service(state, payload),
        "DeleteService" => delete_service(state, payload),
        "ListServices" => list_services(state),
        "GetService" => get_service(state, payload),
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

fn create_environment_template(state: &ProtonState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?.to_owned();
    let description = body.get("Description").and_then(|v| v.as_str()).unwrap_or("").to_owned();
    let arn = format!("arn:aws:proton:{REGION}:{ACCOUNT_ID}:environment-template/{name}");
    let created_at = chrono::Utc::now().to_rfc3339();

    let template = EnvironmentTemplate {
        arn: arn.clone(),
        name: name.clone(),
        description: description.clone(),
        status: "DRAFT".into(),
        created_at: created_at.clone(),
    };

    state.environments.insert(name.clone(), template);

    Ok(json_response(json!({
        "environmentTemplate": {
            "arn": arn,
            "name": name,
            "description": description,
            "status": "DRAFT",
            "createdAt": created_at
        }
    })))
}

fn delete_environment_template(state: &ProtonState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?;
    let removed = state.environments.remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("environment template not found: {name}")))?;

    let t = removed.1;
    Ok(json_response(json!({
        "environmentTemplate": {
            "arn": t.arn,
            "name": t.name,
            "status": "DELETE_COMPLETE"
        }
    })))
}

fn list_environment_templates(state: &ProtonState) -> Result<Response, LawsError> {
    let templates: Vec<Value> = state.environments.iter().map(|entry| {
        let t = entry.value();
        json!({
            "arn": t.arn,
            "name": t.name,
            "description": t.description,
            "status": t.status,
            "createdAt": t.created_at
        })
    }).collect();

    Ok(json_response(json!({
        "templates": templates
    })))
}

fn create_service(state: &ProtonState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?.to_owned();
    let description = body.get("Description").and_then(|v| v.as_str()).unwrap_or("").to_owned();
    let template_name = body.get("TemplateName").and_then(|v| v.as_str()).unwrap_or("").to_owned();
    let arn = format!("arn:aws:proton:{REGION}:{ACCOUNT_ID}:service/{name}");
    let created_at = chrono::Utc::now().to_rfc3339();

    let service = ProtonService {
        arn: arn.clone(),
        name: name.clone(),
        description: description.clone(),
        status: "ACTIVE".into(),
        created_at: created_at.clone(),
        template_name: template_name.clone(),
    };

    state.services.insert(name.clone(), service);

    Ok(json_response(json!({
        "service": {
            "arn": arn,
            "name": name,
            "description": description,
            "status": "CREATE_IN_PROGRESS",
            "createdAt": created_at,
            "templateName": template_name
        }
    })))
}

fn delete_service(state: &ProtonState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?;
    let removed = state.services.remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("service not found: {name}")))?;

    let s = removed.1;
    Ok(json_response(json!({
        "service": {
            "arn": s.arn,
            "name": s.name,
            "status": "DELETE_IN_PROGRESS"
        }
    })))
}

fn list_services(state: &ProtonState) -> Result<Response, LawsError> {
    let services: Vec<Value> = state.services.iter().map(|entry| {
        let s = entry.value();
        json!({
            "arn": s.arn,
            "name": s.name,
            "description": s.description,
            "status": s.status,
            "createdAt": s.created_at,
            "templateName": s.template_name
        })
    }).collect();

    Ok(json_response(json!({
        "services": services
    })))
}

fn get_service(state: &ProtonState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "Name")?;
    let s = state.services.get(name)
        .ok_or_else(|| LawsError::NotFound(format!("service not found: {name}")))?;

    Ok(json_response(json!({
        "service": {
            "arn": s.arn,
            "name": s.name,
            "description": s.description,
            "status": s.status,
            "createdAt": s.created_at,
            "templateName": s.template_name
        }
    })))
}
