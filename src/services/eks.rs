use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Json;
use chrono::Utc;
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
pub struct EksCluster {
    pub name: String,
    pub arn: String,
    pub role_arn: String,
    pub status: String,
    pub endpoint: String,
    pub created_at: String,
    pub kubernetes_version: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct EksState {
    pub clusters: DashMap<String, EksCluster>,
}

impl Default for EksState {
    fn default() -> Self {
        Self {
            clusters: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<EksState>) -> axum::Router {
    axum::Router::new()
        .route("/clusters", post(create_cluster).get(list_clusters))
        .route(
            "/clusters/{name}",
            get(describe_cluster).delete(delete_cluster),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_cluster(State(state): State<Arc<EksState>>, Json(body): Json<Value>) -> Response {
    match do_create_cluster(&state, &body) {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

fn do_create_cluster(state: &EksState, body: &Value) -> Result<Response, LawsError> {
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest("missing required field: name".into()))?
        .to_owned();

    let role_arn = body
        .get("roleArn")
        .and_then(|v| v.as_str())
        .unwrap_or(&format!("arn:aws:iam::{ACCOUNT_ID}:role/eks-role"))
        .to_owned();

    if state.clusters.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "cluster already exists: {name}"
        )));
    }

    let arn = format!("arn:aws:eks:{REGION}:{ACCOUNT_ID}:cluster/{name}");
    let endpoint_id = uuid::Uuid::new_v4().to_string().replace('-', "")[..32].to_string();
    let endpoint = format!("https://{endpoint_id}.gr7.{REGION}.eks.amazonaws.com");

    let cluster = EksCluster {
        name: name.clone(),
        arn,
        role_arn,
        status: "ACTIVE".into(),
        endpoint,
        created_at: Utc::now().to_rfc3339(),
        kubernetes_version: "1.29".into(),
    };

    let resp = cluster_to_json(&cluster);
    state.clusters.insert(name, cluster);

    Ok(rest_json::created(json!({ "cluster": resp })))
}

async fn list_clusters(State(state): State<Arc<EksState>>) -> Response {
    let names: Vec<String> = state
        .clusters
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    rest_json::ok(json!({ "clusters": names }))
}

async fn describe_cluster(
    State(state): State<Arc<EksState>>,
    Path(name): Path<String>,
) -> Response {
    match state.clusters.get(&name) {
        Some(cluster) => rest_json::ok(json!({ "cluster": cluster_to_json(&cluster) })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("cluster not found: {name}")))
        }
    }
}

async fn delete_cluster(State(state): State<Arc<EksState>>, Path(name): Path<String>) -> Response {
    match state.clusters.remove(&name) {
        Some((_, cluster)) => rest_json::ok(json!({ "cluster": cluster_to_json(&cluster) })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("cluster not found: {name}")))
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn cluster_to_json(c: &EksCluster) -> Value {
    json!({
        "name": c.name,
        "arn": c.arn,
        "roleArn": c.role_arn,
        "status": c.status,
        "endpoint": c.endpoint,
        "createdAt": c.created_at,
        "version": c.kubernetes_version,
        "resourcesVpcConfig": {
            "subnetIds": [],
            "securityGroupIds": [],
            "vpcId": "vpc-12345678"
        },
        "platformVersion": "eks.1"
    })
}
