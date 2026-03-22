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
pub struct Channel {
    pub id: String,
    pub arn: String,
    pub description: String,
    pub hls_ingest_url: String,
}

#[derive(Debug, Clone)]
pub struct OriginEndpoint {
    pub id: String,
    pub arn: String,
    pub channel_id: String,
    pub description: String,
    pub url: String,
    pub startover_window_seconds: u32,
    pub time_delay_seconds: u32,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct MediaPackageState {
    pub channels: DashMap<String, Channel>,
    pub origin_endpoints: DashMap<String, OriginEndpoint>,
}

impl Default for MediaPackageState {
    fn default() -> Self {
        Self {
            channels: DashMap::new(),
            origin_endpoints: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<MediaPackageState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/channels",
            axum::routing::post(create_channel).get(list_channels),
        )
        .route(
            "/channels/{id}",
            axum::routing::get(describe_channel).delete(delete_channel),
        )
        .route(
            "/origin_endpoints",
            axum::routing::post(create_origin_endpoint).get(list_origin_endpoints),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_channel(
    State(state): State<Arc<MediaPackageState>>,
    Json(payload): Json<Value>,
) -> Response {
    let id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or(&uuid::Uuid::new_v4().to_string())
        .to_owned();

    let description = payload
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let arn = format!("arn:aws:mediapackage:{REGION}:{ACCOUNT_ID}:channels/{id}");
    let hls_ingest_url =
        format!("https://{id}.mediapackage.{REGION}.amazonaws.com/in/v2/{id}/channel");

    let channel = Channel {
        id: id.clone(),
        arn: arn.clone(),
        description: description.clone(),
        hls_ingest_url: hls_ingest_url.clone(),
    };

    state.channels.insert(id.clone(), channel);

    rest_json::created(json!({
        "id": id,
        "arn": arn,
        "description": description,
        "hlsIngest": {
            "ingestEndpoints": [{
                "url": hls_ingest_url
            }]
        }
    }))
}

async fn list_channels(State(state): State<Arc<MediaPackageState>>) -> Response {
    let channels: Vec<Value> = state
        .channels
        .iter()
        .map(|e| {
            let c = e.value();
            json!({
                "id": c.id,
                "arn": c.arn,
                "description": c.description,
                "hlsIngest": {
                    "ingestEndpoints": [{
                        "url": c.hls_ingest_url
                    }]
                }
            })
        })
        .collect();

    rest_json::ok(json!({
        "channels": channels
    }))
}

async fn describe_channel(
    State(state): State<Arc<MediaPackageState>>,
    Path(id): Path<String>,
) -> Response {
    match state.channels.get(&id) {
        Some(c) => rest_json::ok(json!({
            "id": c.id,
            "arn": c.arn,
            "description": c.description,
            "hlsIngest": {
                "ingestEndpoints": [{
                    "url": c.hls_ingest_url
                }]
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!("Channel not found: {id}"))),
    }
}

async fn delete_channel(
    State(state): State<Arc<MediaPackageState>>,
    Path(id): Path<String>,
) -> Response {
    match state.channels.remove(&id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!("Channel not found: {id}"))),
    }
}

async fn create_origin_endpoint(
    State(state): State<Arc<MediaPackageState>>,
    Json(payload): Json<Value>,
) -> Response {
    let id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or(&uuid::Uuid::new_v4().to_string())
        .to_owned();

    let channel_id = payload
        .get("channelId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let description = payload
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let startover_window_seconds = payload
        .get("startoverWindowSeconds")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let time_delay_seconds = payload
        .get("timeDelaySeconds")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let arn = format!("arn:aws:mediapackage:{REGION}:{ACCOUNT_ID}:origin_endpoints/{id}");
    let url = format!("https://{id}.mediapackage.{REGION}.amazonaws.com/out/v1/{id}.m3u8");

    let endpoint = OriginEndpoint {
        id: id.clone(),
        arn: arn.clone(),
        channel_id: channel_id.clone(),
        description: description.clone(),
        url: url.clone(),
        startover_window_seconds,
        time_delay_seconds,
    };

    state.origin_endpoints.insert(id.clone(), endpoint);

    rest_json::created(json!({
        "id": id,
        "arn": arn,
        "channelId": channel_id,
        "description": description,
        "url": url,
        "startoverWindowSeconds": startover_window_seconds,
        "timeDelaySeconds": time_delay_seconds
    }))
}

async fn list_origin_endpoints(State(state): State<Arc<MediaPackageState>>) -> Response {
    let endpoints: Vec<Value> = state
        .origin_endpoints
        .iter()
        .map(|e| {
            let ep = e.value();
            json!({
                "id": ep.id,
                "arn": ep.arn,
                "channelId": ep.channel_id,
                "description": ep.description,
                "url": ep.url,
                "startoverWindowSeconds": ep.startover_window_seconds,
                "timeDelaySeconds": ep.time_delay_seconds
            })
        })
        .collect();

    rest_json::ok(json!({
        "originEndpoints": endpoints
    }))
}
