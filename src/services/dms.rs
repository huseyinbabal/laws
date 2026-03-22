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
pub struct ReplicationInstance {
    pub identifier: String,
    pub arn: String,
    pub instance_class: String,
    pub status: String,
    pub engine_version: String,
    pub allocated_storage: u32,
}

#[derive(Debug, Clone)]
pub struct Endpoint {
    pub identifier: String,
    pub arn: String,
    pub endpoint_type: String,
    pub engine_name: String,
    pub server_name: String,
    pub port: u16,
    pub database_name: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ReplicationTask {
    pub identifier: String,
    pub arn: String,
    pub source_endpoint_arn: String,
    pub target_endpoint_arn: String,
    pub replication_instance_arn: String,
    pub migration_type: String,
    pub table_mappings: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DmsState {
    pub replication_instances: DashMap<String, ReplicationInstance>,
    pub endpoints: DashMap<String, Endpoint>,
    pub tasks: DashMap<String, ReplicationTask>,
}

impl Default for DmsState {
    fn default() -> Self {
        Self {
            replication_instances: DashMap::new(),
            endpoints: DashMap::new(),
            tasks: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &DmsState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("AmazonDMSv20160101.").unwrap_or(target);

    let result = match action {
        "CreateReplicationInstance" => create_replication_instance(state, payload),
        "DeleteReplicationInstance" => delete_replication_instance(state, payload),
        "DescribeReplicationInstances" => describe_replication_instances(state),
        "CreateEndpoint" => create_endpoint(state, payload),
        "DeleteEndpoint" => delete_endpoint(state, payload),
        "DescribeEndpoints" => describe_endpoints(state),
        "CreateReplicationTask" => create_replication_task(state, payload),
        "StartReplicationTask" => start_replication_task(state, payload),
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

fn replication_instance_to_json(ri: &ReplicationInstance) -> Value {
    json!({
        "ReplicationInstanceIdentifier": ri.identifier,
        "ReplicationInstanceArn": ri.arn,
        "ReplicationInstanceClass": ri.instance_class,
        "ReplicationInstanceStatus": ri.status,
        "EngineVersion": ri.engine_version,
        "AllocatedStorage": ri.allocated_storage,
    })
}

fn endpoint_to_json(e: &Endpoint) -> Value {
    json!({
        "EndpointIdentifier": e.identifier,
        "EndpointArn": e.arn,
        "EndpointType": e.endpoint_type,
        "EngineName": e.engine_name,
        "ServerName": e.server_name,
        "Port": e.port,
        "DatabaseName": e.database_name,
        "Status": e.status,
    })
}

fn task_to_json(t: &ReplicationTask) -> Value {
    json!({
        "ReplicationTaskIdentifier": t.identifier,
        "ReplicationTaskArn": t.arn,
        "SourceEndpointArn": t.source_endpoint_arn,
        "TargetEndpointArn": t.target_endpoint_arn,
        "ReplicationInstanceArn": t.replication_instance_arn,
        "MigrationType": t.migration_type,
        "TableMappings": t.table_mappings,
        "Status": t.status,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_replication_instance(state: &DmsState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["ReplicationInstanceIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("ReplicationInstanceIdentifier is required".to_string())
        })?
        .to_string();

    if state.replication_instances.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "Replication instance '{}' already exists",
            id
        )));
    }

    let arn = format!("arn:aws:dms:{REGION}:{ACCOUNT_ID}:rep:{id}");
    let instance_class = payload["ReplicationInstanceClass"]
        .as_str()
        .unwrap_or("dms.t3.medium")
        .to_string();
    let allocated_storage = payload["AllocatedStorage"].as_u64().unwrap_or(50) as u32;

    let ri = ReplicationInstance {
        identifier: id.clone(),
        arn,
        instance_class,
        status: "available".to_string(),
        engine_version: "3.4.7".to_string(),
        allocated_storage,
    };

    let resp = replication_instance_to_json(&ri);
    state.replication_instances.insert(id, ri);

    Ok(json_response(
        StatusCode::OK,
        json!({ "ReplicationInstance": resp }),
    ))
}

fn delete_replication_instance(state: &DmsState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload["ReplicationInstanceArn"].as_str().ok_or_else(|| {
        LawsError::InvalidRequest("ReplicationInstanceArn is required".to_string())
    })?;

    // Find by ARN
    let key = state
        .replication_instances
        .iter()
        .find(|entry| entry.value().arn == arn)
        .map(|entry| entry.key().clone())
        .ok_or_else(|| LawsError::NotFound(format!("Replication instance not found: {}", arn)))?;

    let (_, ri) = state.replication_instances.remove(&key).unwrap();

    Ok(json_response(
        StatusCode::OK,
        json!({ "ReplicationInstance": replication_instance_to_json(&ri) }),
    ))
}

fn describe_replication_instances(state: &DmsState) -> Result<Response, LawsError> {
    let instances: Vec<Value> = state
        .replication_instances
        .iter()
        .map(|entry| replication_instance_to_json(entry.value()))
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "ReplicationInstances": instances }),
    ))
}

