use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use dashmap::DashMap;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::LawsError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub timestamp: i64,
    pub message: String,
    #[serde(rename = "ingestionTime")]
    pub ingestion_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStream {
    pub name: String,
    pub events: Vec<LogEvent>,
    #[serde(rename = "creationTime")]
    pub creation_time: i64,
    #[serde(rename = "firstEventTimestamp")]
    pub first_event_timestamp: Option<i64>,
    #[serde(rename = "lastEventTimestamp")]
    pub last_event_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogGroup {
    pub name: String,
    pub streams: HashMap<String, LogStream>,
    #[serde(rename = "creationTime")]
    pub creation_time: i64,
    pub arn: String,
}

pub struct CloudWatchLogsState {
    pub log_groups: Arc<DashMap<String, LogGroup>>,
    pub account_id: String,
    pub region: String,
}

impl Default for CloudWatchLogsState {
    fn default() -> Self {
        Self {
            log_groups: Arc::new(DashMap::new()),
            account_id: "000000000000".to_string(),
            region: "us-east-1".to_string(),
        }
    }
}

pub fn router(state: Arc<CloudWatchLogsState>) -> Router {
    Router::new()
        .route("/", post(handle_action))
        .with_state(state)
}

pub async fn handle_request(
    state: &CloudWatchLogsState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("Logs_20140328.").unwrap_or(target);

    let result = match action {
        "CreateLogGroup" => create_log_group(state, payload).await,
        "DeleteLogGroup" => delete_log_group(state, payload).await,
        "DescribeLogGroups" => describe_log_groups(state).await,
        "CreateLogStream" => create_log_stream(state, payload).await,
        "DeleteLogStream" => delete_log_stream(state, payload).await,
        "DescribeLogStreams" => describe_log_streams(state, payload).await,
        "PutLogEvents" => put_log_events(state, payload).await,
        "GetLogEvents" => get_log_events(state, payload).await,
        _ => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            action
        ))),
    };

    match result {
        Ok(resp) => resp,
        Err(e) => e.into_response(),
    }
}

async fn handle_action(
    State(state): State<Arc<CloudWatchLogsState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Response, LawsError> {
    let target = headers
        .get("X-Amz-Target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let action = target.strip_prefix("Logs_20140328.").unwrap_or(target);

    let payload: serde_json::Value =
        serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

    match action {
        "CreateLogGroup" => create_log_group(&state, &payload).await,
        "DeleteLogGroup" => delete_log_group(&state, &payload).await,
        "DescribeLogGroups" => describe_log_groups(&state).await,
        "CreateLogStream" => create_log_stream(&state, &payload).await,
        "DeleteLogStream" => delete_log_stream(&state, &payload).await,
        "DescribeLogStreams" => describe_log_streams(&state, &payload).await,
        "PutLogEvents" => put_log_events(&state, &payload).await,
        "GetLogEvents" => get_log_events(&state, &payload).await,
        _ => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            action
        ))),
    }
}

