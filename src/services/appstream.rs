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
pub struct Fleet {
    pub name: String,
    pub arn: String,
    pub instance_type: String,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Stack {
    pub name: String,
    pub arn: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ImageBuilder {
    pub name: String,
    pub arn: String,
    pub instance_type: String,
    pub state: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AppStreamState {
    pub fleets: DashMap<String, Fleet>,
    pub stacks: DashMap<String, Stack>,
    pub image_builders: DashMap<String, ImageBuilder>,
}

impl Default for AppStreamState {
    fn default() -> Self {
        Self {
            fleets: DashMap::new(),
            stacks: DashMap::new(),
            image_builders: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &AppStreamState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("PhotonAdminProxyService.")
        .unwrap_or(target);

    let result = match action {
        "CreateFleet" => create_fleet(state, payload),
        "DeleteFleet" => delete_fleet(state, payload),
        "DescribeFleets" => describe_fleets(state, payload),
        "CreateStack" => create_stack(state, payload),
        "DeleteStack" => delete_stack(state, payload),
        "DescribeStacks" => describe_stacks(state, payload),
        "CreateImageBuilder" => create_image_builder(state, payload),
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

fn fleet_to_json(f: &Fleet) -> Value {
    json!({
        "Name": f.name,
        "Arn": f.arn,
        "InstanceType": f.instance_type,
        "State": f.state,
        "CreatedTime": f.created_at,
    })
}

fn stack_to_json(s: &Stack) -> Value {
    json!({
        "Name": s.name,
        "Arn": s.arn,
        "Description": s.description,
        "CreatedTime": s.created_at,
    })
}

fn image_builder_to_json(ib: &ImageBuilder) -> Value {
    json!({
        "Name": ib.name,
        "Arn": ib.arn,
        "InstanceType": ib.instance_type,
        "State": ib.state,
        "CreatedTime": ib.created_at,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_fleet(state: &AppStreamState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    if state.fleets.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Fleet '{}' already exists",
            name
        )));
    }

    let instance_type = payload["InstanceType"]
        .as_str()
        .unwrap_or("stream.standard.medium")
        .to_string();

    let arn = format!("arn:aws:appstream:{REGION}:{ACCOUNT_ID}:fleet/{name}");
    let now = chrono::Utc::now().to_rfc3339();

    let fleet = Fleet {
        name: name.clone(),
        arn,
        instance_type,
        state: "STOPPED".to_string(),
        created_at: now,
    };

    let resp = fleet_to_json(&fleet);
    state.fleets.insert(name, fleet);

    Ok(json_response(json!({ "Fleet": resp })))
}

fn delete_fleet(state: &AppStreamState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    state
        .fleets
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Fleet '{}' not found", name)))?;

    Ok(json_response(json!({})))
}

fn describe_fleets(state: &AppStreamState, payload: &Value) -> Result<Response, LawsError> {
    let names = payload["Names"].as_array();

    let fleets: Vec<Value> = if let Some(names) = names {
        names
            .iter()
            .filter_map(|n| {
                let name = n.as_str()?;
                state.fleets.get(name).map(|f| fleet_to_json(f.value()))
            })
            .collect()
    } else {
        state
            .fleets
            .iter()
            .map(|entry| fleet_to_json(entry.value()))
            .collect()
    };

    Ok(json_response(json!({ "Fleets": fleets })))
}

fn create_stack(state: &AppStreamState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    if state.stacks.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Stack '{}' already exists",
            name
        )));
    }

    let description = payload["Description"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let arn = format!("arn:aws:appstream:{REGION}:{ACCOUNT_ID}:stack/{name}");
    let now = chrono::Utc::now().to_rfc3339();

    let stack = Stack {
        name: name.clone(),
        arn,
        description,
        created_at: now,
    };

    let resp = stack_to_json(&stack);
    state.stacks.insert(name, stack);

    Ok(json_response(json!({ "Stack": resp })))
}

fn delete_stack(state: &AppStreamState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    state
        .stacks
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Stack '{}' not found", name)))?;

    Ok(json_response(json!({})))
}

fn describe_stacks(state: &AppStreamState, payload: &Value) -> Result<Response, LawsError> {
    let names = payload["Names"].as_array();

    let stacks: Vec<Value> = if let Some(names) = names {
        names
            .iter()
            .filter_map(|n| {
                let name = n.as_str()?;
                state.stacks.get(name).map(|s| stack_to_json(s.value()))
            })
            .collect()
    } else {
        state
            .stacks
            .iter()
            .map(|entry| stack_to_json(entry.value()))
            .collect()
    };

    Ok(json_response(json!({ "Stacks": stacks })))
}

fn create_image_builder(
    state: &AppStreamState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    if state.image_builders.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Image builder '{}' already exists",
            name
        )));
    }

    let instance_type = payload["InstanceType"]
        .as_str()
        .unwrap_or("stream.standard.medium")
        .to_string();

    let arn = format!("arn:aws:appstream:{REGION}:{ACCOUNT_ID}:image-builder/{name}");
    let now = chrono::Utc::now().to_rfc3339();

    let ib = ImageBuilder {
        name: name.clone(),
        arn,
        instance_type,
        state: "PENDING".to_string(),
        created_at: now,
    };

    let resp = image_builder_to_json(&ib);
    state.image_builders.insert(name, ib);

    Ok(json_response(json!({ "ImageBuilder": resp })))
}
