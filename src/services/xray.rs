use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::post;
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
pub struct XRayGroup {
    pub group_name: String,
    pub group_arn: String,
    pub filter_expression: Option<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct XRayState {
    pub groups: DashMap<String, XRayGroup>,
    pub traces_count: std::sync::atomic::AtomicU64,
}

impl Default for XRayState {
    fn default() -> Self {
        Self {
            groups: DashMap::new(),
            traces_count: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<XRayState>) -> axum::Router {
    axum::Router::new()
        .route("/Traces", post(put_trace_segments))
        .route("/TraceIds", post(get_trace_summaries))
        .route("/Trace/{trace_id}", post(batch_get_traces))
        .route("/Groups", post(create_group))
        .route("/GetGroups", post(get_groups))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn put_trace_segments(
    State(state): State<Arc<XRayState>>,
    Json(payload): Json<Value>,
) -> Response {
    if let Some(docs) = payload["TraceSegmentDocuments"].as_array() {
        state
            .traces_count
            .fetch_add(docs.len() as u64, std::sync::atomic::Ordering::Relaxed);
    }

    rest_json::ok(json!({
        "UnprocessedTraceSegments": []
    }))
}

async fn get_trace_summaries(
    State(_state): State<Arc<XRayState>>,
    Json(_payload): Json<Value>,
) -> Response {
    rest_json::ok(json!({
        "TraceSummaries": [],
        "ApproximateTime": chrono::Utc::now().timestamp(),
        "TracesProcessedCount": 0,
    }))
}

async fn batch_get_traces(
    State(_state): State<Arc<XRayState>>,
    Path(_trace_id): Path<String>,
    Json(_payload): Json<Value>,
) -> Response {
    rest_json::ok(json!({
        "Traces": [],
        "UnprocessedTraceIds": [],
    }))
}

async fn create_group(State(state): State<Arc<XRayState>>, Json(payload): Json<Value>) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let group_name = payload["GroupName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing GroupName".into()))?
            .to_string();

        let filter_expression = payload["FilterExpression"].as_str().map(|s| s.to_string());

        let group_arn = format!("arn:aws:xray:{REGION}:{ACCOUNT_ID}:group/{group_name}");

        let group = XRayGroup {
            group_name: group_name.clone(),
            group_arn: group_arn.clone(),
            filter_expression: filter_expression.clone(),
        };

        state.groups.insert(group_name.clone(), group);

        Ok(rest_json::created(json!({
            "Group": {
                "GroupName": group_name,
                "GroupARN": group_arn,
                "FilterExpression": filter_expression,
            }
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn get_groups(State(state): State<Arc<XRayState>>, Json(_payload): Json<Value>) -> Response {
    let groups: Vec<Value> = state
        .groups
        .iter()
        .map(|entry| {
            let g = entry.value();
            json!({
                "GroupName": g.group_name,
                "GroupARN": g.group_arn,
                "FilterExpression": g.filter_expression,
            })
        })
        .collect();

    rest_json::ok(json!({ "Groups": groups }))
}
