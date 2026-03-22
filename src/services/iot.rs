use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, post};
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
pub struct IotThing {
    pub thing_name: String,
    pub thing_arn: String,
    pub attributes: Value,
}

#[derive(Debug, Clone)]
pub struct IotPolicy {
    pub policy_name: String,
    pub policy_arn: String,
    pub policy_document: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct IotState {
    pub things: DashMap<String, IotThing>,
    pub policies: DashMap<String, IotPolicy>,
}

impl Default for IotState {
    fn default() -> Self {
        Self {
            things: DashMap::new(),
            policies: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<IotState>) -> axum::Router {
    axum::Router::new()
        .route("/things", get(list_things))
        .route(
            "/things/{name}",
            post(create_thing).get(describe_thing).delete(delete_thing),
        )
        .route("/policies", get(list_policies))
        .route(
            "/policies/{name}",
            post(create_policy).delete(delete_policy),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_thing(
    State(state): State<Arc<IotState>>,
    Path(name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let thing_arn = format!("arn:aws:iot:{REGION}:{ACCOUNT_ID}:thing/{name}");

    let attributes = payload.get("attributes").cloned().unwrap_or(json!({}));

    let thing = IotThing {
        thing_name: name.clone(),
        thing_arn: thing_arn.clone(),
        attributes: attributes.clone(),
    };

    state.things.insert(name.clone(), thing);

    rest_json::ok(json!({
        "thingName": name,
        "thingArn": thing_arn,
        "attributes": attributes,
    }))
}

async fn list_things(State(state): State<Arc<IotState>>) -> Response {
    let things: Vec<Value> = state
        .things
        .iter()
        .map(|entry| {
            let t = entry.value();
            json!({
                "thingName": t.thing_name,
                "thingArn": t.thing_arn,
                "attributes": t.attributes,
            })
        })
        .collect();

    rest_json::ok(json!({ "things": things }))
}

async fn describe_thing(State(state): State<Arc<IotState>>, Path(name): Path<String>) -> Response {
    match state.things.get(&name) {
        Some(thing) => rest_json::ok(json!({
            "thingName": thing.thing_name,
            "thingArn": thing.thing_arn,
            "attributes": thing.attributes,
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Thing '{}' not found", name)))
        }
    }
}

async fn delete_thing(State(state): State<Arc<IotState>>, Path(name): Path<String>) -> Response {
    match state.things.remove(&name) {
        Some(_) => rest_json::ok(json!({})),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Thing '{}' not found", name)))
        }
    }
}

async fn create_policy(
    State(state): State<Arc<IotState>>,
    Path(name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let policy_arn = format!("arn:aws:iot:{REGION}:{ACCOUNT_ID}:policy/{name}");

    let policy_document = payload["policyDocument"]
        .as_str()
        .unwrap_or("{}")
        .to_string();

    let policy = IotPolicy {
        policy_name: name.clone(),
        policy_arn: policy_arn.clone(),
        policy_document: policy_document.clone(),
    };

    state.policies.insert(name.clone(), policy);

    rest_json::ok(json!({
        "policyName": name,
        "policyArn": policy_arn,
        "policyDocument": policy_document,
    }))
}

async fn list_policies(State(state): State<Arc<IotState>>) -> Response {
    let policies: Vec<Value> = state
        .policies
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "policyName": p.policy_name,
                "policyArn": p.policy_arn,
            })
        })
        .collect();

    rest_json::ok(json!({ "policies": policies }))
}

async fn delete_policy(State(state): State<Arc<IotState>>, Path(name): Path<String>) -> Response {
    match state.policies.remove(&name) {
        Some(_) => rest_json::ok(json!({})),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Policy '{}' not found", name)))
        }
    }
}
