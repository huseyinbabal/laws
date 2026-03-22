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
pub struct NeptuneCluster {
    pub cluster_id: String,
    pub arn: String,
    pub engine: String,
    pub engine_version: String,
    pub status: String,
    pub endpoint: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct NeptuneInstance {
    pub instance_id: String,
    pub arn: String,
    pub cluster_id: String,
    pub instance_class: String,
    pub engine: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct NeptuneState {
    pub clusters: DashMap<String, NeptuneCluster>,
    pub instances: DashMap<String, NeptuneInstance>,
}

impl Default for NeptuneState {
    fn default() -> Self {
        Self {
            clusters: DashMap::new(),
            instances: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &NeptuneState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AmazonNeptuneV20171115.")
        .unwrap_or(target);

    let result = match action {
        "CreateDBCluster" => create_db_cluster(state, payload),
        "DeleteDBCluster" => delete_db_cluster(state, payload),
        "DescribeDBClusters" => describe_db_clusters(state),
        "CreateDBInstance" => create_db_instance(state, payload),
        "DeleteDBInstance" => delete_db_instance(state, payload),
        "DescribeDBInstances" => describe_db_instances(state),
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

fn create_db_cluster(
    state: &NeptuneState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let cluster_id = payload["DBClusterIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DBClusterIdentifier".into()))?
        .to_string();

    let engine = payload["Engine"]
        .as_str()
        .unwrap_or("neptune")
        .to_string();

    let engine_version = payload["EngineVersion"]
        .as_str()
        .unwrap_or("1.2.0.0")
        .to_string();

    let arn = format!(
        "arn:aws:rds:{REGION}:{ACCOUNT_ID}:cluster:{cluster_id}"
    );
    let endpoint = format!("{cluster_id}.cluster-abc123.{REGION}.neptune.amazonaws.com");
    let now = chrono::Utc::now().to_rfc3339();

    let cluster = NeptuneCluster {
        cluster_id: cluster_id.clone(),
        arn: arn.clone(),
        engine: engine.clone(),
        engine_version: engine_version.clone(),
        status: "available".to_string(),
        endpoint: endpoint.clone(),
        created_at: now.clone(),
    };

    state.clusters.insert(cluster_id.clone(), cluster);

    Ok(json_response(json!({
        "DBCluster": {
            "DBClusterIdentifier": cluster_id,
            "DBClusterArn": arn,
            "Engine": engine,
            "EngineVersion": engine_version,
            "Status": "available",
            "Endpoint": endpoint,
            "ClusterCreateTime": now,
        }
    })))
}

fn delete_db_cluster(
    state: &NeptuneState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let cluster_id = payload["DBClusterIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DBClusterIdentifier".into()))?;

    let (_, cluster) = state
        .clusters
        .remove(cluster_id)
        .ok_or_else(|| {
            LawsError::NotFound(format!("DBCluster '{}' not found", cluster_id))
        })?;

    Ok(json_response(json!({
        "DBCluster": {
            "DBClusterIdentifier": cluster.cluster_id,
            "DBClusterArn": cluster.arn,
            "Status": "deleting",
        }
    })))
}

fn describe_db_clusters(
    state: &NeptuneState,
) -> Result<Response, LawsError> {
    let clusters: Vec<Value> = state
        .clusters
        .iter()
        .map(|entry| {
            let c = entry.value();
            json!({
                "DBClusterIdentifier": c.cluster_id,
                "DBClusterArn": c.arn,
                "Engine": c.engine,
                "EngineVersion": c.engine_version,
                "Status": c.status,
                "Endpoint": c.endpoint,
                "ClusterCreateTime": c.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({
        "DBClusters": clusters
    })))
}

fn create_db_instance(
    state: &NeptuneState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let instance_id = payload["DBInstanceIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DBInstanceIdentifier".into()))?
        .to_string();

    let cluster_id = payload["DBClusterIdentifier"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let instance_class = payload["DBInstanceClass"]
        .as_str()
        .unwrap_or("db.r5.large")
        .to_string();

    let engine = payload["Engine"]
        .as_str()
        .unwrap_or("neptune")
        .to_string();

    let arn = format!(
        "arn:aws:rds:{REGION}:{ACCOUNT_ID}:db:{instance_id}"
    );
    let now = chrono::Utc::now().to_rfc3339();

    let instance = NeptuneInstance {
        instance_id: instance_id.clone(),
        arn: arn.clone(),
        cluster_id: cluster_id.clone(),
        instance_class: instance_class.clone(),
        engine: engine.clone(),
        status: "available".to_string(),
        created_at: now.clone(),
    };

    state.instances.insert(instance_id.clone(), instance);

    Ok(json_response(json!({
        "DBInstance": {
            "DBInstanceIdentifier": instance_id,
            "DBInstanceArn": arn,
            "DBClusterIdentifier": cluster_id,
            "DBInstanceClass": instance_class,
            "Engine": engine,
            "DBInstanceStatus": "available",
            "InstanceCreateTime": now,
        }
    })))
}

fn delete_db_instance(
    state: &NeptuneState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let instance_id = payload["DBInstanceIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DBInstanceIdentifier".into()))?;

    let (_, instance) = state
        .instances
        .remove(instance_id)
        .ok_or_else(|| {
            LawsError::NotFound(format!("DBInstance '{}' not found", instance_id))
        })?;

    Ok(json_response(json!({
        "DBInstance": {
            "DBInstanceIdentifier": instance.instance_id,
            "DBInstanceArn": instance.arn,
            "DBInstanceStatus": "deleting",
        }
    })))
}

fn describe_db_instances(
    state: &NeptuneState,
) -> Result<Response, LawsError> {
    let instances: Vec<Value> = state
        .instances
        .iter()
        .map(|entry| {
            let i = entry.value();
            json!({
                "DBInstanceIdentifier": i.instance_id,
                "DBInstanceArn": i.arn,
                "DBClusterIdentifier": i.cluster_id,
                "DBInstanceClass": i.instance_class,
                "Engine": i.engine,
                "DBInstanceStatus": i.status,
                "InstanceCreateTime": i.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({
        "DBInstances": instances
    })))
}
