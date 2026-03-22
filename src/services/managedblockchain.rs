use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::Json;
use dashmap::DashMap;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::rest_json;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Network {
    pub id: String,
    pub name: String,
    pub framework: String,
    pub framework_version: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub network_id: String,
    pub member_id: String,
    pub instance_type: String,
    pub availability_zone: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ManagedBlockchainState {
    pub networks: DashMap<String, Network>,
    pub nodes: DashMap<String, Node>,
}

impl Default for ManagedBlockchainState {
    fn default() -> Self {
        Self {
            networks: DashMap::new(),
            nodes: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<ManagedBlockchainState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/networks",
            axum::routing::post(create_network).get(list_networks),
        )
        .route(
            "/networks/{network_id}",
            axum::routing::get(get_network),
        )
        .route(
            "/networks/{network_id}/nodes",
            axum::routing::post(create_node).get(list_nodes),
        )
        .route(
            "/networks/{network_id}/nodes/{node_id}",
            axum::routing::get(get_node).delete(delete_node),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_network(
    State(state): State<Arc<ManagedBlockchainState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("Name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let framework = payload
        .get("Framework")
        .and_then(|v| v.as_str())
        .unwrap_or("HYPERLEDGER_FABRIC")
        .to_owned();

    let framework_version = payload
        .get("FrameworkVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("1.4")
        .to_owned();

    let id = format!("n-{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..24].to_string());
    let now = chrono::Utc::now().to_rfc3339();

    let network = Network {
        id: id.clone(),
        name: name.clone(),
        framework: framework.clone(),
        framework_version: framework_version.clone(),
        status: "AVAILABLE".into(),
        created_at: now,
    };

    state.networks.insert(id.clone(), network);

    rest_json::created(json!({
        "NetworkId": id
    }))
}

async fn list_networks(
    State(state): State<Arc<ManagedBlockchainState>>,
) -> Response {
    let networks: Vec<Value> = state
        .networks
        .iter()
        .map(|e| {
            let n = e.value();
            json!({
                "Id": n.id,
                "Name": n.name,
                "Framework": n.framework,
                "FrameworkVersion": n.framework_version,
                "Status": n.status,
                "CreationDate": n.created_at
            })
        })
        .collect();

    rest_json::ok(json!({
        "Networks": networks
    }))
}

async fn get_network(
    State(state): State<Arc<ManagedBlockchainState>>,
    Path(network_id): Path<String>,
) -> Response {
    match state.networks.get(&network_id) {
        Some(n) => rest_json::ok(json!({
            "Network": {
                "Id": n.id,
                "Name": n.name,
                "Framework": n.framework,
                "FrameworkVersion": n.framework_version,
                "Status": n.status,
                "CreationDate": n.created_at
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Network not found: {network_id}"
        ))),
    }
}

async fn create_node(
    State(state): State<Arc<ManagedBlockchainState>>,
    Path(network_id): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    if !state.networks.contains_key(&network_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Network not found: {network_id}"
        )));
    }

    let member_id = payload
        .get("MemberId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let instance_type = payload
        .get("NodeConfiguration")
        .and_then(|v| v.get("InstanceType"))
        .and_then(|v| v.as_str())
        .unwrap_or("bc.t3.small")
        .to_owned();

    let availability_zone = payload
        .get("NodeConfiguration")
        .and_then(|v| v.get("AvailabilityZone"))
        .and_then(|v| v.as_str())
        .unwrap_or("us-east-1a")
        .to_owned();

    let id = format!("nd-{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..24].to_string());
    let now = chrono::Utc::now().to_rfc3339();

    let node = Node {
        id: id.clone(),
        network_id: network_id.clone(),
        member_id,
        instance_type,
        availability_zone,
        status: "AVAILABLE".into(),
        created_at: now,
    };

    state.nodes.insert(id.clone(), node);

    rest_json::created(json!({
        "NodeId": id
    }))
}

async fn list_nodes(
    State(state): State<Arc<ManagedBlockchainState>>,
    Path(network_id): Path<String>,
) -> Response {
    let nodes: Vec<Value> = state
        .nodes
        .iter()
        .filter(|e| e.value().network_id == network_id)
        .map(|e| {
            let n = e.value();
            json!({
                "Id": n.id,
                "NetworkId": n.network_id,
                "InstanceType": n.instance_type,
                "AvailabilityZone": n.availability_zone,
                "Status": n.status,
                "CreationDate": n.created_at
            })
        })
        .collect();

    rest_json::ok(json!({
        "Nodes": nodes
    }))
}

async fn get_node(
    State(state): State<Arc<ManagedBlockchainState>>,
    Path((network_id, node_id)): Path<(String, String)>,
) -> Response {
    match state.nodes.get(&node_id) {
        Some(n) if n.network_id == network_id => rest_json::ok(json!({
            "Node": {
                "Id": n.id,
                "NetworkId": n.network_id,
                "MemberId": n.member_id,
                "InstanceType": n.instance_type,
                "AvailabilityZone": n.availability_zone,
                "Status": n.status,
                "CreationDate": n.created_at
            }
        })),
        _ => rest_json::error_response(&LawsError::NotFound(format!(
            "Node not found: {node_id}"
        ))),
    }
}

async fn delete_node(
    State(state): State<Arc<ManagedBlockchainState>>,
    Path((network_id, node_id)): Path<(String, String)>,
) -> Response {
    match state.nodes.get(&node_id) {
        Some(n) if n.network_id == network_id => {
            drop(n);
            state.nodes.remove(&node_id);
            rest_json::no_content()
        }
        _ => rest_json::error_response(&LawsError::NotFound(format!(
            "Node not found: {node_id}"
        ))),
    }
}
