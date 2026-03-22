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
pub struct MemoryDbCluster {
    pub name: String,
    pub arn: String,
    pub status: String,
    pub node_type: String,
    pub num_shards: u64,
    pub num_replicas_per_shard: u64,
    pub engine_version: String,
}

#[derive(Debug, Clone)]
pub struct MemoryDbSnapshot {
    pub name: String,
    pub arn: String,
    pub cluster_name: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct MemoryDbState {
    pub clusters: DashMap<String, MemoryDbCluster>,
    pub snapshots: DashMap<String, MemoryDbSnapshot>,
}

impl Default for MemoryDbState {
    fn default() -> Self {
        Self {
            clusters: DashMap::new(),
            snapshots: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &MemoryDbState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AmazonMemoryDB.")
        .unwrap_or(target);

    let result = match action {
        "CreateCluster" => create_cluster(state, payload),
        "DeleteCluster" => delete_cluster(state, payload),
        "DescribeClusters" => describe_clusters(state, payload),
        "UpdateCluster" => update_cluster(state, payload),
        "CreateSnapshot" => create_snapshot(state, payload),
        "DescribeSnapshots" => describe_snapshots(state, payload),
        other => Err(LawsError::InvalidRequest(format!(
            "unknown action: {other}"
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

fn cluster_to_json(c: &MemoryDbCluster) -> Value {
    json!({
        "Name": c.name,
        "ARN": c.arn,
        "Status": c.status,
        "NodeType": c.node_type,
        "NumberOfShards": c.num_shards,
        "NumReplicasPerShard": c.num_replicas_per_shard,
        "EngineVersion": c.engine_version,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_cluster(state: &MemoryDbState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["ClusterName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterName is required".to_string()))?
        .to_string();

    if state.clusters.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Cluster '{}' already exists",
            name
        )));
    }

    let node_type = payload["NodeType"]
        .as_str()
        .unwrap_or("db.r6g.large")
        .to_string();

    let num_shards = payload["NumShards"]
        .as_u64()
        .unwrap_or(1);

    let num_replicas_per_shard = payload["NumReplicasPerShard"]
        .as_u64()
        .unwrap_or(1);

    let engine_version = payload["EngineVersion"]
        .as_str()
        .unwrap_or("7.0")
        .to_string();

    let arn = format!(
        "arn:aws:memorydb:{REGION}:{ACCOUNT_ID}:cluster/{name}"
    );

    let cluster = MemoryDbCluster {
        name: name.clone(),
        arn,
        status: "available".to_string(),
        node_type,
        num_shards,
        num_replicas_per_shard,
        engine_version,
    };

    let resp = cluster_to_json(&cluster);
    state.clusters.insert(name, cluster);

    Ok(json_response(json!({ "Cluster": resp })))
}

fn delete_cluster(state: &MemoryDbState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["ClusterName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterName is required".to_string()))?;

    let (_, cluster) = state
        .clusters
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Cluster '{}' not found", name)))?;

    Ok(json_response(json!({ "Cluster": cluster_to_json(&cluster) })))
}

fn describe_clusters(state: &MemoryDbState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_name = payload["ClusterName"].as_str();

    let clusters: Vec<Value> = state
        .clusters
        .iter()
        .filter(|entry| {
            cluster_name
                .map(|n| entry.key() == n)
                .unwrap_or(true)
        })
        .map(|entry| cluster_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "Clusters": clusters })))
}

fn update_cluster(state: &MemoryDbState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["ClusterName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterName is required".to_string()))?;

    let mut cluster = state
        .clusters
        .get_mut(name)
        .ok_or_else(|| LawsError::NotFound(format!("Cluster '{}' not found", name)))?;

    if let Some(node_type) = payload["NodeType"].as_str() {
        cluster.node_type = node_type.to_string();
    }

    if let Some(engine_version) = payload["EngineVersion"].as_str() {
        cluster.engine_version = engine_version.to_string();
    }

    Ok(json_response(json!({ "Cluster": cluster_to_json(&cluster) })))
}

fn create_snapshot(state: &MemoryDbState, payload: &Value) -> Result<Response, LawsError> {
    let snapshot_name = payload["SnapshotName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("SnapshotName is required".to_string()))?
        .to_string();

    let cluster_name = payload["ClusterName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterName is required".to_string()))?
        .to_string();

    if !state.clusters.contains_key(&cluster_name) {
        return Err(LawsError::NotFound(format!(
            "Cluster '{}' not found",
            cluster_name
        )));
    }

    let arn = format!(
        "arn:aws:memorydb:{REGION}:{ACCOUNT_ID}:snapshot/{snapshot_name}"
    );

    let snapshot = MemoryDbSnapshot {
        name: snapshot_name.clone(),
        arn,
        cluster_name,
        status: "available".to_string(),
    };

    let resp = json!({
        "Name": snapshot.name,
        "ARN": snapshot.arn,
        "ClusterName": snapshot.cluster_name,
        "Status": snapshot.status,
    });

    state.snapshots.insert(snapshot_name, snapshot);

    Ok(json_response(json!({ "Snapshot": resp })))
}

fn describe_snapshots(state: &MemoryDbState, payload: &Value) -> Result<Response, LawsError> {
    let snapshot_name = payload["SnapshotName"].as_str();

    let snapshots: Vec<Value> = state
        .snapshots
        .iter()
        .filter(|entry| {
            snapshot_name
                .map(|n| entry.key() == n)
                .unwrap_or(true)
        })
        .map(|entry| {
            let s = entry.value();
            json!({
                "Name": s.name,
                "ARN": s.arn,
                "ClusterName": s.cluster_name,
                "Status": s.status,
            })
        })
        .collect();

    Ok(json_response(json!({ "Snapshots": snapshots })))
}
