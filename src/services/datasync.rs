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
pub struct DataSyncTask {
    pub task_arn: String,
    pub name: String,
    pub status: String,
    pub source_location_arn: String,
    pub destination_location_arn: String,
}

#[derive(Debug, Clone)]
pub struct DataSyncLocation {
    pub location_arn: String,
    pub location_uri: String,
    pub location_type: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DataSyncState {
    pub tasks: DashMap<String, DataSyncTask>,
    pub locations: DashMap<String, DataSyncLocation>,
}

impl Default for DataSyncState {
    fn default() -> Self {
        Self {
            tasks: DashMap::new(),
            locations: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &DataSyncState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("FmrsService.")
        .unwrap_or(target);

    let result = match action {
        "CreateTask" => create_task(state, payload),
        "DeleteTask" => delete_task(state, payload),
        "DescribeTask" => describe_task(state, payload),
        "ListTasks" => list_tasks(state),
        "CreateLocationS3" => create_location_s3(state, payload),
        "ListLocations" => list_locations(state),
        "StartTaskExecution" => start_task_execution(state, payload),
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

fn task_to_json(t: &DataSyncTask) -> Value {
    json!({
        "TaskArn": t.task_arn,
        "Name": t.name,
        "Status": t.status,
        "SourceLocationArn": t.source_location_arn,
        "DestinationLocationArn": t.destination_location_arn,
    })
}

fn location_to_json(l: &DataSyncLocation) -> Value {
    json!({
        "LocationArn": l.location_arn,
        "LocationUri": l.location_uri,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_task(state: &DataSyncState, payload: &Value) -> Result<Response, LawsError> {
    let task_id = uuid::Uuid::new_v4().to_string();
    let task_arn = format!("arn:aws:datasync:{REGION}:{ACCOUNT_ID}:task/task-{task_id}");

    let name = payload["Name"].as_str().unwrap_or("").to_string();
    let source = payload["SourceLocationArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing SourceLocationArn".into()))?
        .to_string();
    let destination = payload["DestinationLocationArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DestinationLocationArn".into()))?
        .to_string();

    let task = DataSyncTask {
        task_arn: task_arn.clone(),
        name,
        status: "AVAILABLE".to_string(),
        source_location_arn: source,
        destination_location_arn: destination,
    };

    state.tasks.insert(task_arn.clone(), task);
    Ok(json_response(json!({ "TaskArn": task_arn })))
}

fn delete_task(state: &DataSyncState, payload: &Value) -> Result<Response, LawsError> {
    let task_arn = payload["TaskArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing TaskArn".into()))?;

    state
        .tasks
        .remove(task_arn)
        .ok_or_else(|| LawsError::NotFound(format!("Task not found: {task_arn}")))?;

    Ok(json_response(json!({})))
}

fn describe_task(state: &DataSyncState, payload: &Value) -> Result<Response, LawsError> {
    let task_arn = payload["TaskArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing TaskArn".into()))?;

    let task = state
        .tasks
        .get(task_arn)
        .ok_or_else(|| LawsError::NotFound(format!("Task not found: {task_arn}")))?;

    Ok(json_response(task_to_json(task.value())))
}

fn list_tasks(state: &DataSyncState) -> Result<Response, LawsError> {
    let tasks: Vec<Value> = state
        .tasks
        .iter()
        .map(|entry| {
            let t = entry.value();
            json!({ "TaskArn": t.task_arn, "Name": t.name, "Status": t.status })
        })
        .collect();

    Ok(json_response(json!({ "Tasks": tasks })))
}

fn create_location_s3(state: &DataSyncState, payload: &Value) -> Result<Response, LawsError> {
    let loc_id = uuid::Uuid::new_v4().to_string();
    let location_arn = format!("arn:aws:datasync:{REGION}:{ACCOUNT_ID}:location/loc-{loc_id}");

    let s3_bucket = payload["S3BucketArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing S3BucketArn".into()))?;
    let subdirectory = payload["Subdirectory"].as_str().unwrap_or("/");
    let location_uri = format!("s3://{}{}", s3_bucket.rsplit(':').next().unwrap_or(s3_bucket), subdirectory);

    let location = DataSyncLocation {
        location_arn: location_arn.clone(),
        location_uri,
        location_type: "S3".to_string(),
    };

    state.locations.insert(location_arn.clone(), location);
    Ok(json_response(json!({ "LocationArn": location_arn })))
}

fn list_locations(state: &DataSyncState) -> Result<Response, LawsError> {
    let locations: Vec<Value> = state
        .locations
        .iter()
        .map(|entry| location_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "Locations": locations })))
}

fn start_task_execution(state: &DataSyncState, payload: &Value) -> Result<Response, LawsError> {
    let task_arn = payload["TaskArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing TaskArn".into()))?;

    let _task = state
        .tasks
        .get(task_arn)
        .ok_or_else(|| LawsError::NotFound(format!("Task not found: {task_arn}")))?;

    let exec_id = uuid::Uuid::new_v4().to_string();
    let execution_arn = format!("{task_arn}/execution/exec-{exec_id}");

    Ok(json_response(json!({ "TaskExecutionArn": execution_arn })))
}
