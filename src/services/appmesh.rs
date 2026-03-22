use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::Deserialize;
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
pub struct Mesh {
    pub mesh_name: String,
    pub arn: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub spec: Value,
}

#[derive(Debug, Clone)]
pub struct VirtualNode {
    pub mesh_name: String,
    pub virtual_node_name: String,
    pub arn: String,
    pub status: String,
    pub created_at: String,
    pub spec: Value,
}

#[derive(Debug, Clone)]
pub struct VirtualService {
    pub mesh_name: String,
    pub virtual_service_name: String,
    pub arn: String,
    pub status: String,
    pub created_at: String,
    pub spec: Value,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AppMeshState {
    pub meshes: DashMap<String, Mesh>,
    pub virtual_nodes: DashMap<String, VirtualNode>,
}

impl Default for AppMeshState {
    fn default() -> Self {
        Self {
            meshes: DashMap::new(),
            virtual_nodes: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<AppMeshState>) -> axum::Router {
    axum::Router::new()
        .route("/v20190125/meshes", post(create_mesh).get(list_meshes))
        .route(
            "/v20190125/meshes/{mesh_name}",
            get(describe_mesh).delete(delete_mesh),
        )
        .route(
            "/v20190125/meshes/{mesh_name}/virtualNodes",
            post(create_virtual_node).get(list_virtual_nodes),
        )
        .route(
            "/v20190125/meshes/{mesh_name}/virtualServices",
            post(create_virtual_service),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateMeshRequest {
    #[serde(alias = "meshName")]
    mesh_name: String,
    #[serde(default)]
    spec: Option<Value>,
}

async fn create_mesh(
    State(state): State<Arc<AppMeshState>>,
    Json(req): Json<CreateMeshRequest>,
) -> Response {
    if state.meshes.contains_key(&req.mesh_name) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "Mesh already exists: {}",
            req.mesh_name
        )));
    }

    let arn = format!(
        "arn:aws:appmesh:{REGION}:{ACCOUNT_ID}:mesh/{name}",
        name = req.mesh_name
    );
    let now = Utc::now().to_rfc3339();

    let mesh = Mesh {
        mesh_name: req.mesh_name.clone(),
        arn: arn.clone(),
        status: "ACTIVE".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        spec: req.spec.unwrap_or(json!({})),
    };

    state.meshes.insert(req.mesh_name.clone(), mesh);

