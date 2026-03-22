use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataZoneDomain {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arn: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataZoneProject {
    pub id: String,
    pub domain_id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DataZoneState {
    pub domains: DashMap<String, DataZoneDomain>,
    pub projects: DashMap<String, DataZoneProject>,
}

impl Default for DataZoneState {
    fn default() -> Self {
        Self {
            domains: DashMap::new(),
            projects: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<DataZoneState>) -> axum::Router {
    axum::Router::new()
        .route("/v2/domains", post(create_domain).get(list_domains))
        .route(
            "/v2/domains/{domain_id}",
            get(get_domain).delete(delete_domain),
        )
        .route(
            "/v2/domains/{domain_id}/projects",
            post(create_project).get(list_projects),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_domain(
    State(state): State<Arc<DataZoneState>>,
    Json(body): Json<Value>,
) -> Response {
    let name = match body["name"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest("Missing name".into()))
        }
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let arn = format!("arn:aws:datazone:{REGION}:{ACCOUNT_ID}:domain/{id}");
    let description = body["description"].as_str().unwrap_or("").to_string();

    let domain = DataZoneDomain {
        id: id.clone(),
        name,
        description,
        arn: arn.clone(),
        status: "AVAILABLE".to_string(),
        created_at: now,
    };

    let resp = json!({
        "id": domain.id,
        "name": domain.name,
        "description": domain.description,
        "arn": domain.arn,
        "status": domain.status,
        "createdAt": domain.created_at,
    });

    state.domains.insert(id, domain);
    rest_json::created(resp)
}

async fn list_domains(State(state): State<Arc<DataZoneState>>) -> Response {
    let items: Vec<Value> = state
        .domains
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "id": d.id,
                "name": d.name,
                "arn": d.arn,
                "status": d.status,
            })
        })
        .collect();

    rest_json::ok(json!({ "items": items }))
}

async fn get_domain(
    State(state): State<Arc<DataZoneState>>,
    Path(domain_id): Path<String>,
) -> Response {
    match state.domains.get(&domain_id) {
        Some(d) => rest_json::ok(json!({
            "id": d.id,
            "name": d.name,
            "description": d.description,
            "arn": d.arn,
            "status": d.status,
            "createdAt": d.created_at,
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Domain not found: {domain_id}"
        ))),
    }
}

async fn delete_domain(
    State(state): State<Arc<DataZoneState>>,
    Path(domain_id): Path<String>,
) -> Response {
    match state.domains.remove(&domain_id) {
        Some(_) => {
            state.projects.retain(|_, p| p.domain_id != domain_id);
            rest_json::ok(json!({ "status": "DELETED" }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Domain not found: {domain_id}"
        ))),
    }
}

async fn create_project(
    State(state): State<Arc<DataZoneState>>,
    Path(domain_id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    if !state.domains.contains_key(&domain_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Domain not found: {domain_id}"
        )));
    }

    let name = match body["name"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest("Missing name".into()))
        }
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let description = body["description"].as_str().unwrap_or("").to_string();

    let project = DataZoneProject {
        id: id.clone(),
        domain_id: domain_id.clone(),
        name,
        description,
        created_at: now,
    };

    let resp = json!({
        "id": project.id,
        "domainId": project.domain_id,
        "name": project.name,
        "description": project.description,
        "createdAt": project.created_at,
    });

    state.projects.insert(id, project);
    rest_json::created(resp)
}

async fn list_projects(
    State(state): State<Arc<DataZoneState>>,
    Path(domain_id): Path<String>,
) -> Response {
    if !state.domains.contains_key(&domain_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Domain not found: {domain_id}"
        )));
    }

    let items: Vec<Value> = state
        .projects
        .iter()
        .filter(|entry| entry.value().domain_id == domain_id)
        .map(|entry| {
            let p = entry.value();
            json!({
                "id": p.id,
                "domainId": p.domain_id,
                "name": p.name,
                "description": p.description,
                "createdAt": p.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "items": items }))
}
