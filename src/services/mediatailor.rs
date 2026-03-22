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
pub struct PlaybackConfiguration {
    pub name: String,
    pub ad_decision_server_url: String,
    pub video_content_source_url: String,
    pub playback_configuration_arn: String,
    pub session_initialization_endpoint_prefix: String,
    pub hls_configuration_manifest_endpoint_prefix: String,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub channel_name: String,
    pub arn: String,
    pub channel_state: String,
    pub playback_mode: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct MediaTailorState {
    pub configurations: DashMap<String, PlaybackConfiguration>,
    pub channels: DashMap<String, Channel>,
}

impl Default for MediaTailorState {
    fn default() -> Self {
        Self {
            configurations: DashMap::new(),
            channels: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<MediaTailorState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/playbackConfiguration",
            axum::routing::put(put_playback_configuration).get(list_playback_configurations),
        )
        .route(
            "/playbackConfiguration/{name}",
            axum::routing::get(get_playback_configuration).delete(delete_playback_configuration),
        )
        .route(
            "/channels",
            axum::routing::post(create_channel).get(list_channels),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn put_playback_configuration(
    State(state): State<Arc<MediaTailorState>>,
    Json(payload): Json<Value>,
) -> Response {
    let name = payload
        .get("Name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let ad_decision_server_url = payload
        .get("AdDecisionServerUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let video_content_source_url = payload
        .get("VideoContentSourceUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let arn = format!("arn:aws:mediatailor:{REGION}:{ACCOUNT_ID}:playbackConfiguration/{name}");
    let session_prefix = format!(
        "https://{}.mediatailor.{REGION}.amazonaws.com/v1/session/{ACCOUNT_ID}/{name}",
        uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string()
    );
    let hls_prefix = format!("{session_prefix}/");

    let config = PlaybackConfiguration {
        name: name.clone(),
        ad_decision_server_url: ad_decision_server_url.clone(),
        video_content_source_url: video_content_source_url.clone(),
        playback_configuration_arn: arn.clone(),
        session_initialization_endpoint_prefix: session_prefix.clone(),
        hls_configuration_manifest_endpoint_prefix: hls_prefix.clone(),
    };

    state.configurations.insert(name.clone(), config);

    rest_json::ok(json!({
        "Name": name,
        "AdDecisionServerUrl": ad_decision_server_url,
        "VideoContentSourceUrl": video_content_source_url,
        "PlaybackConfigurationArn": arn,
        "SessionInitializationEndpointPrefix": session_prefix,
        "HlsConfiguration": {
            "ManifestEndpointPrefix": hls_prefix
        }
    }))
}

async fn get_playback_configuration(
    State(state): State<Arc<MediaTailorState>>,
    Path(name): Path<String>,
) -> Response {
    match state.configurations.get(&name) {
        Some(c) => rest_json::ok(json!({
            "Name": c.name,
            "AdDecisionServerUrl": c.ad_decision_server_url,
            "VideoContentSourceUrl": c.video_content_source_url,
            "PlaybackConfigurationArn": c.playback_configuration_arn,
            "SessionInitializationEndpointPrefix": c.session_initialization_endpoint_prefix,
            "HlsConfiguration": {
                "ManifestEndpointPrefix": c.hls_configuration_manifest_endpoint_prefix
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Playback configuration not found: {name}"
        ))),
    }
}

async fn list_playback_configurations(State(state): State<Arc<MediaTailorState>>) -> Response {
    let configs: Vec<Value> = state
        .configurations
        .iter()
        .map(|e| {
            let c = e.value();
            json!({
                "Name": c.name,
                "AdDecisionServerUrl": c.ad_decision_server_url,
                "VideoContentSourceUrl": c.video_content_source_url,
                "PlaybackConfigurationArn": c.playback_configuration_arn
            })
        })
        .collect();

    rest_json::ok(json!({
        "Items": configs
    }))
}

async fn delete_playback_configuration(
    State(state): State<Arc<MediaTailorState>>,
    Path(name): Path<String>,
) -> Response {
    match state.configurations.remove(&name) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Playback configuration not found: {name}"
        ))),
    }
}

async fn create_channel(
    State(state): State<Arc<MediaTailorState>>,
    Json(payload): Json<Value>,
) -> Response {
    let channel_name = payload
        .get("ChannelName")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();

    let playback_mode = payload
        .get("PlaybackMode")
        .and_then(|v| v.as_str())
        .unwrap_or("LOOP")
        .to_owned();

    let arn = format!("arn:aws:mediatailor:{REGION}:{ACCOUNT_ID}:channel/{channel_name}");
    let now = chrono::Utc::now().to_rfc3339();

    let channel = Channel {
        channel_name: channel_name.clone(),
        arn: arn.clone(),
        channel_state: "STOPPED".into(),
        playback_mode: playback_mode.clone(),
        created_at: now.clone(),
    };

    state.channels.insert(channel_name.clone(), channel);

    rest_json::created(json!({
        "ChannelName": channel_name,
        "Arn": arn,
        "ChannelState": "STOPPED",
        "PlaybackMode": playback_mode,
        "CreationTime": now
    }))
}

async fn list_channels(State(state): State<Arc<MediaTailorState>>) -> Response {
    let channels: Vec<Value> = state
        .channels
        .iter()
        .map(|e| {
            let ch = e.value();
            json!({
                "ChannelName": ch.channel_name,
                "Arn": ch.arn,
                "ChannelState": ch.channel_state,
                "PlaybackMode": ch.playback_mode,
                "CreationTime": ch.created_at
            })
        })
        .collect();

    rest_json::ok(json!({
        "Items": channels
    }))
}
