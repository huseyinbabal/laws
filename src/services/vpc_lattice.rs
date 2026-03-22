use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, post};
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
pub struct LatticeService {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub status: String,
    pub created_at: String,
    pub dns_entry: String,
}

#[derive(Debug, Clone)]
pub struct TargetGroup {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub target_group_type: String,
    pub status: String,
    pub created_at: String,
    pub targets: Vec<Target>,
}

#[derive(Debug, Clone)]
pub struct Target {
    pub id: String,
    pub port: u16,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct VpcLatticeState {
    pub services: DashMap<String, LatticeService>,
    pub target_groups: DashMap<String, TargetGroup>,
}

impl Default for VpcLatticeState {
    fn default() -> Self {
        Self {
            services: DashMap::new(),
            target_groups: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<VpcLatticeState>) -> axum::Router {
    axum::Router::new()
        .route("/services", post(create_service).get(list_services))
        .route(
            "/services/{id}",
            get(get_service).delete(delete_service),
        )
        .route(
            "/targetgroups",
            post(create_target_group).get(list_target_groups),
        )
        .route(
            "/targetgroups/{id}/registertargets",
            post(register_targets),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateServiceRequest {
    name: String,
}

async fn create_service(
    State(state): State<Arc<VpcLatticeState>>,
    Json(req): Json<CreateServiceRequest>,
) -> Response {
    let id = format!("svc-{}", uuid::Uuid::new_v4().simple());
    let arn = format!("arn:aws:vpc-lattice:{REGION}:{ACCOUNT_ID}:service/{id}");
    let now = Utc::now().to_rfc3339();
    let dns_entry = format!("{id}.{REGION}.vpc-lattice-svcs.amazonaws.com");

    let service = LatticeService {
        id: id.clone(),
        name: req.name.clone(),
        arn: arn.clone(),
        status: "ACTIVE".to_string(),
        created_at: now.clone(),
        dns_entry: dns_entry.clone(),
    };

    state.services.insert(id.clone(), service);

    rest_json::created(json!({
        "id": id,
        "name": req.name,
        "arn": arn,
        "status": "ACTIVE",
        "createdAt": now,
        "dnsEntry": { "domainName": dns_entry },
    }))
}

async fn list_services(State(state): State<Arc<VpcLatticeState>>) -> Response {
    let items: Vec<Value> = state
        .services
        .iter()
        .map(|entry| {
            let s = entry.value();
            json!({
                "id": s.id,
                "name": s.name,
                "arn": s.arn,
                "status": s.status,
                "createdAt": s.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "items": items }))
}

async fn get_service(
    State(state): State<Arc<VpcLatticeState>>,
    Path(id): Path<String>,
) -> Response {
    match state.services.get(&id) {
        Some(s) => rest_json::ok(json!({
            "id": s.id,
            "name": s.name,
            "arn": s.arn,
            "status": s.status,
            "createdAt": s.created_at,
            "dnsEntry": { "domainName": s.dns_entry },
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Service not found: {id}"
        ))),
    }
}

async fn delete_service(
    State(state): State<Arc<VpcLatticeState>>,
    Path(id): Path<String>,
) -> Response {
    match state.services.remove(&id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Service not found: {id}"
        ))),
    }
}

#[derive(Deserialize)]
struct CreateTargetGroupRequest {
    name: String,
    #[serde(default, rename = "type")]
    target_group_type: Option<String>,
}

async fn create_target_group(
    State(state): State<Arc<VpcLatticeState>>,
    Json(req): Json<CreateTargetGroupRequest>,
) -> Response {
    let id = format!("tg-{}", uuid::Uuid::new_v4().simple());
    let arn = format!("arn:aws:vpc-lattice:{REGION}:{ACCOUNT_ID}:targetgroup/{id}");
    let now = Utc::now().to_rfc3339();
    let tg_type = req.target_group_type.unwrap_or_else(|| "INSTANCE".into());

    let tg = TargetGroup {
        id: id.clone(),
        name: req.name.clone(),
        arn: arn.clone(),
        target_group_type: tg_type.clone(),
        status: "ACTIVE".to_string(),
        created_at: now.clone(),
        targets: Vec::new(),
    };

    state.target_groups.insert(id.clone(), tg);

    rest_json::created(json!({
        "id": id,
        "name": req.name,
        "arn": arn,
        "type": tg_type,
        "status": "ACTIVE",
        "createdAt": now,
    }))
}

async fn list_target_groups(State(state): State<Arc<VpcLatticeState>>) -> Response {
    let items: Vec<Value> = state
        .target_groups
        .iter()
        .map(|entry| {
            let tg = entry.value();
            json!({
                "id": tg.id,
                "name": tg.name,
                "arn": tg.arn,
                "type": tg.target_group_type,
                "status": tg.status,
                "createdAt": tg.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "items": items }))
}

#[derive(Deserialize)]
struct RegisterTargetsRequest {
    targets: Vec<TargetInput>,
}

#[derive(Deserialize)]
struct TargetInput {
    id: String,
    #[serde(default)]
    port: Option<u16>,
}

async fn register_targets(
    State(state): State<Arc<VpcLatticeState>>,
    Path(id): Path<String>,
    Json(req): Json<RegisterTargetsRequest>,
) -> Response {
    match state.target_groups.get_mut(&id) {
        Some(mut tg) => {
            let mut successful = Vec::new();
            for t in req.targets {
                let port = t.port.unwrap_or(80);
                tg.targets.push(Target {
                    id: t.id.clone(),
                    port,
                });
                successful.push(json!({ "id": t.id, "port": port }));
            }
            rest_json::ok(json!({
                "successful": successful,
                "unsuccessful": [],
            }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "TargetGroup not found: {id}"
        ))),
    }
}
