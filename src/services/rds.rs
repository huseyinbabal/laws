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
pub struct DbInstance {
    pub db_instance_identifier: String,
    pub db_instance_class: String,
    pub engine: String,
    pub status: String,
    pub arn: String,
    pub endpoint: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct DbCluster {
    pub db_cluster_identifier: String,
    pub engine: String,
    pub status: String,
    pub arn: String,
    pub endpoint: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct RdsState {
    pub instances: DashMap<String, DbInstance>,
    pub clusters: DashMap<String, DbCluster>,
}

impl Default for RdsState {
    fn default() -> Self {
        Self {
            instances: DashMap::new(),
            clusters: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &RdsState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("AmazonRDSv19.").unwrap_or(target);

    let result = match action {
        "CreateDBInstance" => create_db_instance(state, payload),
        "DeleteDBInstance" => delete_db_instance(state, payload),
        "DescribeDBInstances" => describe_db_instances(state, payload),
        "StartDBInstance" => start_db_instance(state, payload),
        "StopDBInstance" => stop_db_instance(state, payload),
        "RebootDBInstance" => reboot_db_instance(state, payload),
        "CreateDBCluster" => create_db_cluster(state, payload),
        "DeleteDBCluster" => delete_db_cluster(state, payload),
        "DescribeDBClusters" => describe_db_clusters(state, payload),
        "DescribeDBSnapshots" => describe_db_snapshots(state, payload),
        "DeleteDBSnapshot" => delete_db_snapshot(state, payload),
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

fn default_port_for_engine(engine: &str) -> u16 {
    match engine {
        "postgres" | "aurora-postgresql" => 5432,
        _ => 3306,
    }
}

fn instance_to_json(inst: &DbInstance) -> Value {
    json!({
        "DBInstanceIdentifier": inst.db_instance_identifier,
        "DBInstanceClass": inst.db_instance_class,
        "Engine": inst.engine,
        "DBInstanceStatus": inst.status,
        "DBInstanceArn": inst.arn,
        "Endpoint": {
            "Address": inst.endpoint,
            "Port": inst.port
        }
    })
}

fn cluster_to_json(c: &DbCluster) -> Value {
    json!({
        "DBClusterIdentifier": c.db_cluster_identifier,
        "Engine": c.engine,
        "Status": c.status,
        "DBClusterArn": c.arn,
        "Endpoint": c.endpoint
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_db_instance(state: &RdsState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DBInstanceIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string()))?
        .to_string();

    if state.instances.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "DB instance '{}' already exists",
            id
        )));
    }

    let engine = payload["Engine"].as_str().unwrap_or("mysql").to_string();
    let db_instance_class = payload["DBInstanceClass"]
        .as_str()
        .unwrap_or("db.t3.micro")
        .to_string();
    let port = payload["Port"]
        .as_u64()
        .map(|p| p as u16)
        .unwrap_or_else(|| default_port_for_engine(&engine));

    let arn = format!("arn:aws:rds:{REGION}:{ACCOUNT_ID}:db:{id}");
    let endpoint = format!("{id}.{ACCOUNT_ID}.{REGION}.rds.amazonaws.com");

    let instance = DbInstance {
        db_instance_identifier: id.clone(),
        db_instance_class,
        engine,
        status: "available".to_string(),
        arn,
        endpoint,
        port,
    };

    let resp = instance_to_json(&instance);
    state.instances.insert(id, instance);

    Ok(json_response(json!({ "DBInstance": resp })))
}

fn delete_db_instance(state: &RdsState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DBInstanceIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string()))?;

    let (_, instance) = state
        .instances
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("DB instance '{}' not found", id)))?;

    Ok(json_response(
        json!({ "DBInstance": instance_to_json(&instance) }),
    ))
}

fn describe_db_instances(state: &RdsState, payload: &Value) -> Result<Response, LawsError> {
    let filter_id = payload["DBInstanceIdentifier"].as_str();

    let instances: Vec<Value> = state
        .instances
        .iter()
        .filter(|entry| filter_id.map(|fid| entry.key() == fid).unwrap_or(true))
        .map(|entry| instance_to_json(entry.value()))
        .collect();

    if let Some(fid) = filter_id {
        if instances.is_empty() {
            return Err(LawsError::NotFound(format!(
                "DB instance '{}' not found",
                fid
            )));
        }
    }

    Ok(json_response(json!({ "DBInstances": instances })))
}

fn start_db_instance(state: &RdsState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DBInstanceIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string()))?;

    let mut instance = state
        .instances
        .get_mut(id)
        .ok_or_else(|| LawsError::NotFound(format!("DB instance '{}' not found", id)))?;

    instance.status = "available".to_string();
    let resp = instance_to_json(&instance);

    Ok(json_response(json!({ "DBInstance": resp })))
}