fn json_response(body: serde_json::Value) -> Response {
    (
        StatusCode::OK,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

async fn create_log_group(
    state: &CloudWatchLogsState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["logGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("logGroupName is required".to_string()))?;

    if state.log_groups.contains_key(name) {
        return Err(LawsError::AlreadyExists(format!(
            "Log group '{}' already exists",
            name
        )));
    }

    let now = chrono::Utc::now().timestamp_millis();
    let arn = format!(
        "arn:aws:logs:{}:{}:log-group:{}",
        state.region, state.account_id, name
    );

    let group = LogGroup {
        name: name.to_string(),
        streams: HashMap::new(),
        creation_time: now,
        arn,
    };

    state.log_groups.insert(name.to_string(), group);

    Ok(json_response(serde_json::json!({})))
}

async fn delete_log_group(
    state: &CloudWatchLogsState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["logGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("logGroupName is required".to_string()))?;

    state
        .log_groups
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Log group '{}' not found", name)))?;

    Ok(json_response(serde_json::json!({})))
}

async fn describe_log_groups(state: &CloudWatchLogsState) -> Result<Response, LawsError> {
    let groups: Vec<serde_json::Value> = state
        .log_groups
        .iter()
        .map(|entry| {
            let g = entry.value();
            serde_json::json!({
                "logGroupName": g.name,
                "creationTime": g.creation_time,
                "arn": g.arn,
                "storedBytes": 0,
                "metricFilterCount": 0,
            })
        })
        .collect();

    Ok(json_response(serde_json::json!({
        "logGroups": groups,
    })))
}

async fn create_log_stream(
    state: &CloudWatchLogsState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let group_name = payload["logGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("logGroupName is required".to_string()))?;
    let stream_name = payload["logStreamName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("logStreamName is required".to_string()))?;

    let mut group = state
        .log_groups
        .get_mut(group_name)
        .ok_or_else(|| LawsError::NotFound(format!("Log group '{}' not found", group_name)))?;

    if group.streams.contains_key(stream_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Log stream '{}' already exists",
            stream_name
        )));
    }

    let now = chrono::Utc::now().timestamp_millis();
    let stream = LogStream {
        name: stream_name.to_string(),
        events: Vec::new(),
        creation_time: now,
        first_event_timestamp: None,
        last_event_timestamp: None,
    };

    group.streams.insert(stream_name.to_string(), stream);

    Ok(json_response(serde_json::json!({})))
}

async fn delete_log_stream(
    state: &CloudWatchLogsState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let group_name = payload["logGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("logGroupName is required".to_string()))?;
    let stream_name = payload["logStreamName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("logStreamName is required".to_string()))?;

    let mut group = state
        .log_groups
        .get_mut(group_name)
        .ok_or_else(|| LawsError::NotFound(format!("Log group '{}' not found", group_name)))?;

    group
        .streams
        .remove(stream_name)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Log stream '{}' not found", stream_name))
        })?;

    Ok(json_response(serde_json::json!({})))
}

async fn describe_log_streams(
    state: &CloudWatchLogsState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let group_name = payload["logGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("logGroupName is required".to_string()))?;

    let group = state
        .log_groups
        .get(group_name)
        .ok_or_else(|| LawsError::NotFound(format!("Log group '{}' not found", group_name)))?;

    let streams: Vec<serde_json::Value> = group
        .streams
        .values()
        .map(|s| {
            let mut obj = serde_json::json!({
                "logStreamName": s.name,
                "creationTime": s.creation_time,
                "storedBytes": 0,
            });
            if let Some(ts) = s.first_event_timestamp {
                obj["firstEventTimestamp"] = serde_json::json!(ts);
            }
            if let Some(ts) = s.last_event_timestamp {
                obj["lastEventTimestamp"] = serde_json::json!(ts);
            }
            obj
        })
        .collect();

    Ok(json_response(serde_json::json!({
        "logStreams": streams,
    })))
}

async fn put_log_events(
    state: &CloudWatchLogsState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let group_name = payload["logGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("logGroupName is required".to_string()))?;
    let stream_name = payload["logStreamName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("logStreamName is required".to_string()))?;
    let log_events = payload["logEvents"]
        .as_array()
        .ok_or_else(|| LawsError::InvalidRequest("logEvents is required".to_string()))?;

    let mut group = state
        .log_groups
        .get_mut(group_name)
        .ok_or_else(|| LawsError::NotFound(format!("Log group '{}' not found", group_name)))?;

    let stream = group
        .streams
        .get_mut(stream_name)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Log stream '{}' not found", stream_name))
        })?;

    let now = chrono::Utc::now().timestamp_millis();

    for event in log_events {
        let timestamp = event["timestamp"].as_i64().unwrap_or(now);
        let message = event["message"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let log_event = LogEvent {
            timestamp,
            message,
            ingestion_time: now,
        };

        // Update stream timestamps
        match stream.first_event_timestamp {
            None => stream.first_event_timestamp = Some(timestamp),
            Some(first) if timestamp < first => stream.first_event_timestamp = Some(timestamp),
            _ => {}
        }
        match stream.last_event_timestamp {
            None => stream.last_event_timestamp = Some(timestamp),
            Some(last) if timestamp > last => stream.last_event_timestamp = Some(timestamp),
            _ => {}
        }

        stream.events.push(log_event);
    }

    Ok(json_response(serde_json::json!({
        "nextSequenceToken": uuid::Uuid::new_v4().to_string(),
    })))
}

async fn get_log_events(
    state: &CloudWatchLogsState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let group_name = payload["logGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("logGroupName is required".to_string()))?;
    let stream_name = payload["logStreamName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("logStreamName is required".to_string()))?;

    let group = state
        .log_groups
        .get(group_name)
        .ok_or_else(|| LawsError::NotFound(format!("Log group '{}' not found", group_name)))?;

    let stream = group
        .streams
        .get(stream_name)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Log stream '{}' not found", stream_name))
        })?;

    let events: Vec<serde_json::Value> = stream
        .events
        .iter()
        .map(|e| {
            serde_json::json!({
                "timestamp": e.timestamp,
                "message": e.message,
                "ingestionTime": e.ingestion_time,
            })
        })
        .collect();

    Ok(json_response(serde_json::json!({
        "events": events,
        "nextForwardToken": format!("f/{}", uuid::Uuid::new_v4()),
        "nextBackwardToken": format!("b/{}", uuid::Uuid::new_v4()),
    })))
}
