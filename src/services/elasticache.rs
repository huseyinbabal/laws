use axum::body::Bytes;
use axum::http::{HeaderMap, Uri};
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::query::{parse_query_request, xml_error_response, xml_response};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CacheCluster {
    pub cache_cluster_id: String,
    pub engine: String,
    pub cache_node_type: String,
    pub num_cache_nodes: u32,
    pub status: String,
    pub arn: String,
}

#[derive(Debug, Clone)]
pub struct ReplicationGroup {
    pub replication_group_id: String,
    pub description: String,
    pub status: String,
    pub arn: String,
    pub num_node_groups: u32,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ElastiCacheState {
    pub clusters: DashMap<String, CacheCluster>,
    pub replication_groups: DashMap<String, ReplicationGroup>,
}

impl Default for ElastiCacheState {
    fn default() -> Self {
        Self {
            clusters: DashMap::new(),
            replication_groups: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &ElastiCacheState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target
        .strip_prefix("AmazonElastiCacheV9.")
        .unwrap_or(target);

    let result = match action {
        "CreateCacheCluster" => create_cache_cluster(state, payload),
        "DeleteCacheCluster" => delete_cache_cluster(state, payload),
        "DescribeCacheClusters" => describe_cache_clusters(state, payload),
        "CreateReplicationGroup" => create_replication_group(state, payload),
        "DeleteReplicationGroup" => delete_replication_group(state, payload),
        "DescribeReplicationGroups" => describe_replication_groups(state, payload),
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

fn cache_cluster_to_json(c: &CacheCluster) -> Value {
    json!({
        "CacheClusterId": c.cache_cluster_id,
        "Engine": c.engine,
        "CacheNodeType": c.cache_node_type,
        "NumCacheNodes": c.num_cache_nodes,
        "CacheClusterStatus": c.status,
        "ARN": c.arn
    })
}

fn replication_group_to_json(rg: &ReplicationGroup) -> Value {
    json!({
        "ReplicationGroupId": rg.replication_group_id,
        "Description": rg.description,
        "Status": rg.status,
        "ARN": rg.arn,
        "NumNodeGroups": rg.num_node_groups
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_cache_cluster(state: &ElastiCacheState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["CacheClusterId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("CacheClusterId is required".to_string()))?
        .to_string();

    if state.clusters.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "Cache cluster '{}' already exists",
            id
        )));
    }

    let engine = payload["Engine"]
        .as_str()
        .unwrap_or("redis")
        .to_string();
    let cache_node_type = payload["CacheNodeType"]
        .as_str()
        .unwrap_or("cache.t3.micro")
        .to_string();
    let num_cache_nodes = payload["NumCacheNodes"]
        .as_u64()
        .unwrap_or(1) as u32;
    let arn = format!("arn:aws:elasticache:{REGION}:{ACCOUNT_ID}:cluster:{id}");

    let cluster = CacheCluster {
        cache_cluster_id: id.clone(),
        engine,
        cache_node_type,
        num_cache_nodes,
        status: "available".to_string(),
        arn,
    };

    let resp = cache_cluster_to_json(&cluster);
    state.clusters.insert(id, cluster);

    Ok(json_response(json!({ "CacheCluster": resp })))
}

fn delete_cache_cluster(state: &ElastiCacheState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["CacheClusterId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("CacheClusterId is required".to_string()))?;

    let (_, cluster) = state
        .clusters
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Cache cluster '{}' not found", id)))?;

    Ok(json_response(json!({ "CacheCluster": cache_cluster_to_json(&cluster) })))
}

fn describe_cache_clusters(state: &ElastiCacheState, payload: &Value) -> Result<Response, LawsError> {
    let filter_id = payload["CacheClusterId"].as_str();

    let clusters: Vec<Value> = state
        .clusters
        .iter()
        .filter(|entry| {
            filter_id
                .map(|fid| entry.key() == fid)
                .unwrap_or(true)
        })
        .map(|entry| cache_cluster_to_json(entry.value()))
        .collect();

    if let Some(fid) = filter_id {
        if clusters.is_empty() {
            return Err(LawsError::NotFound(format!(
                "Cache cluster '{}' not found",
                fid
            )));
        }
    }

    Ok(json_response(json!({ "CacheClusters": clusters })))
}

fn create_replication_group(state: &ElastiCacheState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["ReplicationGroupId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ReplicationGroupId is required".to_string()))?
        .to_string();

    if state.replication_groups.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "Replication group '{}' already exists",
            id
        )));
    }

