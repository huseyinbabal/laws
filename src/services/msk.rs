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
pub struct MskCluster {
    pub cluster_name: String,
    pub arn: String,
    pub state: String,
    pub kafka_version: String,
    pub number_of_broker_nodes: u32,
    pub broker_node_group_info: Value,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct MskState {
    pub clusters: DashMap<String, MskCluster>,
}

impl Default for MskState {
    fn default() -> Self {
        Self {
            clusters: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &MskState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("Kafka.").unwrap_or(target);

    let result = match action {
        "CreateCluster" => create_cluster(state, payload),
        "DeleteCluster" => delete_cluster(state, payload),
        "ListClusters" => list_clusters(state),
        "DescribeCluster" => describe_cluster(state, payload),
        "ListNodes" => list_nodes(state, payload),
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

fn require_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, LawsError> {
    body.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest(format!("missing required field: {field}")))
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_cluster(state: &MskState, body: &Value) -> Result<Response, LawsError> {
    let cluster_name = require_str(body, "ClusterName")?.to_owned();
    let kafka_version = body
        .get("KafkaVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("3.5.1")
        .to_owned();
    let number_of_broker_nodes = body
        .get("NumberOfBrokerNodes")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as u32;
    let broker_node_group_info = body
        .get("BrokerNodeGroupInfo")
        .cloned()
        .unwrap_or(json!({}));

    if state.clusters.contains_key(&cluster_name) {
        return Err(LawsError::AlreadyExists(format!(
            "cluster already exists: {cluster_name}"
        )));
    }

    let cluster_uuid = uuid::Uuid::new_v4();
    let arn = format!("arn:aws:kafka:{REGION}:{ACCOUNT_ID}:cluster/{cluster_name}/{cluster_uuid}");
    let created_at = chrono::Utc::now().to_rfc3339();

    let cluster = MskCluster {
        cluster_name: cluster_name.clone(),
        arn: arn.clone(),
        state: "ACTIVE".into(),
        kafka_version,
        number_of_broker_nodes,
        broker_node_group_info,
        created_at,
    };

    let name = cluster_name.clone();
    state.clusters.insert(cluster_name, cluster);

    Ok(json_response(json!({
        "ClusterArn": arn,
        "ClusterName": name,
        "State": "CREATING"
    })))
}

fn delete_cluster(state: &MskState, body: &Value) -> Result<Response, LawsError> {
    let cluster_arn = require_str(body, "ClusterArn")?;
    let found = state
        .clusters
        .iter()
        .find(|e| e.value().arn == cluster_arn)
        .map(|e| e.key().clone());
    match found {
        Some(name) => {
            state.clusters.remove(&name);
            Ok(json_response(json!({
                "ClusterArn": cluster_arn,
                "State": "DELETING"
            })))
        }
        None => Err(LawsError::NotFound(format!(
            "cluster not found: {cluster_arn}"
        ))),
    }
}

fn list_clusters(state: &MskState) -> Result<Response, LawsError> {
    let cluster_list: Vec<Value> = state
        .clusters
        .iter()
        .map(|entry| {
            let c = entry.value();
            json!({
                "ClusterArn": c.arn,
                "ClusterName": c.cluster_name,
                "State": c.state,
                "KafkaVersion": c.kafka_version,
                "NumberOfBrokerNodes": c.number_of_broker_nodes,
                "CreationTime": c.created_at
            })
        })
        .collect();

    Ok(json_response(json!({
        "ClusterInfoList": cluster_list
    })))
}

fn describe_cluster(state: &MskState, body: &Value) -> Result<Response, LawsError> {
    let cluster_arn = require_str(body, "ClusterArn")?;
    let cluster = state
        .clusters
        .iter()
        .find(|e| e.value().arn == cluster_arn)
        .ok_or_else(|| LawsError::NotFound(format!("cluster not found: {cluster_arn}")))?;

    let c = cluster.value();
    Ok(json_response(json!({
        "ClusterInfo": {
            "ClusterArn": c.arn,
            "ClusterName": c.cluster_name,
            "State": c.state,
            "KafkaVersion": c.kafka_version,
            "NumberOfBrokerNodes": c.number_of_broker_nodes,
            "BrokerNodeGroupInfo": c.broker_node_group_info,
            "CreationTime": c.created_at
        }
    })))
}

fn list_nodes(state: &MskState, body: &Value) -> Result<Response, LawsError> {
    let cluster_arn = require_str(body, "ClusterArn")?;
    let cluster = state
        .clusters
        .iter()
        .find(|e| e.value().arn == cluster_arn)
        .ok_or_else(|| LawsError::NotFound(format!("cluster not found: {cluster_arn}")))?;

    let c = cluster.value();
    let mut nodes = Vec::new();
    for i in 0..c.number_of_broker_nodes {
        nodes.push(json!({
            "NodeType": "BROKER",
            "NodeARN": format!("{}/broker/{}", c.arn, i + 1),
            "NodeInfo": {
                "BrokerNodeInfo": {
                    "BrokerId": i + 1,
                    "Endpoints": [format!("b-{}.kafka.{REGION}.amazonaws.com", i + 1)],
                    "CurrentBrokerSoftwareInfo": {
                        "KafkaVersion": c.kafka_version
                    }
                }
            }
        }));
    }

    Ok(json_response(json!({
        "NodeInfoList": nodes
    })))
}