    rest_json::created(json!({
        "mesh": {
            "meshName": req.mesh_name,
            "metadata": {
                "arn": arn,
                "createdAt": now,
                "lastUpdatedAt": now,
                "meshOwner": ACCOUNT_ID,
                "resourceOwner": ACCOUNT_ID,
                "version": 1,
            },
            "status": { "status": "ACTIVE" },
        }
    }))
}

async fn list_meshes(State(state): State<Arc<AppMeshState>>) -> Response {
    let meshes: Vec<Value> = state
        .meshes
        .iter()
        .map(|entry| {
            let m = entry.value();
            json!({
                "meshName": m.mesh_name,
                "arn": m.arn,
                "meshOwner": ACCOUNT_ID,
                "resourceOwner": ACCOUNT_ID,
                "createdAt": m.created_at,
                "lastUpdatedAt": m.updated_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "meshes": meshes }))
}

async fn describe_mesh(
    State(state): State<Arc<AppMeshState>>,
    Path(mesh_name): Path<String>,
) -> Response {
    match state.meshes.get(&mesh_name) {
        Some(m) => rest_json::ok(json!({
            "mesh": {
                "meshName": m.mesh_name,
                "metadata": {
                    "arn": m.arn,
                    "createdAt": m.created_at,
                    "lastUpdatedAt": m.updated_at,
                    "meshOwner": ACCOUNT_ID,
                    "resourceOwner": ACCOUNT_ID,
                    "version": 1,
                },
                "spec": m.spec,
                "status": { "status": m.status },
            }
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Mesh not found: {mesh_name}")))
        }
    }
}

async fn delete_mesh(
    State(state): State<Arc<AppMeshState>>,
    Path(mesh_name): Path<String>,
) -> Response {
    match state.meshes.remove(&mesh_name) {
        Some((_, m)) => rest_json::ok(json!({
            "mesh": {
                "meshName": m.mesh_name,
                "metadata": {
                    "arn": m.arn,
                    "createdAt": m.created_at,
                    "lastUpdatedAt": m.updated_at,
                    "meshOwner": ACCOUNT_ID,
                    "resourceOwner": ACCOUNT_ID,
                    "version": 1,
                },
                "status": { "status": "DELETED" },
            }
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Mesh not found: {mesh_name}")))
        }
    }
}

#[derive(Deserialize)]
struct CreateVirtualNodeRequest {
    #[serde(alias = "virtualNodeName")]
    virtual_node_name: String,
    #[serde(default)]
    spec: Option<Value>,
}

async fn create_virtual_node(
    State(state): State<Arc<AppMeshState>>,
    Path(mesh_name): Path<String>,
    Json(req): Json<CreateVirtualNodeRequest>,
) -> Response {
    if !state.meshes.contains_key(&mesh_name) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Mesh not found: {mesh_name}"
        )));
    }

    let key = format!("{mesh_name}:{}", req.virtual_node_name);
    if state.virtual_nodes.contains_key(&key) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "VirtualNode already exists: {}",
            req.virtual_node_name
        )));
    }

    let arn = format!(
        "arn:aws:appmesh:{REGION}:{ACCOUNT_ID}:mesh/{mesh_name}/virtualNode/{vn}",
        vn = req.virtual_node_name
    );
    let now = Utc::now().to_rfc3339();
    let spec = req.spec.unwrap_or(json!({}));

    let vn = VirtualNode {
        mesh_name: mesh_name.clone(),
        virtual_node_name: req.virtual_node_name.clone(),
        arn: arn.clone(),
        status: "ACTIVE".to_string(),
        created_at: now.clone(),
        spec: spec.clone(),
    };

    state.virtual_nodes.insert(key, vn);

    rest_json::created(json!({
        "virtualNode": {
            "meshName": mesh_name,
            "virtualNodeName": req.virtual_node_name,
            "metadata": {
                "arn": arn,
                "createdAt": now,
                "lastUpdatedAt": now,
                "meshOwner": ACCOUNT_ID,
                "resourceOwner": ACCOUNT_ID,
                "version": 1,
            },
            "spec": spec,
            "status": { "status": "ACTIVE" },
        }
    }))
}

async fn list_virtual_nodes(
    State(state): State<Arc<AppMeshState>>,
    Path(mesh_name): Path<String>,
) -> Response {
    let nodes: Vec<Value> = state
        .virtual_nodes
        .iter()
        .filter(|entry| entry.value().mesh_name == mesh_name)
        .map(|entry| {
            let vn = entry.value();
            json!({
                "meshName": vn.mesh_name,
                "virtualNodeName": vn.virtual_node_name,
                "arn": vn.arn,
                "meshOwner": ACCOUNT_ID,
                "resourceOwner": ACCOUNT_ID,
                "createdAt": vn.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "virtualNodes": nodes }))
}

#[derive(Deserialize)]
struct CreateVirtualServiceRequest {
    #[serde(alias = "virtualServiceName")]
    virtual_service_name: String,
    #[serde(default)]
    spec: Option<Value>,
}

async fn create_virtual_service(
    State(state): State<Arc<AppMeshState>>,
    Path(mesh_name): Path<String>,
    Json(req): Json<CreateVirtualServiceRequest>,
) -> Response {
    if !state.meshes.contains_key(&mesh_name) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Mesh not found: {mesh_name}"
        )));
    }

    let arn = format!(
        "arn:aws:appmesh:{REGION}:{ACCOUNT_ID}:mesh/{mesh_name}/virtualService/{vs}",
        vs = req.virtual_service_name
    );
    let now = Utc::now().to_rfc3339();
    let spec = req.spec.unwrap_or(json!({}));

    rest_json::created(json!({
        "virtualService": {
            "meshName": mesh_name,
            "virtualServiceName": req.virtual_service_name,
            "metadata": {
                "arn": arn,
                "createdAt": now,
                "lastUpdatedAt": now,
                "meshOwner": ACCOUNT_ID,
                "resourceOwner": ACCOUNT_ID,
                "version": 1,
            },
            "spec": spec,
            "status": { "status": "ACTIVE" },
        }
    }))
}