    let description = payload["ReplicationGroupDescription"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let num_node_groups = payload["NumNodeGroups"]
        .as_u64()
        .unwrap_or(1) as u32;
    let arn = format!("arn:aws:elasticache:{REGION}:{ACCOUNT_ID}:replicationgroup:{id}");

    let rg = ReplicationGroup {
        replication_group_id: id.clone(),
        description,
        status: "available".to_string(),
        arn,
        num_node_groups,
    };

    let resp = replication_group_to_json(&rg);
    state.replication_groups.insert(id, rg);

    Ok(json_response(json!({ "ReplicationGroup": resp })))
}

fn delete_replication_group(state: &ElastiCacheState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["ReplicationGroupId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ReplicationGroupId is required".to_string()))?;

    let (_, rg) = state
        .replication_groups
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Replication group '{}' not found", id)))?;

    Ok(json_response(json!({ "ReplicationGroup": replication_group_to_json(&rg) })))
}

fn describe_replication_groups(state: &ElastiCacheState, payload: &Value) -> Result<Response, LawsError> {
    let filter_id = payload["ReplicationGroupId"].as_str();

    let groups: Vec<Value> = state
        .replication_groups
        .iter()
        .filter(|entry| {
            filter_id
                .map(|fid| entry.key() == fid)
                .unwrap_or(true)
        })
        .map(|entry| replication_group_to_json(entry.value()))
        .collect();

    if let Some(fid) = filter_id {
        if groups.is_empty() {
            return Err(LawsError::NotFound(format!(
                "Replication group '{}' not found",
                fid
            )));
        }
    }

    Ok(json_response(json!({ "ReplicationGroups": groups })))
}

// ---------------------------------------------------------------------------
// XML helpers for query protocol
// ---------------------------------------------------------------------------

fn cache_cluster_to_xml(c: &CacheCluster) -> String {
    format!(
        "<CacheCluster>\
            <CacheClusterId>{id}</CacheClusterId>\
            <Engine>{engine}</Engine>\
            <CacheNodeType>{node_type}</CacheNodeType>\
            <NumCacheNodes>{num}</NumCacheNodes>\
            <CacheClusterStatus>{status}</CacheClusterStatus>\
            <ARN>{arn}</ARN>\
        </CacheCluster>",
        id = c.cache_cluster_id,
        engine = c.engine,
        node_type = c.cache_node_type,
        num = c.num_cache_nodes,
        status = c.status,
        arn = c.arn,
    )
}

fn replication_group_to_xml(rg: &ReplicationGroup) -> String {
    format!(
        "<ReplicationGroup>\
            <ReplicationGroupId>{id}</ReplicationGroupId>\
            <Description>{desc}</Description>\
            <Status>{status}</Status>\
            <ARN>{arn}</ARN>\
            <NumNodeGroups>{num}</NumNodeGroups>\
        </ReplicationGroup>",
        id = rg.replication_group_id,
        desc = rg.description,
        status = rg.status,
        arn = rg.arn,
        num = rg.num_node_groups,
    )
}

// ---------------------------------------------------------------------------
// Query protocol handler (XML responses for taws compatibility)
// ---------------------------------------------------------------------------

pub fn handle_query_request(
    state: &ElastiCacheState,
    headers: &HeaderMap,
    body: &Bytes,
    uri: &Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "CreateCacheCluster" => query_create_cache_cluster(state, &req.params),
        "DeleteCacheCluster" => query_delete_cache_cluster(state, &req.params),
        "DescribeCacheClusters" => query_describe_cache_clusters(state, &req.params),
        "CreateReplicationGroup" => query_create_replication_group(state, &req.params),
        "DeleteReplicationGroup" => query_delete_replication_group(state, &req.params),
        "DescribeReplicationGroups" => query_describe_replication_groups(state, &req.params),
        _ => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            req.action
        ))),
    };

    match result {
        Ok(resp) => resp,
        Err(e) => xml_error_response(&e),
    }
}

// ---------------------------------------------------------------------------
// Query protocol operations (XML)
// ---------------------------------------------------------------------------

