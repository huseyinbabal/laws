use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::Deserialize;
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
pub struct Application {
    pub application_id: String,
    pub name: String,
    pub arn: String,
    pub release_label: String,
    pub application_type: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct JobRun {
    pub application_id: String,
    pub job_run_id: String,
    pub arn: String,
    pub name: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
    pub execution_role: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct EmrServerlessState {
    pub applications: DashMap<String, Application>,
    pub job_runs: DashMap<String, JobRun>,
}

impl Default for EmrServerlessState {
    fn default() -> Self {
        Self {
            applications: DashMap::new(),
            job_runs: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<EmrServerlessState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/applications",
            post(create_application).get(list_applications),
        )
        .route(
            "/applications/{application_id}",
            get(get_application).delete(delete_application),
        )
        .route(
            "/applications/{application_id}/jobruns",
            post(start_job_run).get(list_job_runs),
        )
        .route(
            "/applications/{application_id}/jobruns/{job_run_id}",
            get(get_job_run),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateApplicationRequest {
    name: String,
    #[serde(default, rename = "releaseLabel")]
    release_label: Option<String>,
    #[serde(default, rename = "type")]
    application_type: Option<String>,
}

async fn create_application(
    State(state): State<Arc<EmrServerlessState>>,
    Json(req): Json<CreateApplicationRequest>,
) -> Response {
    let application_id = uuid::Uuid::new_v4().to_string();
    let arn =
        format!("arn:aws:emr-serverless:{REGION}:{ACCOUNT_ID}:/applications/{application_id}");
    let now = Utc::now().to_rfc3339();

    let app = Application {
        application_id: application_id.clone(),
        name: req.name.clone(),
        arn: arn.clone(),
        release_label: req.release_label.unwrap_or_else(|| "emr-6.15.0".into()),
        application_type: req.application_type.unwrap_or_else(|| "SPARK".into()),
        state: "CREATED".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    state.applications.insert(application_id.clone(), app);

    rest_json::created(json!({
        "applicationId": application_id,
        "name": req.name,
        "arn": arn,
    }))
}

async fn list_applications(State(state): State<Arc<EmrServerlessState>>) -> Response {
    let apps: Vec<Value> = state
        .applications
        .iter()
        .map(|entry| {
            let a = entry.value();
            json!({
                "id": a.application_id,
                "name": a.name,
                "arn": a.arn,
                "releaseLabel": a.release_label,
                "type": a.application_type,
                "state": a.state,
                "createdAt": a.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "applications": apps }))
}

async fn get_application(
    State(state): State<Arc<EmrServerlessState>>,
    Path(application_id): Path<String>,
) -> Response {
    match state.applications.get(&application_id) {
        Some(a) => rest_json::ok(json!({
            "application": {
                "applicationId": a.application_id,
                "name": a.name,
                "arn": a.arn,
                "releaseLabel": a.release_label,
                "type": a.application_type,
                "state": a.state,
                "createdAt": a.created_at,
                "updatedAt": a.updated_at,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Application not found: {application_id}"
        ))),
    }
}

async fn delete_application(
    State(state): State<Arc<EmrServerlessState>>,
    Path(application_id): Path<String>,
) -> Response {
    match state.applications.remove(&application_id) {
        Some(_) => rest_json::ok(json!({})),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Application not found: {application_id}"
        ))),
    }
}

#[derive(Deserialize)]
struct StartJobRunRequest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default, rename = "executionRoleArn")]
    execution_role_arn: Option<String>,
}

async fn start_job_run(
    State(state): State<Arc<EmrServerlessState>>,
    Path(application_id): Path<String>,
    Json(req): Json<StartJobRunRequest>,
) -> Response {
    if !state.applications.contains_key(&application_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Application not found: {application_id}"
        )));
    }

    let job_run_id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:emr-serverless:{REGION}:{ACCOUNT_ID}:/applications/{application_id}/jobruns/{job_run_id}"
    );
    let now = Utc::now().to_rfc3339();

    let job_run = JobRun {
        application_id: application_id.clone(),
        job_run_id: job_run_id.clone(),
        arn: arn.clone(),
        name: req.name.unwrap_or_else(|| format!("job-{job_run_id}")),
        state: "SUBMITTED".to_string(),
        created_at: now.clone(),
        updated_at: now,
        execution_role: req
            .execution_role_arn
            .unwrap_or_else(|| format!("arn:aws:iam::{ACCOUNT_ID}:role/emr-serverless-role")),
    };

    state
        .job_runs
        .insert(format!("{application_id}:{job_run_id}"), job_run);

    rest_json::ok(json!({
        "applicationId": application_id,
        "jobRunId": job_run_id,
        "arn": arn,
    }))
}

async fn list_job_runs(
    State(state): State<Arc<EmrServerlessState>>,
    Path(application_id): Path<String>,
) -> Response {
    let runs: Vec<Value> = state
        .job_runs
        .iter()
        .filter(|entry| entry.value().application_id == application_id)
        .map(|entry| {
            let j = entry.value();
            json!({
                "applicationId": j.application_id,
                "id": j.job_run_id,
                "arn": j.arn,
                "name": j.name,
                "state": j.state,
                "createdAt": j.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "jobRuns": runs }))
}

async fn get_job_run(
    State(state): State<Arc<EmrServerlessState>>,
    Path((application_id, job_run_id)): Path<(String, String)>,
) -> Response {
    let key = format!("{application_id}:{job_run_id}");
    match state.job_runs.get(&key) {
        Some(j) => rest_json::ok(json!({
            "jobRun": {
                "applicationId": j.application_id,
                "jobRunId": j.job_run_id,
                "arn": j.arn,
                "name": j.name,
                "state": j.state,
                "createdAt": j.created_at,
                "updatedAt": j.updated_at,
                "executionRole": j.execution_role,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "JobRun not found: {job_run_id}"
        ))),
    }
}
