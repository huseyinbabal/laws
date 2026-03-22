use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use dashmap::DashMap;
use http::StatusCode;
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
pub struct Channel {
    pub id: String,
    pub arn: String,
    pub name: String,
    pub state: String,
    pub input_attachments: Vec<String>,
    pub role_arn: String,
}

#[derive(Debug, Clone)]
pub struct Input {
    pub id: String,
    pub arn: String,
    pub name: String,
    pub input_type: String,
    pub state: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct MediaLiveState {
    pub channels: DashMap<String, Channel>,
    pub inputs: DashMap<String, Input>,
}

impl Default for MediaLiveState {
    fn default() -> Self {
        Self {
            channels: DashMap::new(),
            inputs: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<MediaLiveState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/prod/channels",
            axum::routing::post(create_channel).get(list_channels),
        )
        .route(
            "/prod/channels/{id}",
            axum::routing::get(describe_channel).delete(delete_channel),
        )
        .route(
            "/prod/inputs",
            axum::routing::post(create_input).get(list_inputs),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_id() -> String {
    use rand::RngExt;
    rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(7)
        .map(char::from)
        .collect::<String>()
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_channel(
    State(state): State<Arc<MediaLiveState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("Name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();
    let role_arn = payload
        .get("RoleArn")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let input_attachments: Vec<String> = payload
        .get("InputAttachments")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.get("InputId")
                        .and_then(|id| id.as_str())
                        .map(|s| s.to_owned())
                })
                .collect()
        })
        .unwrap_or_default();

    let id = random_id();
    let arn = format!("arn:aws:medialive:{REGION}:{ACCOUNT_ID}:channel:{id}");

    let channel = Channel {
        id: id.clone(),
        arn: arn.clone(),
        name: name.clone(),
        state: "IDLE".into(),
        input_attachments,
        role_arn,
    };

    state.channels.insert(id.clone(), channel);

    rest_json::created(json!({
        "Channel": {
            "Id": id,
            "Arn": arn,
            "Name": name,
            "State": "CREATING"
        }
    }))
}

async fn list_channels(State(state): State<Arc<MediaLiveState>>) -> Response {
    let channels: Vec<Value> = state
        .channels
        .iter()
        .map(|entry| {
            let c = entry.value();
            json!({
                "Id": c.id,
                "Arn": c.arn,
                "Name": c.name,
                "State": c.state
            })
        })
        .collect();

    rest_json::ok(json!({
        "Channels": channels
    }))
}

async fn describe_channel(
    State(state): State<Arc<MediaLiveState>>,
    Path(id): Path<String>,
) -> Response {
    match state.channels.get(&id) {
        Some(c) => rest_json::ok(json!({
            "Id": c.id,
            "Arn": c.arn,
            "Name": c.name,
            "State": c.state,
            "InputAttachments": c.input_attachments,
            "RoleArn": c.role_arn
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!("channel not found: {id}"))),
    }
}

async fn delete_channel(
    State(state): State<Arc<MediaLiveState>>,
    Path(id): Path<String>,
) -> Response {
    match state.channels.remove(&id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!("channel not found: {id}"))),
    }
}

async fn create_input(
    State(state): State<Arc<MediaLiveState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("Name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();
    let input_type = payload
        .get("Type")
        .and_then(|v| v.as_str())
        .unwrap_or("URL_PULL")
        .to_owned();

    let id = random_id();
    let arn = format!("arn:aws:medialive:{REGION}:{ACCOUNT_ID}:input:{id}");

    let input = Input {
        id: id.clone(),
        arn: arn.clone(),
        name: name.clone(),
        input_type: input_type.clone(),
        state: "DETACHED".into(),
    };

    state.inputs.insert(id.clone(), input);

    rest_json::created(json!({
        "Input": {
            "Id": id,
            "Arn": arn,
            "Name": name,
            "Type": input_type,
            "State": "CREATING"
        }
    }))
}

async fn list_inputs(State(state): State<Arc<MediaLiveState>>) -> Response {
    let inputs: Vec<Value> = state
        .inputs
        .iter()
        .map(|entry| {
            let i = entry.value();
            json!({
                "Id": i.id,
                "Arn": i.arn,
                "Name": i.name,
                "Type": i.input_type,
                "State": i.state
            })
        })
        .collect();

    rest_json::ok(json!({
        "Inputs": inputs
    }))
}
