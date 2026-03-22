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
pub struct Collaboration {
    pub id: String,
    pub arn: String,
    pub name: String,
    pub description: String,
    pub creator_account_id: String,
    pub query_log_status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Membership {
    pub id: String,
    pub arn: String,
    pub collaboration_id: String,
    pub collaboration_arn: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CleanRoomsState {
    pub collaborations: DashMap<String, Collaboration>,
    pub memberships: DashMap<String, Membership>,
}

impl Default for CleanRoomsState {
    fn default() -> Self {
        Self {
            collaborations: DashMap::new(),
            memberships: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<CleanRoomsState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/collaborations",
            post(create_collaboration).get(list_collaborations),
        )
        .route(
            "/collaborations/{collaboration_id}",
            get(get_collaboration).delete(delete_collaboration),
        )
        .route(
            "/memberships",
            post(create_membership).get(list_memberships),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn collaboration_to_json(c: &Collaboration) -> Value {
    json!({
        "id": c.id,
        "arn": c.arn,
        "name": c.name,
        "description": c.description,
        "creatorAccountId": c.creator_account_id,
        "queryLogStatus": c.query_log_status,
        "createTime": c.created_at,
    })
}

fn membership_to_json(m: &Membership) -> Value {
    json!({
        "id": m.id,
        "arn": m.arn,
        "collaborationId": m.collaboration_id,
        "collaborationArn": m.collaboration_arn,
        "status": m.status,
        "createTime": m.created_at,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateCollaborationRequest {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "queryLogStatus", default)]
    query_log_status: Option<String>,
    members: Option<Vec<Value>>,
}

async fn create_collaboration(
    State(state): State<Arc<CleanRoomsState>>,
    Json(req): Json<CreateCollaborationRequest>,
) -> Response {
    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:cleanrooms:{REGION}:{ACCOUNT_ID}:collaboration/{id}");
    let now = Utc::now().to_rfc3339();

    let collaboration = Collaboration {
        id: id.clone(),
        arn,
        name: req.name,
        description: req.description.unwrap_or_default(),
        creator_account_id: ACCOUNT_ID.to_string(),
        query_log_status: req
            .query_log_status
            .unwrap_or_else(|| "DISABLED".to_string()),
        created_at: now,
    };

    let resp = collaboration_to_json(&collaboration);
    state.collaborations.insert(id, collaboration);

    rest_json::created(json!({ "collaboration": resp }))
}

async fn list_collaborations(State(state): State<Arc<CleanRoomsState>>) -> Response {
    let items: Vec<Value> = state
        .collaborations
        .iter()
        .map(|entry| collaboration_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "collaborationList": items }))
}

async fn get_collaboration(
    State(state): State<Arc<CleanRoomsState>>,
    Path(collaboration_id): Path<String>,
) -> Response {
    match state.collaborations.get(&collaboration_id) {
        Some(c) => rest_json::ok(json!({ "collaboration": collaboration_to_json(c.value()) })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Collaboration not found: {collaboration_id}"
        ))),
    }
}

async fn delete_collaboration(
    State(state): State<Arc<CleanRoomsState>>,
    Path(collaboration_id): Path<String>,
) -> Response {
    match state.collaborations.remove(&collaboration_id) {
        Some(_) => {
            state
                .memberships
                .retain(|_, m| m.collaboration_id != collaboration_id);
            rest_json::no_content()
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Collaboration not found: {collaboration_id}"
        ))),
    }
}

#[derive(Deserialize)]
struct CreateMembershipRequest {
    #[serde(rename = "collaborationIdentifier")]
    collaboration_identifier: String,
    #[serde(rename = "queryLogStatus", default)]
    query_log_status: Option<String>,
}

async fn create_membership(
    State(state): State<Arc<CleanRoomsState>>,
    Json(req): Json<CreateMembershipRequest>,
) -> Response {
    let collaboration = match state.collaborations.get(&req.collaboration_identifier) {
        Some(c) => c,
        None => {
            return rest_json::error_response(&LawsError::NotFound(format!(
                "Collaboration not found: {}",
                req.collaboration_identifier
            )));
        }
    };

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:cleanrooms:{REGION}:{ACCOUNT_ID}:membership/{id}");
    let now = Utc::now().to_rfc3339();

    let membership = Membership {
        id: id.clone(),
        arn,
        collaboration_id: collaboration.id.clone(),
        collaboration_arn: collaboration.arn.clone(),
        status: "ACTIVE".to_string(),
        created_at: now,
    };

    let resp = membership_to_json(&membership);
    state.memberships.insert(id, membership);

    rest_json::created(json!({ "membership": resp }))
}

async fn list_memberships(State(state): State<Arc<CleanRoomsState>>) -> Response {
    let items: Vec<Value> = state
        .memberships
        .iter()
        .map(|entry| membership_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "membershipList": items }))
}
