use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

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
    pub arn: String,
    pub name: String,
    pub latency_mode: String,
    pub channel_type: String,
    pub recording_configuration_arn: String,
    pub ingest_endpoint: String,
    pub playback_url: String,
    pub authorized: bool,
}

#[derive(Debug, Clone)]
pub struct StreamKey {
    pub arn: String,
    pub channel_arn: String,
    pub value: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct IvsState {
    pub channels: DashMap<String, Channel>,
    pub stream_keys: DashMap<String, StreamKey>,
}

impl Default for IvsState {
    fn default() -> Self {
        Self {
            channels: DashMap::new(),
            stream_keys: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &IvsState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AmazonInteractiveVideoService.")
        .unwrap_or(target);

    let result = match action {
        "CreateChannel" => create_channel(state, payload),
        "DeleteChannel" => delete_channel(state, payload),
        "GetChannel" => get_channel(state, payload),
        "ListChannels" => list_channels(state),
        "CreateStreamKey" => create_stream_key(state, payload),
        "ListStreamKeys" => list_stream_keys(state, payload),
        other => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            other
        ))),
    };

    match result {
        Ok(resp) => resp,
        Err(e) => e.into_response(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn json_response(body: Value) -> Response {
    (
        StatusCode::OK,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_channel(state: &IvsState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["name"].as_str().unwrap_or("unnamed").to_string();

    let latency_mode = payload["latencyMode"].as_str().unwrap_or("LOW").to_string();

    let channel_type = payload["type"].as_str().unwrap_or("STANDARD").to_string();

    let authorized = payload["authorized"].as_bool().unwrap_or(false);

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:ivs:{REGION}:{ACCOUNT_ID}:channel/{id}");
    let ingest_endpoint = format!("{id}.global-contribute.live-video.net");
    let playback_url = format!(
        "https://{id}.{REGION}.playback.live-video.net/api/video/v1/{ACCOUNT_ID}.channel.{id}.m3u8"
    );

    let channel = Channel {
        arn: arn.clone(),
        name: name.clone(),
        latency_mode: latency_mode.clone(),
        channel_type: channel_type.clone(),
        recording_configuration_arn: String::new(),
        ingest_endpoint: ingest_endpoint.clone(),
        playback_url: playback_url.clone(),
        authorized,
    };

    // Also create a default stream key
    let sk_id = uuid::Uuid::new_v4().to_string();
    let sk_arn = format!("arn:aws:ivs:{REGION}:{ACCOUNT_ID}:stream-key/{sk_id}");
    let sk_value = format!(
        "sk_{}_{}",
        REGION,
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );

    let stream_key = StreamKey {
        arn: sk_arn.clone(),
        channel_arn: arn.clone(),
        value: sk_value.clone(),
    };

    state.channels.insert(arn.clone(), channel);
    state.stream_keys.insert(sk_arn.clone(), stream_key);

    Ok(json_response(json!({
        "channel": {
            "arn": arn,
            "name": name,
            "latencyMode": latency_mode,
            "type": channel_type,
            "ingestEndpoint": ingest_endpoint,
            "playbackUrl": playback_url,
            "authorized": authorized
        },
        "streamKey": {
            "arn": sk_arn,
            "channelArn": arn,
            "value": sk_value
        }
    })))
}

fn delete_channel(state: &IvsState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload["arn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing arn".into()))?;

    state
        .channels
        .remove(arn)
        .ok_or_else(|| LawsError::NotFound(format!("Channel '{}' not found", arn)))?;

    // Remove associated stream keys
    let keys_to_remove: Vec<String> = state
        .stream_keys
        .iter()
        .filter(|e| e.value().channel_arn == arn)
        .map(|e| e.key().clone())
        .collect();
    for key in keys_to_remove {
        state.stream_keys.remove(&key);
    }

    Ok(json_response(json!({})))
}

fn get_channel(state: &IvsState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload["arn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing arn".into()))?;

    let ch = state
        .channels
        .get(arn)
        .ok_or_else(|| LawsError::NotFound(format!("Channel '{}' not found", arn)))?;

    Ok(json_response(json!({
        "channel": {
            "arn": ch.arn,
            "name": ch.name,
            "latencyMode": ch.latency_mode,
            "type": ch.channel_type,
            "ingestEndpoint": ch.ingest_endpoint,
            "playbackUrl": ch.playback_url,
            "authorized": ch.authorized
        }
    })))
}

fn list_channels(state: &IvsState) -> Result<Response, LawsError> {
    let channels: Vec<Value> = state
        .channels
        .iter()
        .map(|e| {
            let ch = e.value();
            json!({
                "arn": ch.arn,
                "name": ch.name,
                "latencyMode": ch.latency_mode,
                "authorized": ch.authorized
            })
        })
        .collect();

    Ok(json_response(json!({
        "channels": channels
    })))
}

fn create_stream_key(state: &IvsState, payload: &Value) -> Result<Response, LawsError> {
    let channel_arn = payload["channelArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing channelArn".into()))?
        .to_string();

    if !state.channels.contains_key(&channel_arn) {
        return Err(LawsError::NotFound(format!(
            "Channel '{}' not found",
            channel_arn
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:ivs:{REGION}:{ACCOUNT_ID}:stream-key/{id}");
    let value = format!(
        "sk_{}_{}",
        REGION,
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );

    let stream_key = StreamKey {
        arn: arn.clone(),
        channel_arn: channel_arn.clone(),
        value: value.clone(),
    };

    state.stream_keys.insert(arn.clone(), stream_key);

    Ok(json_response(json!({
        "streamKey": {
            "arn": arn,
            "channelArn": channel_arn,
            "value": value
        }
    })))
}

fn list_stream_keys(state: &IvsState, payload: &Value) -> Result<Response, LawsError> {
    let channel_arn = payload["channelArn"].as_str().unwrap_or("");

    let keys: Vec<Value> = state
        .stream_keys
        .iter()
        .filter(|e| channel_arn.is_empty() || e.value().channel_arn == channel_arn)
        .map(|e| {
            let sk = e.value();
            json!({
                "arn": sk.arn,
                "channelArn": sk.channel_arn
            })
        })
        .collect();

    Ok(json_response(json!({
        "streamKeys": keys
    })))
}