fn create_endpoint(state: &DmsState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["EndpointIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("EndpointIdentifier is required".to_string()))?
        .to_string();

    if state.endpoints.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "Endpoint '{}' already exists",
            id
        )));
    }

    let arn = format!("arn:aws:dms:{REGION}:{ACCOUNT_ID}:endpoint:{id}");
    let endpoint_type = payload["EndpointType"]
        .as_str()
        .unwrap_or("source")
        .to_string();
    let engine_name = payload["EngineName"]
        .as_str()
        .unwrap_or("mysql")
        .to_string();
    let server_name = payload["ServerName"]
        .as_str()
        .unwrap_or("localhost")
        .to_string();
    let port = payload["Port"].as_u64().unwrap_or(3306) as u16;
    let database_name = payload["DatabaseName"].as_str().unwrap_or("").to_string();

    let endpoint = Endpoint {
        identifier: id.clone(),
        arn,
        endpoint_type,
        engine_name,
        server_name,
        port,
        database_name,
        status: "active".to_string(),
    };

    let resp = endpoint_to_json(&endpoint);
    state.endpoints.insert(id, endpoint);

    Ok(json_response(StatusCode::OK, json!({ "Endpoint": resp })))
}

fn delete_endpoint(state: &DmsState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload["EndpointArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("EndpointArn is required".to_string()))?;

    let key = state
        .endpoints
        .iter()
        .find(|entry| entry.value().arn == arn)
        .map(|entry| entry.key().clone())
        .ok_or_else(|| LawsError::NotFound(format!("Endpoint not found: {}", arn)))?;

    let (_, endpoint) = state.endpoints.remove(&key).unwrap();

    Ok(json_response(
        StatusCode::OK,
        json!({ "Endpoint": endpoint_to_json(&endpoint) }),
    ))
}

fn describe_endpoints(state: &DmsState) -> Result<Response, LawsError> {
    let endpoints: Vec<Value> = state
        .endpoints
        .iter()
        .map(|entry| endpoint_to_json(entry.value()))
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "Endpoints": endpoints }),
    ))
}

fn create_replication_task(state: &DmsState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["ReplicationTaskIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("ReplicationTaskIdentifier is required".to_string())
        })?
        .to_string();

    if state.tasks.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "Replication task '{}' already exists",
            id
        )));
    }

    let arn = format!("arn:aws:dms:{REGION}:{ACCOUNT_ID}:task:{id}");
    let source_arn = payload["SourceEndpointArn"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let target_arn = payload["TargetEndpointArn"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let ri_arn = payload["ReplicationInstanceArn"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let migration_type = payload["MigrationType"]
        .as_str()
        .unwrap_or("full-load")
        .to_string();
    let table_mappings = payload["TableMappings"]
        .as_str()
        .unwrap_or("{}")
        .to_string();

    let task = ReplicationTask {
        identifier: id.clone(),
        arn,
        source_endpoint_arn: source_arn,
        target_endpoint_arn: target_arn,
        replication_instance_arn: ri_arn,
        migration_type,
        table_mappings,
        status: "ready".to_string(),
    };

    let resp = task_to_json(&task);
    state.tasks.insert(id, task);

    Ok(json_response(
        StatusCode::OK,
        json!({ "ReplicationTask": resp }),
    ))
}

fn start_replication_task(state: &DmsState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload["ReplicationTaskArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ReplicationTaskArn is required".to_string()))?;

    let key = state
        .tasks
        .iter()
        .find(|entry| entry.value().arn == arn)
        .map(|entry| entry.key().clone())
        .ok_or_else(|| LawsError::NotFound(format!("Replication task not found: {}", arn)))?;

    let mut task = state.tasks.get_mut(&key).unwrap();
    task.status = "running".to_string();
    let resp = task_to_json(&task);

    Ok(json_response(
        StatusCode::OK,
        json!({ "ReplicationTask": resp }),
    ))
}