fn query_create_cache_cluster(
    state: &ElastiCacheState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let id = params
        .get("CacheClusterId")
        .ok_or_else(|| LawsError::InvalidRequest("CacheClusterId is required".to_string()))?
        .to_string();

    if state.clusters.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "Cache cluster '{}' already exists",
            id
        )));
    }

    let engine = params.get("Engine").cloned().unwrap_or_else(|| "redis".into());
    let cache_node_type = params.get("CacheNodeType").cloned().unwrap_or_else(|| "cache.t3.micro".into());
    let num_cache_nodes = params
        .get("NumCacheNodes")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);
    let arn = format!("arn:aws:elasticache:{REGION}:{ACCOUNT_ID}:cluster:{id}");

    let cluster = CacheCluster {
        cache_cluster_id: id.clone(),
        engine,
        cache_node_type,
        num_cache_nodes,
        status: "available".to_string(),
        arn,
    };

    let xml = cache_cluster_to_xml(&cluster);
    state.clusters.insert(id, cluster);

    Ok(xml_response("CreateCacheCluster", &format!("<CacheCluster>{}</CacheCluster>", xml)))
}

fn query_delete_cache_cluster(
    state: &ElastiCacheState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let id = params
        .get("CacheClusterId")
        .ok_or_else(|| LawsError::InvalidRequest("CacheClusterId is required".to_string()))?;

    let (_, cluster) = state
        .clusters
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Cache cluster '{}' not found", id)))?;

    let xml = cache_cluster_to_xml(&cluster);
    Ok(xml_response("DeleteCacheCluster", &format!("<CacheCluster>{}</CacheCluster>", xml)))
}

fn query_describe_cache_clusters(
    state: &ElastiCacheState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let filter_id = params.get("CacheClusterId").map(|s| s.as_str());

    let clusters: Vec<String> = state
        .clusters
        .iter()
        .filter(|entry| {
            filter_id
                .map(|fid| entry.key() == fid)
                .unwrap_or(true)
        })
        .map(|entry| cache_cluster_to_xml(entry.value()))
        .collect();

    if let Some(fid) = filter_id {
        if clusters.is_empty() {
            return Err(LawsError::NotFound(format!(
                "Cache cluster '{}' not found",
                fid
            )));
        }
    }

    let inner = format!("<CacheClusters>{}</CacheClusters>", clusters.join(""));
    Ok(xml_response("DescribeCacheClusters", &inner))
}

fn query_create_replication_group(
    state: &ElastiCacheState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let id = params
        .get("ReplicationGroupId")
        .ok_or_else(|| LawsError::InvalidRequest("ReplicationGroupId is required".to_string()))?
        .to_string();

    if state.replication_groups.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "Replication group '{}' already exists",
            id
        )));
    }

    let description = params.get("ReplicationGroupDescription").cloned().unwrap_or_default();
    let num_node_groups = params
        .get("NumNodeGroups")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);
    let arn = format!("arn:aws:elasticache:{REGION}:{ACCOUNT_ID}:replicationgroup:{id}");

    let rg = ReplicationGroup {
        replication_group_id: id.clone(),
        description,
        status: "available".to_string(),
        arn,
        num_node_groups,
    };

    let xml = replication_group_to_xml(&rg);
    state.replication_groups.insert(id, rg);

    Ok(xml_response("CreateReplicationGroup", &format!("<ReplicationGroup>{}</ReplicationGroup>", xml)))
}

fn query_delete_replication_group(
    state: &ElastiCacheState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let id = params
        .get("ReplicationGroupId")
        .ok_or_else(|| LawsError::InvalidRequest("ReplicationGroupId is required".to_string()))?;

    let (_, rg) = state
        .replication_groups
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("Replication group '{}' not found", id)))?;

    let xml = replication_group_to_xml(&rg);
    Ok(xml_response("DeleteReplicationGroup", &format!("<ReplicationGroup>{}</ReplicationGroup>", xml)))
}

fn query_describe_replication_groups(
    state: &ElastiCacheState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let filter_id = params.get("ReplicationGroupId").map(|s| s.as_str());

    let groups: Vec<String> = state
        .replication_groups
        .iter()
        .filter(|entry| {
            filter_id
                .map(|fid| entry.key() == fid)
                .unwrap_or(true)
        })
        .map(|entry| replication_group_to_xml(entry.value()))
        .collect();

    if let Some(fid) = filter_id {
        if groups.is_empty() {
            return Err(LawsError::NotFound(format!(
                "Replication group '{}' not found",
                fid
            )));
        }
    }

    let inner = format!("<ReplicationGroups>{}</ReplicationGroups>", groups.join(""));
    Ok(xml_response("DescribeReplicationGroups", &inner))
}
