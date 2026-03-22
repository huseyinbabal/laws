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
pub struct AthenaWorkGroup {
    pub name: String,
    pub state: String,
    pub description: String,
    pub arn: String,
}

#[derive(Debug, Clone)]
pub struct QueryExecution {
    pub query_execution_id: String,
    pub query: String,
    pub work_group: String,
    pub database: Option<String>,
    pub status: String,
    pub submitted_at: String,
    pub completion_time: Option<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AthenaState {
    pub work_groups: DashMap<String, AthenaWorkGroup>,
    pub query_executions: DashMap<String, QueryExecution>,
}

impl Default for AthenaState {
    fn default() -> Self {
        Self {
            work_groups: DashMap::new(),
            query_executions: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &AthenaState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("AmazonAthena.").unwrap_or(target);

    let result = match action {
        "CreateWorkGroup" => create_work_group(state, payload).await,
        "DeleteWorkGroup" => delete_work_group(state, payload).await,
        "ListWorkGroups" => list_work_groups(state).await,
        "GetWorkGroup" => get_work_group(state, payload).await,
        "StartQueryExecution" => start_query_execution(state, payload).await,
        "GetQueryExecution" => get_query_execution(state, payload).await,
        "GetQueryResults" => get_query_results(state, payload).await,
        "StopQueryExecution" => stop_query_execution(state, payload).await,
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

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

async fn create_work_group(
    state: &AthenaState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Name is required".to_string()))?
        .to_string();

    if state.work_groups.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "WorkGroup already exists: {}",
            name
        )));
    }

    let description = payload["Description"].as_str().unwrap_or("").to_string();
    let arn = format!(
        "arn:aws:athena:{}:{}:workgroup/{}",
        REGION, ACCOUNT_ID, name
    );

    let wg = AthenaWorkGroup {
        name: name.clone(),
        state: "ENABLED".to_string(),
        description,
        arn,
    };

    state.work_groups.insert(name, wg);

    Ok(json_response(json!({})))
}

async fn delete_work_group(
    state: &AthenaState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["WorkGroup"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("WorkGroup is required".to_string()))?;

    state
        .work_groups
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("WorkGroup not found: {}", name)))?;

    Ok(json_response(json!({})))
}

async fn list_work_groups(state: &AthenaState) -> Result<Response, LawsError> {
    let work_groups: Vec<Value> = state
        .work_groups
        .iter()
        .map(|entry| {
            let wg = entry.value();
            json!({
                "Name": wg.name,
                "State": wg.state,
                "Description": wg.description,
            })
        })
        .collect();

    Ok(json_response(json!({
        "WorkGroups": work_groups,
    })))
}

async fn get_work_group(
    state: &AthenaState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["WorkGroup"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("WorkGroup is required".to_string()))?;

    let wg = state
        .work_groups
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("WorkGroup not found: {}", name)))?;

    Ok(json_response(json!({
        "WorkGroup": {
            "Name": wg.name,
            "State": wg.state,
            "Description": wg.description,
            "WorkGroupArn": wg.arn,
        }
    })))
}

async fn start_query_execution(
    state: &AthenaState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let query = payload["QueryString"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("QueryString is required".to_string()))?
        .to_string();

    let work_group = payload["WorkGroup"]
        .as_str()
        .unwrap_or("primary")
        .to_string();

    let database = payload["QueryExecutionContext"]["Database"]
        .as_str()
        .map(|s| s.to_string());

    let query_execution_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let execution = QueryExecution {
        query_execution_id: query_execution_id.clone(),
        query,
        work_group,
        database,
        status: "SUCCEEDED".to_string(),
        submitted_at: now.clone(),
        completion_time: Some(now),
    };

    state
        .query_executions
        .insert(query_execution_id.clone(), execution);

    Ok(json_response(json!({
        "QueryExecutionId": query_execution_id,
    })))
}

async fn get_query_execution(
    state: &AthenaState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let id = payload["QueryExecutionId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("QueryExecutionId is required".to_string()))?;

    let exec = state
        .query_executions
        .get(id)
        .ok_or_else(|| LawsError::NotFound(format!("QueryExecution not found: {}", id)))?;

    Ok(json_response(json!({
        "QueryExecution": {
            "QueryExecutionId": exec.query_execution_id,
            "Query": exec.query,
            "WorkGroup": exec.work_group,
            "QueryExecutionContext": {
                "Database": exec.database,
            },
            "Status": {
                "State": exec.status,
                "SubmissionDateTime": exec.submitted_at,
                "CompletionDateTime": exec.completion_time,
            },
        }
    })))
}

async fn get_query_results(
    state: &AthenaState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let id = payload["QueryExecutionId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("QueryExecutionId is required".to_string()))?;

    if !state.query_executions.contains_key(id) {
        return Err(LawsError::NotFound(format!(
            "QueryExecution not found: {}",
            id
        )));
    }

    Ok(json_response(json!({
        "ResultSet": {
            "Rows": [],
            "ResultSetMetadata": {
                "ColumnInfo": [],
            },
        }
    })))
}

async fn stop_query_execution(
    state: &AthenaState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let id = payload["QueryExecutionId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("QueryExecutionId is required".to_string()))?;

    if let Some(mut exec) = state.query_executions.get_mut(id) {
        exec.status = "CANCELLED".to_string();
    }

    Ok(json_response(json!({})))
}
