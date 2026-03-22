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
pub struct DaxCluster {
    pub cluster_name: String,
    pub cluster_arn: String,
    pub node_type: String,
    pub replication_factor: u64,
    pub status: String,
    pub total_nodes: u64,
    pub active_nodes: u64,
    pub cluster_discovery_endpoint: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DaxState {
    pub clusters: DashMap<String, DaxCluster>,
}

impl Default for DaxState {
    fn default() -> Self {
        Self {
            clusters: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &DaxState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("AmazonDAXV3.").unwrap_or(target);

    let result = match action {
        "CreateCluster" => create_cluster(state, payload),
        "DeleteCluster" => delete_cluster(state, payload),
        "DescribeClusters" => describe_clusters(state, payload),
        "IncreaseReplicationFactor" => increase_replication_factor(state, payload),
        "DecreaseReplicationFactor" => decrease_replication_factor(state, payload),
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

fn cluster_to_json(cluster: &DaxCluster) -> Value {
    json!({
        "ClusterName": cluster.cluster_name,
        "ClusterArn": cluster.cluster_arn,
        "NodeType": cluster.node_type,
        "ReplicationFactor": cluster.replication_factor,
        "Status": cluster.status,
        "TotalNodes": cluster.total_nodes,
        "ActiveNodes": cluster.active_nodes,
        "ClusterDiscoveryEndpoint": {
            "Address": cluster.cluster_discovery_endpoint,
            "Port": 8111,
        },
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_cluster(state: &DaxState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_name = payload["ClusterName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterName is required".to_string()))?
        .to_string();

    if state.clusters.contains_key(&cluster_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Cluster '{}' already exists",
            cluster_name
        )));
    }

    let node_type = payload["NodeType"]
        .as_str()
        .unwrap_or("dax.r5.large")
        .to_string();

    let replication_factor = payload["ReplicationFactor"].as_u64().unwrap_or(1);

    let cluster_arn = format!("arn:aws:dax:{REGION}:{ACCOUNT_ID}:cache/{cluster_name}");

    let endpoint = format!("{cluster_name}.{REGION}.dax-clusters.amazonaws.com");

    let cluster = DaxCluster {
        cluster_name: cluster_name.clone(),
        cluster_arn,
        node_type,
        replication_factor,
        status: "available".to_string(),
        total_nodes: replication_factor,
        active_nodes: replication_factor,
        cluster_discovery_endpoint: endpoint,
    };

    let resp = cluster_to_json(&cluster);
    state.clusters.insert(cluster_name, cluster);

    Ok(json_response(json!({
        "Cluster": resp,
    })))
}

fn delete_cluster(state: &DaxState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_name = payload["ClusterName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterName is required".to_string()))?;

    let (_, cluster) = state
        .clusters
        .remove(cluster_name)
        .ok_or_else(|| LawsError::NotFound(format!("Cluster '{}' not found", cluster_name)))?;

    Ok(json_response(json!({
        "Cluster": cluster_to_json(&cluster),
    })))
}

fn describe_clusters(state: &DaxState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_names = payload["ClusterNames"].as_array();

    let clusters: Vec<Value> = state
        .clusters
        .iter()
        .filter(|entry| match cluster_names {
            Some(names) => names
                .iter()
                .any(|n| n.as_str() == Some(entry.key().as_str())),
            None => true,
        })
        .map(|entry| cluster_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "Clusters": clusters })))
}

fn increase_replication_factor(state: &DaxState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_name = payload["ClusterName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterName is required".to_string()))?;

    let new_replication_factor = payload["NewReplicationFactor"]
        .as_u64()
        .ok_or_else(|| LawsError::InvalidRequest("NewReplicationFactor is required".to_string()))?;

    let mut cluster = state
        .clusters
        .get_mut(cluster_name)
        .ok_or_else(|| LawsError::NotFound(format!("Cluster '{}' not found", cluster_name)))?;

    if new_replication_factor <= cluster.replication_factor {
        return Err(LawsError::InvalidRequest(
            "NewReplicationFactor must be greater than current value".to_string(),
        ));
    }

    cluster.replication_factor = new_replication_factor;
    cluster.total_nodes = new_replication_factor;
    cluster.active_nodes = new_replication_factor;

    Ok(json_response(json!({
        "Cluster": cluster_to_json(&cluster),
    })))
}

fn decrease_replication_factor(state: &DaxState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_name = payload["ClusterName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterName is required".to_string()))?;

    let new_replication_factor = payload["NewReplicationFactor"]
        .as_u64()
        .ok_or_else(|| LawsError::InvalidRequest("NewReplicationFactor is required".to_string()))?;

    let mut cluster = state
        .clusters
        .get_mut(cluster_name)
        .ok_or_else(|| LawsError::NotFound(format!("Cluster '{}' not found", cluster_name)))?;

    if new_replication_factor >= cluster.replication_factor {
        return Err(LawsError::InvalidRequest(
            "NewReplicationFactor must be less than current value".to_string(),
        ));
    }

    if new_replication_factor < 1 {
        return Err(LawsError::InvalidRequest(
            "NewReplicationFactor must be at least 1".to_string(),
        ));
    }

    cluster.replication_factor = new_replication_factor;
    cluster.total_nodes = new_replication_factor;
    cluster.active_nodes = new_replication_factor;

    Ok(json_response(json!({
        "Cluster": cluster_to_json(&cluster),
    })))
}
