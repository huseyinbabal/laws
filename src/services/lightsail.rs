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
pub struct LightsailInstance {
    pub name: String,
    pub arn: String,
    pub blueprint_id: String,
    pub bundle_id: String,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct LightsailDatabase {
    pub name: String,
    pub arn: String,
    pub engine: String,
    pub master_username: String,
    pub state: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct LightsailState {
    pub instances: DashMap<String, LightsailInstance>,
    pub databases: DashMap<String, LightsailDatabase>,
}

impl Default for LightsailState {
    fn default() -> Self {
        Self {
            instances: DashMap::new(),
            databases: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &LightsailState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("Lightsail_20161128.").unwrap_or(target);

    let result = match action {
        "CreateInstances" => create_instances(state, payload),
        "DeleteInstance" => delete_instance(state, payload),
        "GetInstance" => get_instance(state, payload),
        "GetInstances" => get_instances(state),
        "CreateRelationalDatabase" => create_relational_database(state, payload),
        "DeleteRelationalDatabase" => delete_relational_database(state, payload),
        "GetRelationalDatabases" => get_relational_databases(state),
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

fn create_instances(state: &LightsailState, payload: &Value) -> Result<Response, LawsError> {
    let instance_names = payload["instanceNames"]
        .as_array()
        .ok_or_else(|| LawsError::InvalidRequest("Missing instanceNames".into()))?;

    let blueprint_id = payload["blueprintId"]
        .as_str()
        .unwrap_or("amazon_linux_2")
        .to_string();

    let bundle_id = payload["bundleId"]
        .as_str()
        .unwrap_or("nano_2_0")
        .to_string();

    let now = chrono::Utc::now().to_rfc3339();

    let mut operations = Vec::new();

    for name_val in instance_names {
        let name = name_val
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Invalid instance name".into()))?
            .to_string();

        let arn = format!("arn:aws:lightsail:{REGION}:{ACCOUNT_ID}:Instance/{name}");

        let instance = LightsailInstance {
            name: name.clone(),
            arn: arn.clone(),
            blueprint_id: blueprint_id.clone(),
            bundle_id: bundle_id.clone(),
            state: "running".to_string(),
            created_at: now.clone(),
        };

        state.instances.insert(name.clone(), instance);

        operations.push(json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "resourceName": name,
            "resourceType": "Instance",
            "status": "Succeeded",
            "operationType": "CreateInstance",
        }));
    }

    Ok(json_response(json!({ "operations": operations })))
}

fn delete_instance(state: &LightsailState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["instanceName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing instanceName".into()))?;

    state
        .instances
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Instance '{}' not found", name)))?;

    Ok(json_response(json!({
        "operations": [{
            "id": uuid::Uuid::new_v4().to_string(),
            "resourceName": name,
            "resourceType": "Instance",
            "status": "Succeeded",
            "operationType": "DeleteInstance",
        }]
    })))
}

fn get_instance(state: &LightsailState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["instanceName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing instanceName".into()))?;

    let instance = state
        .instances
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Instance '{}' not found", name)))?;

    Ok(json_response(json!({
        "instance": {
            "name": instance.name,
            "arn": instance.arn,
            "blueprintId": instance.blueprint_id,
            "bundleId": instance.bundle_id,
            "state": { "name": instance.state },
            "createdAt": instance.created_at,
        }
    })))
}

fn get_instances(state: &LightsailState) -> Result<Response, LawsError> {
    let instances: Vec<Value> = state
        .instances
        .iter()
        .map(|entry| {
            let i = entry.value();
            json!({
                "name": i.name,
                "arn": i.arn,
                "blueprintId": i.blueprint_id,
                "bundleId": i.bundle_id,
                "state": { "name": i.state },
                "createdAt": i.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({ "instances": instances })))
}

fn create_relational_database(
    state: &LightsailState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["relationalDatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing relationalDatabaseName".into()))?
        .to_string();

    let engine = payload["relationalDatabaseBlueprintId"]
        .as_str()
        .unwrap_or("mysql")
        .to_string();

    let master_username = payload["masterUsername"]
        .as_str()
        .unwrap_or("admin")
        .to_string();

    let arn = format!("arn:aws:lightsail:{REGION}:{ACCOUNT_ID}:RelationalDatabase/{name}");
    let now = chrono::Utc::now().to_rfc3339();

    let db = LightsailDatabase {
        name: name.clone(),
        arn,
        engine,
        master_username,
        state: "available".to_string(),
        created_at: now,
    };

    state.databases.insert(name, db);

    Ok(json_response(json!({
        "operations": [{
            "id": uuid::Uuid::new_v4().to_string(),
            "resourceType": "RelationalDatabase",
            "status": "Succeeded",
            "operationType": "CreateRelationalDatabase",
        }]
    })))
}

fn delete_relational_database(
    state: &LightsailState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["relationalDatabaseName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing relationalDatabaseName".into()))?;

    state
        .databases
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("RelationalDatabase '{}' not found", name)))?;

    Ok(json_response(json!({
        "operations": [{
            "id": uuid::Uuid::new_v4().to_string(),
            "resourceType": "RelationalDatabase",
            "status": "Succeeded",
            "operationType": "DeleteRelationalDatabase",
        }]
    })))
}

fn get_relational_databases(state: &LightsailState) -> Result<Response, LawsError> {
    let databases: Vec<Value> = state
        .databases
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "name": d.name,
                "arn": d.arn,
                "engine": d.engine,
                "masterUsername": d.master_username,
                "state": d.state,
                "createdAt": d.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({
        "relationalDatabases": databases
    })))
}