fn stop_db_instance(state: &RdsState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DBInstanceIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string()))?;

    let mut instance = state
        .instances
        .get_mut(id)
        .ok_or_else(|| LawsError::NotFound(format!("DB instance '{}' not found", id)))?;

    instance.status = "stopped".to_string();
    let resp = instance_to_json(&instance);

    Ok(json_response(json!({ "DBInstance": resp })))
}

fn create_db_cluster(state: &RdsState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DBClusterIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DBClusterIdentifier is required".to_string()))?
        .to_string();

    if state.clusters.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "DB cluster '{}' already exists",
            id
        )));
    }

    let engine = payload["Engine"]
        .as_str()
        .unwrap_or("aurora-mysql")
        .to_string();
    let arn = format!("arn:aws:rds:{REGION}:{ACCOUNT_ID}:cluster:{id}");
    let endpoint = format!("{id}.cluster-{ACCOUNT_ID}.{REGION}.rds.amazonaws.com");

    let cluster = DbCluster {
        db_cluster_identifier: id.clone(),
        engine,
        status: "available".to_string(),
        arn,
        endpoint,
    };

    let resp = cluster_to_json(&cluster);
    state.clusters.insert(id, cluster);

    Ok(json_response(json!({ "DBCluster": resp })))
}

fn delete_db_cluster(state: &RdsState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DBClusterIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DBClusterIdentifier is required".to_string()))?;

    let (_, cluster) = state
        .clusters
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("DB cluster '{}' not found", id)))?;

    Ok(json_response(
        json!({ "DBCluster": cluster_to_json(&cluster) }),
    ))
}

fn describe_db_clusters(state: &RdsState, payload: &Value) -> Result<Response, LawsError> {
    let filter_id = payload["DBClusterIdentifier"].as_str();

    let clusters: Vec<Value> = state
        .clusters
        .iter()
        .filter(|entry| filter_id.map(|fid| entry.key() == fid).unwrap_or(true))
        .map(|entry| cluster_to_json(entry.value()))
        .collect();

    if let Some(fid) = filter_id {
        if clusters.is_empty() {
            return Err(LawsError::NotFound(format!(
                "DB cluster '{}' not found",
                fid
            )));
        }
    }

    Ok(json_response(json!({ "DBClusters": clusters })))
}

fn reboot_db_instance(state: &RdsState, payload: &Value) -> Result<Response, LawsError> {
    let id = payload["DBInstanceIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string()))?;

    let instance = state
        .instances
        .get(id)
        .ok_or_else(|| LawsError::NotFound(format!("DB instance '{}' not found", id)))?;

    // Reboot doesn't change state in mock — instance stays available
    let resp = instance_to_json(&instance);

    Ok(json_response(json!({ "DBInstance": resp })))
}

fn describe_db_snapshots(_state: &RdsState, payload: &Value) -> Result<Response, LawsError> {
    let _filter_id = payload["DBInstanceIdentifier"].as_str();
    // Mock: return empty snapshots list
    Ok(json_response(json!({ "DBSnapshots": [] })))
}

fn delete_db_snapshot(_state: &RdsState, payload: &Value) -> Result<Response, LawsError> {
    let snap_id = payload["DBSnapshotIdentifier"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("DBSnapshotIdentifier is required".to_string()))?;

    // Mock: just acknowledge the delete
    Ok(json_response(
        json!({ "DBSnapshot": { "DBSnapshotIdentifier": snap_id, "Status": "deleted" } }),
    ))
}

// ---------------------------------------------------------------------------
// Query protocol handler (XML responses for taws compatibility)
// ---------------------------------------------------------------------------

pub fn handle_query_request(
    state: &RdsState,
    headers: &HeaderMap,
    body: &Bytes,
    uri: &Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "DescribeDBInstances" => query_describe_db_instances(state, &req.params),
        "StartDBInstance" => query_start_db_instance(state, &req.params),
        "StopDBInstance" => query_stop_db_instance(state, &req.params),
        "RebootDBInstance" => query_reboot_db_instance(state, &req.params),
        "DeleteDBInstance" => query_delete_db_instance(state, &req.params),
        "DescribeDBSnapshots" => query_describe_db_snapshots(state, &req.params),
        "DeleteDBSnapshot" => query_delete_db_snapshot(state, &req.params),
        "CreateDBInstance" => query_create_db_instance(state, &req.params),
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

fn instance_to_xml(inst: &DbInstance) -> String {
    format!(
        "<DBInstance>\
            <DBInstanceIdentifier>{id}</DBInstanceIdentifier>\
            <DBInstanceClass>{class}</DBInstanceClass>\
            <Engine>{engine}</Engine>\
            <DBInstanceStatus>{status}</DBInstanceStatus>\
            <DBInstanceArn>{arn}</DBInstanceArn>\
            <Endpoint><Address>{endpoint}</Address><Port>{port}</Port></Endpoint>\
        </DBInstance>",
        id = inst.db_instance_identifier,
        class = inst.db_instance_class,
        engine = inst.engine,
        status = inst.status,
        arn = inst.arn,
        endpoint = inst.endpoint,
        port = inst.port,
    )
}

fn query_describe_db_instances(
    state: &RdsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let filter_id = params.get("DBInstanceIdentifier").map(|s| s.as_str());

    let instances: Vec<String> = state
        .instances
        .iter()
        .filter(|entry| filter_id.map(|fid| entry.key() == fid).unwrap_or(true))
        .map(|entry| instance_to_xml(entry.value()))
        .collect();

    if let Some(fid) = filter_id {
        if instances.is_empty() {
            return Err(LawsError::NotFound(format!(
                "DB instance '{}' not found",
                fid
            )));
        }
    }

    let inner = format!("<DBInstances>{}</DBInstances>", instances.join(""));
    Ok(xml_response("DescribeDBInstances", &inner))
}

fn query_start_db_instance(
    state: &RdsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let id = params
        .get("DBInstanceIdentifier")
        .ok_or_else(|| LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string()))?;

    let mut instance = state
        .instances
        .get_mut(id)
        .ok_or_else(|| LawsError::NotFound(format!("DB instance '{}' not found", id)))?;

    instance.status = "available".to_string();
    let xml = instance_to_xml(&instance);

    Ok(xml_response("StartDBInstance", &xml))
}

fn query_stop_db_instance(
    state: &RdsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let id = params
        .get("DBInstanceIdentifier")
        .ok_or_else(|| LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string()))?;

    let mut instance = state
        .instances
        .get_mut(id)
        .ok_or_else(|| LawsError::NotFound(format!("DB instance '{}' not found", id)))?;

    instance.status = "stopped".to_string();
    let xml = instance_to_xml(&instance);

    Ok(xml_response("StopDBInstance", &xml))
}

