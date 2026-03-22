use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
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
pub struct LifecyclePolicy {
    pub policy_id: String,
    pub description: String,
    pub state: String,
    pub policy_details: Value,
    pub date_created: String,
    pub date_modified: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DlmState {
    pub policies: DashMap<String, LifecyclePolicy>,
}

impl Default for DlmState {
    fn default() -> Self {
        Self {
            policies: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<DlmState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/policies",
            post(create_lifecycle_policy).get(get_lifecycle_policies),
        )
        .route(
            "/policies/{policy_id}",
            get(get_lifecycle_policy)
                .delete(delete_lifecycle_policy)
                .put(update_lifecycle_policy),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_lifecycle_policy(
    State(state): State<Arc<DlmState>>,
    Json(body): Json<Value>,
) -> Response {
    let description = body["Description"].as_str().unwrap_or("").to_string();
    let policy_state = body["State"].as_str().unwrap_or("ENABLED").to_string();
    let policy_details = body["PolicyDetails"].clone();

    let policy_id = format!("policy-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let now = Utc::now().to_rfc3339();

    let policy = LifecyclePolicy {
        policy_id: policy_id.clone(),
        description,
        state: policy_state,
        policy_details,
        date_created: now.clone(),
        date_modified: now,
    };

    state.policies.insert(policy_id.clone(), policy);
    rest_json::created(json!({ "PolicyId": policy_id }))
}

async fn get_lifecycle_policy(
    State(state): State<Arc<DlmState>>,
    Path(policy_id): Path<String>,
) -> Response {
    match state.policies.get(&policy_id) {
        Some(p) => rest_json::ok(json!({
            "Policy": {
                "PolicyId": p.policy_id,
                "Description": p.description,
                "State": p.state,
                "PolicyDetails": p.policy_details,
                "DateCreated": p.date_created,
                "DateModified": p.date_modified,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Policy not found: {policy_id}"
        ))),
    }
}

async fn get_lifecycle_policies(State(state): State<Arc<DlmState>>) -> Response {
    let items: Vec<Value> = state
        .policies
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "PolicyId": p.policy_id,
                "Description": p.description,
                "State": p.state,
            })
        })
        .collect();

    rest_json::ok(json!({ "Policies": items }))
}

async fn delete_lifecycle_policy(
    State(state): State<Arc<DlmState>>,
    Path(policy_id): Path<String>,
) -> Response {
    match state.policies.remove(&policy_id) {
        Some(_) => rest_json::ok(json!({})),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Policy not found: {policy_id}"
        ))),
    }
}

async fn update_lifecycle_policy(
    State(state): State<Arc<DlmState>>,
    Path(policy_id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    match state.policies.get_mut(&policy_id) {
        Some(mut p) => {
            if let Some(desc) = body["Description"].as_str() {
                p.description = desc.to_string();
            }
            if let Some(s) = body["State"].as_str() {
                p.state = s.to_string();
            }
            if body.get("PolicyDetails").is_some() {
                p.policy_details = body["PolicyDetails"].clone();
            }
            p.date_modified = Utc::now().to_rfc3339();
            rest_json::ok(json!({}))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Policy not found: {policy_id}"
        ))),
    }
}
