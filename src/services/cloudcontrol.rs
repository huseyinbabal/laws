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
pub struct Resource {
    pub type_name: String,
    pub identifier: String,
    pub properties: Value,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CloudControlState {
    pub resources: DashMap<String, Resource>,
}

impl Default for CloudControlState {
    fn default() -> Self {
        Self {
            resources: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &CloudControlState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("CloudApiService.").unwrap_or(target);

    let result = match action {
        "CreateResource" => create_resource(state, payload),
        "DeleteResource" => delete_resource(state, payload),
        "GetResource" => get_resource(state, payload),
        "ListResources" => list_resources(state, payload),
        "UpdateResource" => update_resource(state, payload),
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
        [("Content-Type", "application/x-amz-json-1.0")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn make_key(type_name: &str, identifier: &str) -> String {
    format!("{type_name}:{identifier}")
}

fn resource_description(r: &Resource) -> Value {
    json!({
        "TypeName": r.type_name,
        "Identifier": r.identifier,
        "Properties": serde_json::to_string(&r.properties).unwrap_or_default(),
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_resource(state: &CloudControlState, payload: &Value) -> Result<Response, LawsError> {
    let type_name = payload["TypeName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("TypeName is required".to_string()))?
        .to_string();

    let desired_state = payload["DesiredState"].as_str().unwrap_or("{}");

    let properties: Value = serde_json::from_str(desired_state).unwrap_or(json!({}));

    let identifier = uuid::Uuid::new_v4().to_string();
    let key = make_key(&type_name, &identifier);
    let now = chrono::Utc::now().to_rfc3339();

    let resource = Resource {
        type_name: type_name.clone(),
        identifier: identifier.clone(),
        properties,
        created_at: now,
    };

    state.resources.insert(key, resource);

    let request_token = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "ProgressEvent": {
            "TypeName": type_name,
            "Identifier": identifier,
            "RequestToken": request_token,
            "OperationStatus": "SUCCESS",
            "Operation": "CREATE",
        }
    })))
}

fn delete_resource(state: &CloudControlState, payload: &Value) -> Result<Response, LawsError> {
    let type_name = payload["TypeName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("TypeName is required".to_string()))?;

    let identifier = payload["Identifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Identifier is required".to_string()))?;

    let key = make_key(type_name, identifier);

    state
        .resources
        .remove(&key)
        .ok_or_else(|| LawsError::NotFound(format!("Resource '{}' not found", key)))?;

    let request_token = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "ProgressEvent": {
            "TypeName": type_name,
            "Identifier": identifier,
            "RequestToken": request_token,
            "OperationStatus": "SUCCESS",
            "Operation": "DELETE",
        }
    })))
}

fn get_resource(state: &CloudControlState, payload: &Value) -> Result<Response, LawsError> {
    let type_name = payload["TypeName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("TypeName is required".to_string()))?;

    let identifier = payload["Identifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Identifier is required".to_string()))?;

    let key = make_key(type_name, identifier);

    let resource = state
        .resources
        .get(&key)
        .ok_or_else(|| LawsError::NotFound(format!("Resource '{}' not found", key)))?;

    Ok(json_response(json!({
        "ResourceDescription": resource_description(resource.value()),
    })))
}

fn list_resources(state: &CloudControlState, payload: &Value) -> Result<Response, LawsError> {
    let type_name = payload["TypeName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("TypeName is required".to_string()))?;

    let descriptions: Vec<Value> = state
        .resources
        .iter()
        .filter(|entry| entry.value().type_name == type_name)
        .map(|entry| resource_description(entry.value()))
        .collect();

    Ok(json_response(json!({
        "ResourceDescriptions": descriptions,
        "TypeName": type_name,
    })))
}

fn update_resource(state: &CloudControlState, payload: &Value) -> Result<Response, LawsError> {
    let type_name = payload["TypeName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("TypeName is required".to_string()))?;

    let identifier = payload["Identifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Identifier is required".to_string()))?;

    let key = make_key(type_name, identifier);

    let patch_document = payload["PatchDocument"].as_str().unwrap_or("[]");

    let mut resource = state
        .resources
        .get_mut(&key)
        .ok_or_else(|| LawsError::NotFound(format!("Resource '{}' not found", key)))?;

    // Simple merge: parse patch as JSON and merge into properties
    if let Ok(patches) = serde_json::from_str::<Vec<Value>>(patch_document) {
        for patch in &patches {
            if patch["op"].as_str() == Some("replace") || patch["op"].as_str() == Some("add") {
                if let (Some(path), Some(value)) = (patch["path"].as_str(), patch.get("value")) {
                    let field = path.trim_start_matches('/');
                    if let Some(obj) = resource.properties.as_object_mut() {
                        obj.insert(field.to_string(), value.clone());
                    }
                }
            }
        }
    }

    let request_token = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "ProgressEvent": {
            "TypeName": type_name,
            "Identifier": identifier,
            "RequestToken": request_token,
            "OperationStatus": "SUCCESS",
            "Operation": "UPDATE",
        }
    })))
}
