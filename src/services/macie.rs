use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
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
pub struct ClassificationJob {
    pub job_id: String,
    pub name: String,
    pub arn: String,
    pub job_type: String,
    pub job_status: String,
    pub created_at: String,
    pub s3_job_definition: Value,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct MacieState {
    pub classification_jobs: DashMap<String, ClassificationJob>,
}

impl Default for MacieState {
    fn default() -> Self {
        Self {
            classification_jobs: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<MacieState>) -> axum::Router {
    axum::Router::new()
        .route("/jobs", post(create_classification_job))
        .route("/jobs/list", get(list_classification_jobs))
        .route("/jobs/{id}", get(describe_classification_job))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_classification_job(
    State(state): State<Arc<MacieState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["name"]
            .as_str()
            .unwrap_or("unnamed")
            .to_string();

        let job_type = payload["jobType"]
            .as_str()
            .unwrap_or("ONE_TIME")
            .to_string();

        let s3_job_definition = payload
            .get("s3JobDefinition")
            .cloned()
            .unwrap_or(json!({}));

        let job_id = uuid::Uuid::new_v4().to_string();
        let arn = format!(
            "arn:aws:macie2:{REGION}:{ACCOUNT_ID}:classification-job/{job_id}"
        );
        let now = chrono::Utc::now().to_rfc3339();

        let job = ClassificationJob {
            job_id: job_id.clone(),
            name,
            arn,
            job_type,
            job_status: "RUNNING".to_string(),
            created_at: now,
            s3_job_definition,
        };

        state.classification_jobs.insert(job_id.clone(), job);

        Ok(rest_json::created(json!({
            "jobId": job_id,
            "jobArn": format!("arn:aws:macie2:{REGION}:{ACCOUNT_ID}:classification-job/{job_id}"),
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_classification_jobs(
    State(state): State<Arc<MacieState>>,
) -> Response {
    let items: Vec<Value> = state
        .classification_jobs
        .iter()
        .map(|entry| {
            let j = entry.value();
            json!({
                "jobId": j.job_id,
                "name": j.name,
                "jobType": j.job_type,
                "jobStatus": j.job_status,
                "createdAt": j.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "items": items }))
}

async fn describe_classification_job(
    State(state): State<Arc<MacieState>>,
    Path(id): Path<String>,
) -> Response {
    match state.classification_jobs.get(&id) {
        Some(job) => rest_json::ok(json!({
            "jobId": job.job_id,
            "name": job.name,
            "jobArn": job.arn,
            "jobType": job.job_type,
            "jobStatus": job.job_status,
            "createdAt": job.created_at,
            "s3JobDefinition": job.s3_job_definition,
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Classification job '{}' not found",
            id
        ))),
    }
}
