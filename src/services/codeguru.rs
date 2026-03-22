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
pub struct ProfilingGroup {
    pub name: String,
    pub arn: String,
    pub compute_platform: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CodeGuruState {
    pub profiling_groups: DashMap<String, ProfilingGroup>,
}

impl Default for CodeGuruState {
    fn default() -> Self {
        Self {
            profiling_groups: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &CodeGuruState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("CodeGuruProfilerService.")
        .unwrap_or(target);

    let result = match action {
        "CreateProfilingGroup" => create_profiling_group(state, payload),
        "DeleteProfilingGroup" => delete_profiling_group(state, payload),
        "DescribeProfilingGroup" => describe_profiling_group(state, payload),
        "ListProfilingGroups" => list_profiling_groups(state),
        "GetRecommendations" => get_recommendations(state, payload),
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

fn json_response(body: Value) -> Response {
    (
        StatusCode::OK,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn profiling_group_to_json(pg: &ProfilingGroup) -> Value {
    json!({
        "name": pg.name,
        "arn": pg.arn,
        "computePlatform": pg.compute_platform,
        "profilingStatus": {
            "latestAggregatedProfile": {
                "start": pg.created_at,
                "period": "PT5M",
            }
        },
        "createdAt": pg.created_at,
        "updatedAt": pg.updated_at,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_profiling_group(state: &CodeGuruState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["profilingGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("profilingGroupName is required".to_string()))?
        .to_string();

    if state.profiling_groups.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Profiling group '{}' already exists",
            name
        )));
    }

    let compute_platform = payload["computePlatform"]
        .as_str()
        .unwrap_or("Default")
        .to_string();

    let arn = format!("arn:aws:codeguru-profiler:{REGION}:{ACCOUNT_ID}:profilingGroup/{name}");
    let now = chrono::Utc::now().to_rfc3339();

    let pg = ProfilingGroup {
        name: name.clone(),
        arn,
        compute_platform,
        status: "ACTIVE".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    let resp = profiling_group_to_json(&pg);
    state.profiling_groups.insert(name, pg);

    Ok(json_response(json!({ "profilingGroup": resp })))
}

fn delete_profiling_group(state: &CodeGuruState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["profilingGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("profilingGroupName is required".to_string()))?;

    state
        .profiling_groups
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Profiling group '{}' not found", name)))?;

    Ok(json_response(json!({})))
}

fn describe_profiling_group(state: &CodeGuruState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["profilingGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("profilingGroupName is required".to_string()))?;

    let pg = state
        .profiling_groups
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Profiling group '{}' not found", name)))?;

    Ok(json_response(json!({
        "profilingGroup": profiling_group_to_json(pg.value()),
    })))
}

fn list_profiling_groups(state: &CodeGuruState) -> Result<Response, LawsError> {
    let groups: Vec<Value> = state
        .profiling_groups
        .iter()
        .map(|entry| profiling_group_to_json(entry.value()))
        .collect();

    let names: Vec<String> = state
        .profiling_groups
        .iter()
        .map(|entry| entry.value().name.clone())
        .collect();

    Ok(json_response(json!({
        "profilingGroups": groups,
        "profilingGroupNames": names,
    })))
}

fn get_recommendations(state: &CodeGuruState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["profilingGroupName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("profilingGroupName is required".to_string()))?;

    let _pg = state
        .profiling_groups
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Profiling group '{}' not found", name)))?;

    Ok(json_response(json!({
        "anomalies": [],
        "recommendations": [
            {
                "allMatchesCount": 3,
                "allMatchesSum": 15.5,
                "pattern": {
                    "name": "High CPU utilization",
                    "description": "Consider optimizing hot methods to reduce CPU consumption",
                    "countersToAggregate": ["cpu_time"],
                    "resolutionSteps": "Review the flame graph for hot paths and optimize them.",
                },
                "topMatches": [],
            }
        ],
        "profileEndTime": chrono::Utc::now().to_rfc3339(),
        "profileStartTime": (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339(),
    })))
}
