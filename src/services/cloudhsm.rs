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
pub struct Cluster {
    pub cluster_id: String,
    pub hsm_type: String,
    pub subnet_mapping: Value,
    pub state: String,
    pub created_at: String,
    pub hsms: Vec<Hsm>,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone)]
pub struct Hsm {
    pub hsm_id: String,
    pub cluster_id: String,
    pub availability_zone: String,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CloudHsmState {
    pub clusters: DashMap<String, Cluster>,
}

impl Default for CloudHsmState {
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
    state: &CloudHsmState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("BaldrApiService.")
        .unwrap_or(target);

    let result = match action {
        "CreateCluster" => create_cluster(state, payload),
        "DeleteCluster" => delete_cluster(state, payload),
        "DescribeClusters" => describe_clusters(state, payload),
        "CreateHsm" => create_hsm(state, payload),
        "DeleteHsm" => delete_hsm(state, payload),
        "ListTags" => list_tags(state, payload),
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

fn cluster_to_json(c: &Cluster) -> Value {
    let hsms: Vec<Value> = c
        .hsms
        .iter()
        .map(|h| {
            json!({
                "HsmId": h.hsm_id,
                "ClusterId": h.cluster_id,
                "AvailabilityZone": h.availability_zone,
                "State": h.state,
            })
        })
        .collect();

    json!({
        "ClusterId": c.cluster_id,
        "HsmType": c.hsm_type,
        "SubnetMapping": c.subnet_mapping,
        "State": c.state,
        "CreateTimestamp": c.created_at,
        "Hsms": hsms,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_cluster(state: &CloudHsmState, payload: &Value) -> Result<Response, LawsError> {
    let hsm_type = payload["HsmType"]
        .as_str()
        .unwrap_or("hsm1.medium")
        .to_string();

    let subnet_ids = payload["SubnetIds"].clone();

    let cluster_id = format!("cluster-{}", &uuid::Uuid::new_v4().to_string()[..17]);
    let now = chrono::Utc::now().to_rfc3339();

    let tags: Vec<Tag> = payload["TagList"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    Some(Tag {
                        key: t["Key"].as_str()?.to_string(),
                        value: t["Value"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let cluster = Cluster {
        cluster_id: cluster_id.clone(),
        hsm_type,
        subnet_mapping: subnet_ids,
        state: "CREATE_IN_PROGRESS".to_string(),
        created_at: now,
        hsms: Vec::new(),
        tags,
    };

    let resp = cluster_to_json(&cluster);
    state.clusters.insert(cluster_id, cluster);

    Ok(json_response(json!({ "Cluster": resp })))
}

fn delete_cluster(state: &CloudHsmState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_id = payload["ClusterId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterId is required".to_string()))?;

    let (_, cluster) = state
        .clusters
        .remove(cluster_id)
        .ok_or_else(|| LawsError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let mut resp = cluster_to_json(&cluster);
    resp["State"] = json!("DELETE_IN_PROGRESS");

    Ok(json_response(json!({ "Cluster": resp })))
}

fn describe_clusters(state: &CloudHsmState, payload: &Value) -> Result<Response, LawsError> {
    let filter_ids: Option<Vec<&str>> = payload["Filters"]["clusterIds"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect());

    let clusters: Vec<Value> = state
        .clusters
        .iter()
        .filter(|entry| {
            if let Some(ref ids) = filter_ids {
                ids.contains(&entry.value().cluster_id.as_str())
            } else {
                true
            }
        })
        .map(|entry| cluster_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "Clusters": clusters })))
}

fn create_hsm(state: &CloudHsmState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_id = payload["ClusterId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterId is required".to_string()))?;

    let az = payload["AvailabilityZone"]
        .as_str()
        .unwrap_or(&format!("{REGION}a"))
        .to_string();

    let mut cluster = state
        .clusters
        .get_mut(cluster_id)
        .ok_or_else(|| LawsError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let hsm_id = format!("hsm-{}", &uuid::Uuid::new_v4().to_string()[..17]);

    let hsm = Hsm {
        hsm_id: hsm_id.clone(),
        cluster_id: cluster_id.to_string(),
        availability_zone: az.clone(),
        state: "CREATE_IN_PROGRESS".to_string(),
    };

    cluster.hsms.push(hsm);

    Ok(json_response(json!({
        "Hsm": {
            "HsmId": hsm_id,
            "ClusterId": cluster_id,
            "AvailabilityZone": az,
            "State": "CREATE_IN_PROGRESS",
        }
    })))
}

fn delete_hsm(state: &CloudHsmState, payload: &Value) -> Result<Response, LawsError> {
    let cluster_id = payload["ClusterId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ClusterId is required".to_string()))?;

    let hsm_id = payload["HsmId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("HsmId is required".to_string()))?;

    let mut cluster = state
        .clusters
        .get_mut(cluster_id)
        .ok_or_else(|| LawsError::NotFound(format!("Cluster '{}' not found", cluster_id)))?;

    let original_len = cluster.hsms.len();
    cluster.hsms.retain(|h| h.hsm_id != hsm_id);

    if cluster.hsms.len() == original_len {
        return Err(LawsError::NotFound(format!("HSM '{}' not found", hsm_id)));
    }

    Ok(json_response(json!({ "HsmId": hsm_id })))
}

fn list_tags(state: &CloudHsmState, payload: &Value) -> Result<Response, LawsError> {
    let resource_id = payload["ResourceId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ResourceId is required".to_string()))?;

    let cluster = state
        .clusters
        .get(resource_id)
        .ok_or_else(|| LawsError::NotFound(format!("Resource '{}' not found", resource_id)))?;

    let tags: Vec<Value> = cluster
        .tags
        .iter()
        .map(|t| json!({ "Key": t.key, "Value": t.value }))
        .collect();

    Ok(json_response(json!({ "TagList": tags })))
}
