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
pub struct DbCluster {
    pub cluster_identifier: String,
    pub arn: String,
    pub engine: String,
    pub engine_version: String,
    pub status: String,
    pub endpoint: String,
    pub port: u16,
    pub master_username: String,
}

#[derive(Debug, Clone)]
pub struct DbInstance {
    pub instance_identifier: String,
    pub arn: String,
    pub cluster_identifier: String,
    pub instance_class: String,
    pub engine: String,
    pub status: String,
    pub endpoint: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DocumentDbState {
    pub clusters: DashMap<String, DbCluster>,
    pub instances: DashMap<String, DbInstance>,
}

impl Default for DocumentDbState {
    fn default() -> Self {
        Self {
            clusters: DashMap::new(),
            instances: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &DocumentDbState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AmazonRDSv19_DocDB.")
        .unwrap_or(target);

    let result = match action {
        "CreateDBCluster" => create_db_cluster(state, payload),
        "DeleteDBCluster" => delete_db_cluster(state, payload),
        "DescribeDBClusters" => describe_db_clusters(state),
        "CreateDBInstance" => create_db_instance(state, payload),
        "DeleteDBInstance" => delete_db_instance(state, payload),
        "DescribeDBInstances" => describe_db_instances(state),
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

fn cluster_to_json(c: &DbCluster) -> Value {
    json!({
        "DBClusterIdentifier": c.cluster_identifier,
        "DBClusterArn": c.arn,
        "Engine": c.engine,
        "EngineVersion": c.engine_version,
        "Status": c.status,
        "Endpoint": c.endpoint,
        "Port": c.port,
        "MasterUsername": c.master_username,
    })
}

fn instance_to_json(i: &DbInstance) -> Value {
    json!({
        "DBInstanceIdentifier": i.instance_identifier,
        "DBInstanceArn": i.arn,
        "DBClusterIdentifier": i.cluster_identifier,
        "DBInstanceClass": i.instance_class,
        "Engine": i.engine,
        "DBInstanceStatus": i.status,
        "Endpoint": {
            "Address": i.endpoint,
            "Port": 27017,
        },
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_db_cluster(state: &DocumentDbState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DBClusterIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("DBClusterIdentifier is required".to_string())
        })?
        .to_string();

    if state.clusters.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "DB cluster '{}' already exists",
            id
        )));
    }

    let arn = format!("arn:aws:rds:{REGION}:{ACCOUNT_ID}:cluster:{id}");
    let engine_version = payload["EngineVersion"]
        .as_str()
        .unwrap_or("5.0.0")
        .to_string();
    let master_username = payload["MasterUsername"]
        .as_str()
        .unwrap_or("docdbadmin")
        .to_string();

    let cluster = DbCluster {
        cluster_identifier: id.clone(),
        arn,
        engine: "docdb".to_string(),
        engine_version,
        status: "available".to_string(),
        endpoint: format!("{id}.cluster-abc123.{REGION}.docdb.amazonaws.com"),
        port: 27017,
        master_username,
    };

    let resp = cluster_to_json(&cluster);
    state.clusters.insert(id, cluster);

    Ok(json_response(StatusCode::OK, json!({ "DBCluster": resp })))
}

fn delete_db_cluster(state: &DocumentDbState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DBClusterIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("DBClusterIdentifier is required".to_string())
        })?;

    let (_, cluster) = state
        .clusters
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("DB cluster '{}' not found", id)))?;

    Ok(json_response(StatusCode::OK, json!({ "DBCluster": cluster_to_json(&cluster) })))
}

fn describe_db_clusters(state: &DocumentDbState) -> Result<Response, LawsError> {
    let clusters: Vec<Value> = state
        .clusters
        .iter()
        .map(|entry| cluster_to_json(entry.value()))
        .collect();

    Ok(json_response(StatusCode::OK, json!({ "DBClusters": clusters })))
}

fn create_db_instance(state: &DocumentDbState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DBInstanceIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string())
        })?
        .to_string();

    if state.instances.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "DB instance '{}' already exists",
            id
        )));
    }

    let cluster_id = payload["DBClusterIdentifier"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let instance_class = payload["DBInstanceClass"]
        .as_str()
        .unwrap_or("db.r5.large")
        .to_string();

    let arn = format!("arn:aws:rds:{REGION}:{ACCOUNT_ID}:db:{id}");

    let instance = DbInstance {
        instance_identifier: id.clone(),
        arn,
        cluster_identifier: cluster_id,
        instance_class,
        engine: "docdb".to_string(),
        status: "available".to_string(),
        endpoint: format!("{id}.abc123.{REGION}.docdb.amazonaws.com"),
    };

    let resp = instance_to_json(&instance);
    state.instances.insert(id, instance);

    Ok(json_response(StatusCode::OK, json!({ "DBInstance": resp })))
}

fn delete_db_instance(state: &DocumentDbState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DBInstanceIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string())
        })?;

    let (_, instance) = state
        .instances
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("DB instance '{}' not found", id)))?;

    Ok(json_response(StatusCode::OK, json!({ "DBInstance": instance_to_json(&instance) })))
}

fn describe_db_instances(state: &DocumentDbState) -> Result<Response, LawsError> {
    let instances: Vec<Value> = state
        .instances
        .iter()
        .map(|entry| instance_to_json(entry.value()))
        .collect();

    Ok(json_response(StatusCode::OK, json!({ "DBInstances": instances })))
}
