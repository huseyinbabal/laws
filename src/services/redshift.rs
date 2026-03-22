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
pub struct RedshiftCluster {
    pub cluster_identifier: String,
    pub node_type: String,
    pub number_of_nodes: u32,
    pub db_name: String,
    pub master_username: String,
    pub status: String,
    pub arn: String,
    pub endpoint: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct RedshiftState {
    pub clusters: DashMap<String, RedshiftCluster>,
}

impl Default for RedshiftState {
    fn default() -> Self {
        Self {
            clusters: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &RedshiftState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target
        .strip_prefix("RedshiftServiceVersion20121201.")
        .unwrap_or(target);

    let result = match action {
        "CreateCluster" => create_cluster(state, payload),
        "DeleteCluster" => delete_cluster(state, payload),
        "DescribeClusters" => describe_clusters(state, payload),
        "PauseCluster" => pause_cluster(state, payload),
        "ResumeCluster" => resume_cluster(state, payload),
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

fn cluster_to_json(c: &RedshiftCluster) -> Value {
    json!({
        "ClusterIdentifier": c.cluster_identifier,
        "NodeType": c.node_type,
        "NumberOfNodes": c.number_of_nodes,
        "DBName": c.db_name,
        "MasterUsername": c.master_username,
        "ClusterStatus": c.status,
        "ClusterArn": c.arn,
        "Endpoint": {
            "Address": c.endpoint,
            "Port": 5439
        }
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_cluster(state: &RedshiftState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["ClusterIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterIdentifier is required".to_string()))?
        .to_string();

    if state.clusters.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "Redshift cluster '{}' already exists",
            id
        )));
    }

    let node_type = payload["NodeType"]
        .as_str()
        .unwrap_or("dc2.large")
        .to_string();
    let number_of_nodes = payload["NumberOfNodes"]
        .as_u64()
        .unwrap_or(1) as u32;
    let db_name = payload["DBName"]
        .as_str()
        .unwrap_or("dev")
        .to_string();
    let master_username = payload["MasterUsername"]
        .as_str()
        .unwrap_or("admin")
        .to_string();

    let arn = format!("arn:aws:redshift:{REGION}:{ACCOUNT_ID}:cluster:{id}");
    let endpoint = format!("{id}.{ACCOUNT_ID}.{REGION}.redshift.amazonaws.com");

    let cluster = RedshiftCluster {
        cluster_identifier: id.clone(),
        node_type,
        number_of_nodes,
        db_name,
        master_username,
        status: "available".to_string(),
        arn,
        endpoint,
    };

    let resp = cluster_to_json(&cluster);
    state.clusters.insert(id, cluster);

    Ok(json_response(json!({ "Cluster": resp })))
}

fn delete_cluster(state: &RedshiftState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["ClusterIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterIdentifier is required".to_string()))?;

    let (_, cluster) = state
        .clusters
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Redshift cluster '{}' not found", id)))?;

    Ok(json_response(json!({ "Cluster": cluster_to_json(&cluster) })))
}

fn describe_clusters(state: &RedshiftState, payload: &Value) -> Result<Response, LawsError> {
    let filter_id = payload["ClusterIdentifier"].as_str();

    let clusters: Vec<Value> = state
        .clusters
        .iter()
        .filter(|entry| {
            filter_id
                .map(|fid| entry.key() == fid)
                .unwrap_or(true)
        })
        .map(|entry| cluster_to_json(entry.value()))
        .collect();

    if let Some(fid) = filter_id {
        if clusters.is_empty() {
            return Err(LawsError::NotFound(format!(
                "Redshift cluster '{}' not found",
                fid
            )));
        }
    }

    Ok(json_response(json!({ "Clusters": clusters })))
}

fn pause_cluster(state: &RedshiftState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["ClusterIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterIdentifier is required".to_string()))?;

    let mut cluster = state
        .clusters
        .get_mut(id)
        .ok_or_else(|| LawsError::NotFound(format!("Redshift cluster '{}' not found", id)))?;

    cluster.status = "paused".to_string();
    let resp = cluster_to_json(&cluster);

    Ok(json_response(json!({ "Cluster": resp })))
}

fn resume_cluster(state: &RedshiftState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["ClusterIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterIdentifier is required".to_string()))?;

    let mut cluster = state
        .clusters
        .get_mut(id)
        .ok_or_else(|| LawsError::NotFound(format!("Redshift cluster '{}' not found", id)))?;

    cluster.status = "available".to_string();
    let resp = cluster_to_json(&cluster);

    Ok(json_response(json!({ "Cluster": resp })))
}
