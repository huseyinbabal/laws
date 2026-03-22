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
pub struct SequenceStore {
    pub id: String,
    pub arn: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: String,
    pub arn: String,
    pub name: String,
    pub workflow_type: String,
    pub status: String,
    pub description: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct OmicsState {
    pub sequence_stores: DashMap<String, SequenceStore>,
    pub workflows: DashMap<String, Workflow>,
}

impl Default for OmicsState {
    fn default() -> Self {
        Self {
            sequence_stores: DashMap::new(),
            workflows: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<OmicsState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/sequenceStore",
            axum::routing::post(create_sequence_store),
        )
        .route(
            "/sequenceStore/list",
            axum::routing::post(list_sequence_stores),
        )
        .route(
            "/sequenceStore/{id}",
            axum::routing::get(get_sequence_store)
                .delete(delete_sequence_store),
        )
        .route(
            "/workflow",
            axum::routing::post(create_workflow).get(list_workflows),
        )
        .route(
            "/workflow/{id}",
            axum::routing::get(get_workflow),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_id() -> String {
    uuid::Uuid::new_v4().to_string()[..10].to_string()
}

fn sequence_store_to_json(s: &SequenceStore) -> Value {
    json!({
        "id": s.id,
        "arn": s.arn,
        "name": s.name,
        "description": s.description,
        "creationTime": s.created_at,
    })
}

fn workflow_to_json(w: &Workflow) -> Value {
    json!({
        "id": w.id,
        "arn": w.arn,
        "name": w.name,
        "type": w.workflow_type,
        "status": w.status,
        "description": w.description,
        "creationTime": w.created_at,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_sequence_store(
    State(state): State<Arc<OmicsState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing name".into()))?
            .to_string();

        let description = payload["description"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let id = random_id();
        let arn = format!(
            "arn:aws:omics:{REGION}:{ACCOUNT_ID}:sequenceStore/{id}"
        );
        let created_at = chrono::Utc::now().to_rfc3339();

        let store = SequenceStore {
            id: id.clone(),
            arn,
            name,
            description,
            created_at,
        };

        let resp = sequence_store_to_json(&store);
        state.sequence_stores.insert(id, store);

        Ok(rest_json::created(resp))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_sequence_stores(
    State(state): State<Arc<OmicsState>>,
) -> Response {
    let stores: Vec<Value> = state
        .sequence_stores
        .iter()
        .map(|entry| sequence_store_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "sequenceStores": stores }))
}

async fn get_sequence_store(
    State(state): State<Arc<OmicsState>>,
    Path(id): Path<String>,
) -> Response {
    match state.sequence_stores.get(&id) {
        Some(store) => rest_json::ok(sequence_store_to_json(store.value())),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "SequenceStore '{}' not found",
            id
        ))),
    }
}

async fn delete_sequence_store(
    State(state): State<Arc<OmicsState>>,
    Path(id): Path<String>,
) -> Response {
    match state.sequence_stores.remove(&id) {
        Some(_) => rest_json::ok(json!({})),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "SequenceStore '{}' not found",
            id
        ))),
    }
}

async fn create_workflow(
    State(state): State<Arc<OmicsState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing name".into()))?
            .to_string();

        let workflow_type = payload["type"]
            .as_str()
            .unwrap_or("PRIVATE")
            .to_string();

        let description = payload["description"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let id = random_id();
        let arn = format!(
            "arn:aws:omics:{REGION}:{ACCOUNT_ID}:workflow/{id}"
        );
        let created_at = chrono::Utc::now().to_rfc3339();

        let workflow = Workflow {
            id: id.clone(),
            arn,
            name,
            workflow_type,
            status: "ACTIVE".to_string(),
            description,
            created_at,
        };

        let resp = workflow_to_json(&workflow);
        state.workflows.insert(id, workflow);

        Ok(rest_json::created(resp))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_workflows(
    State(state): State<Arc<OmicsState>>,
) -> Response {
    let workflows: Vec<Value> = state
        .workflows
        .iter()
        .map(|entry| workflow_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "items": workflows }))
}

async fn get_workflow(
    State(state): State<Arc<OmicsState>>,
    Path(id): Path<String>,
) -> Response {
    match state.workflows.get(&id) {
        Some(wf) => rest_json::ok(workflow_to_json(wf.value())),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Workflow '{}' not found",
            id
        ))),
    }
}