fn query_reboot_db_instance(
    state: &RdsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let id = params
        .get("DBInstanceIdentifier")
        .ok_or_else(|| LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string()))?;

    let instance = state
        .instances
        .get(id)
        .ok_or_else(|| LawsError::NotFound(format!("DB instance '{}' not found", id)))?;

    let xml = instance_to_xml(&instance);

    Ok(xml_response("RebootDBInstance", &xml))
}

fn query_delete_db_instance(
    state: &RdsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let id = params
        .get("DBInstanceIdentifier")
        .ok_or_else(|| LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string()))?;

    let (_, instance) = state
        .instances
        .remove(id)
        .ok_or_else(|| LawsError::NotFound(format!("DB instance '{}' not found", id)))?;

    let xml = instance_to_xml(&instance);

    Ok(xml_response("DeleteDBInstance", &xml))
}

fn query_describe_db_snapshots(
    _state: &RdsState,
    _params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let inner = "<DBSnapshots></DBSnapshots>";
    Ok(xml_response("DescribeDBSnapshots", inner))
}

fn query_delete_db_snapshot(
    _state: &RdsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let snap_id = params
        .get("DBSnapshotIdentifier")
        .ok_or_else(|| LawsError::InvalidRequest("DBSnapshotIdentifier is required".to_string()))?;

    let inner = format!(
        "<DBSnapshot><DBSnapshotIdentifier>{snap_id}</DBSnapshotIdentifier><Status>deleted</Status></DBSnapshot>"
    );
    Ok(xml_response("DeleteDBSnapshot", &inner))
}

fn query_create_db_instance(
    state: &RdsState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let id = params
        .get("DBInstanceIdentifier")
        .ok_or_else(|| LawsError::InvalidRequest("DBInstanceIdentifier is required".to_string()))?
        .to_string();

    if state.instances.contains_key(&id) {
        return Err(LawsError::AlreadyExists(format!(
            "DB instance '{}' already exists",
            id
        )));
    }

    let engine = params
        .get("Engine")
        .map(|s| s.as_str())
        .unwrap_or("mysql")
        .to_string();
    let db_instance_class = params
        .get("DBInstanceClass")
        .map(|s| s.as_str())
        .unwrap_or("db.t3.micro")
        .to_string();
    let port = params
        .get("Port")
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or_else(|| default_port_for_engine(&engine));

    let arn = format!("arn:aws:rds:{REGION}:{ACCOUNT_ID}:db:{id}");
    let endpoint = format!("{id}.{ACCOUNT_ID}.{REGION}.rds.amazonaws.com");

    let instance = DbInstance {
        db_instance_identifier: id.clone(),
        db_instance_class,
        engine,
        status: "available".to_string(),
        arn,
        endpoint,
        port,
    };

    let xml = instance_to_xml(&instance);
    state.instances.insert(id, instance);

    Ok(xml_response("CreateDBInstance", &xml))
}
