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
pub struct DeviceFarmProject {
    pub arn: String,
    pub name: String,
    pub created: String,
}

#[derive(Debug, Clone)]
pub struct Upload {
    pub arn: String,
    pub project_arn: String,
    pub name: String,
    pub upload_type: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DeviceFarmState {
    pub projects: DashMap<String, DeviceFarmProject>,
    pub uploads: DashMap<String, Upload>,
}

impl Default for DeviceFarmState {
    fn default() -> Self {
        Self {
            projects: DashMap::new(),
            uploads: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &DeviceFarmState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("DeviceFarm_20150623.")
        .unwrap_or(target);

    let result = match action {
        "CreateProject" => create_project(state, payload),
        "ListProjects" => list_projects(state),
        "GetProject" => get_project(state, payload),
        "DeleteProject" => delete_project(state, payload),
        "CreateUpload" => create_upload(state, payload),
        "ListUploads" => list_uploads(state, payload),
        "ScheduleRun" => schedule_run(state, payload),
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

fn project_to_json(p: &DeviceFarmProject) -> Value {
    json!({
        "arn": p.arn,
        "name": p.name,
        "created": p.created,
    })
}

fn upload_to_json(u: &Upload) -> Value {
    json!({
        "arn": u.arn,
        "projectArn": u.project_arn,
        "name": u.name,
        "type": u.upload_type,
        "status": u.status,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_project(state: &DeviceFarmState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing name".into()))?
        .to_string();

    let project_id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:devicefarm:{REGION}:{ACCOUNT_ID}:project:{project_id}");
    let now = chrono::Utc::now().to_rfc3339();

    let project = DeviceFarmProject {
        arn: arn.clone(),
        name,
        created: now,
    };

    state.projects.insert(arn.clone(), project.clone());
    Ok(json_response(json!({ "project": project_to_json(&project) })))
}

fn list_projects(state: &DeviceFarmState) -> Result<Response, LawsError> {
    let projects: Vec<Value> = state
        .projects
        .iter()
        .map(|entry| project_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "projects": projects })))
}

fn get_project(state: &DeviceFarmState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload["arn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing arn".into()))?;

    let project = state
        .projects
        .get(arn)
        .ok_or_else(|| LawsError::NotFound(format!("Project not found: {arn}")))?;

    Ok(json_response(json!({ "project": project_to_json(project.value()) })))
}

fn delete_project(state: &DeviceFarmState, payload: &Value) -> Result<Response, LawsError> {
    let arn = payload["arn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing arn".into()))?;

    state
        .projects
        .remove(arn)
        .ok_or_else(|| LawsError::NotFound(format!("Project not found: {arn}")))?;

    state.uploads.retain(|_, u| u.project_arn != arn);
    Ok(json_response(json!({})))
}

fn create_upload(state: &DeviceFarmState, payload: &Value) -> Result<Response, LawsError> {
    let project_arn = payload["projectArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing projectArn".into()))?
        .to_string();

    if !state.projects.contains_key(&project_arn) {
        return Err(LawsError::NotFound(format!("Project not found: {project_arn}")));
    }

    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing name".into()))?
        .to_string();

    let upload_type = payload["type"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing type".into()))?
        .to_string();

    let upload_id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:devicefarm:{REGION}:{ACCOUNT_ID}:upload:{upload_id}");

    let upload = Upload {
        arn: arn.clone(),
        project_arn,
        name,
        upload_type,
        status: "INITIALIZED".to_string(),
    };

    state.uploads.insert(arn.clone(), upload.clone());
    Ok(json_response(json!({ "upload": upload_to_json(&upload) })))
}

fn list_uploads(state: &DeviceFarmState, payload: &Value) -> Result<Response, LawsError> {
    let project_arn = payload["arn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing arn".into()))?;

    let uploads: Vec<Value> = state
        .uploads
        .iter()
        .filter(|entry| entry.value().project_arn == project_arn)
        .map(|entry| upload_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "uploads": uploads })))
}

fn schedule_run(state: &DeviceFarmState, payload: &Value) -> Result<Response, LawsError> {
    let project_arn = payload["projectArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing projectArn".into()))?;

    if !state.projects.contains_key(project_arn) {
        return Err(LawsError::NotFound(format!("Project not found: {project_arn}")));
    }

    let run_id = uuid::Uuid::new_v4().to_string();
    let run_arn = format!("arn:aws:devicefarm:{REGION}:{ACCOUNT_ID}:run:{run_id}");

    Ok(json_response(json!({
        "run": {
            "arn": run_arn,
            "name": payload["name"].as_str().unwrap_or("test-run"),
            "status": "SCHEDULING",
            "result": "PENDING",
        }
    })))
}
