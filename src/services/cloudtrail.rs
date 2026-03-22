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
pub struct Trail {
    pub name: String,
    pub arn: String,
    pub s3_bucket_name: String,
    pub is_multi_region: bool,
    pub is_logging: bool,
    pub home_region: String,
}

#[derive(Debug, Clone)]
pub struct CloudTrailEvent {
    pub event_id: String,
    pub event_name: String,
    pub event_source: String,
    pub event_time: String,
    pub username: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CloudTrailState {
    pub trails: DashMap<String, Trail>,
}

impl Default for CloudTrailState {
    fn default() -> Self {
        Self {
            trails: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &CloudTrailState, target: &str, payload: &Value) -> Response {
    let action = target.rsplit('.').next().unwrap_or(target);

    let result = match action {
        "CreateTrail" => create_trail(state, payload),
        "DeleteTrail" => delete_trail(state, payload),
        "DescribeTrails" => describe_trails(state, payload),
        "GetTrailStatus" => get_trail_status(state, payload),
        "StartLogging" => start_logging(state, payload),
        "StopLogging" => stop_logging(state, payload),
        "LookupEvents" => lookup_events(state, payload),
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn json_response(status: StatusCode, body: Value) -> Response {
    (
        status,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn trail_to_json(t: &Trail) -> Value {
    json!({
        "Name": t.name,
        "TrailARN": t.arn,
        "S3BucketName": t.s3_bucket_name,
        "IsMultiRegionTrail": t.is_multi_region,
        "HomeRegion": t.home_region,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_trail(state: &CloudTrailState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    if state.trails.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Trail '{}' already exists",
            name
        )));
    }

    let s3_bucket = payload["S3BucketName"]
        .as_str()
        .unwrap_or("aws-cloudtrail-logs")
        .to_string();

    let is_multi_region = payload["IsMultiRegionTrail"].as_bool().unwrap_or(false);

    let arn = format!("arn:aws:cloudtrail:{REGION}:{ACCOUNT_ID}:trail/{name}");

    let trail = Trail {
        name: name.clone(),
        arn,
        s3_bucket_name: s3_bucket,
        is_multi_region,
        is_logging: false,
        home_region: REGION.to_string(),
    };

    let resp = trail_to_json(&trail);
    state.trails.insert(name, trail);

    Ok(json_response(StatusCode::OK, resp))
}

fn delete_trail(state: &CloudTrailState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    state
        .trails
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Trail '{}' not found", name)))?;

    Ok(json_response(StatusCode::OK, json!({})))
}

fn describe_trails(state: &CloudTrailState, _payload: &Value) -> Result<Response, LawsError> {
    let trail_list: Vec<Value> = state
        .trails
        .iter()
        .map(|entry| trail_to_json(entry.value()))
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "trailList": trail_list }),
    ))
}

fn get_trail_status(state: &CloudTrailState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    let trail = state
        .trails
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Trail '{}' not found", name)))?;

    Ok(json_response(
        StatusCode::OK,
        json!({
            "IsLogging": trail.is_logging,
            "LatestDeliveryAttemptTime": "",
            "LatestNotificationAttemptTime": "",
        }),
    ))
}

fn start_logging(state: &CloudTrailState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    let mut trail = state
        .trails
        .get_mut(name)
        .ok_or_else(|| LawsError::NotFound(format!("Trail '{}' not found", name)))?;

    trail.is_logging = true;

    Ok(json_response(StatusCode::OK, json!({})))
}

fn stop_logging(state: &CloudTrailState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?;

    let mut trail = state
        .trails
        .get_mut(name)
        .ok_or_else(|| LawsError::NotFound(format!("Trail '{}' not found", name)))?;

    trail.is_logging = false;

    Ok(json_response(StatusCode::OK, json!({})))
}

fn lookup_events(state: &CloudTrailState, _payload: &Value) -> Result<Response, LawsError> {
    // Return empty events list (no real events in mock)
    let _ = state;
    Ok(json_response(
        StatusCode::OK,
        json!({
            "Events": [],
            "NextToken": null,
        }),
    ))
}
