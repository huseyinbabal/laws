use std::sync::Arc;

use axum::extract::State;
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
pub struct ComputeEnvironment {
    pub name: String,
    pub arn: String,
    pub type_: String,
    pub state: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct JobQueue {
    pub name: String,
    pub arn: String,
    pub state: String,
    pub priority: u32,
    pub compute_environments: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BatchJob {
    pub job_id: String,
    pub job_name: String,
    pub job_queue: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct BatchState {
    pub compute_environments: DashMap<String, ComputeEnvironment>,
    pub job_queues: DashMap<String, JobQueue>,
    pub jobs: DashMap<String, BatchJob>,
}

impl Default for BatchState {
    fn default() -> Self {
        Self {
            compute_environments: DashMap::new(),
            job_queues: DashMap::new(),
            jobs: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<BatchState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/v1/createcomputeenvironment",
            post(create_compute_environment),
        )
        .route(
            "/v1/deregistercomputeenvironment",
            post(delete_compute_environment),
        )
        .route(
            "/v1/describecomputeenvironments",
            post(describe_compute_environments),
        )
        .route("/v1/createjobqueue", post(create_job_queue))
        .route("/v1/describejobqueues", post(describe_job_queues))
        .route("/v1/submitjob", post(submit_job))
        .route("/v1/describejobs", post(describe_jobs))
        .route("/v1/canceljob", post(cancel_job))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ce_to_json(ce: &ComputeEnvironment) -> Value {
    json!({
        "computeEnvironmentName": ce.name,
        "computeEnvironmentArn": ce.arn,
        "type": ce.type_,
        "state": ce.state,
        "status": ce.status,
    })
}

fn jq_to_json(jq: &JobQueue) -> Value {
    json!({
        "jobQueueName": jq.name,
        "jobQueueArn": jq.arn,
        "state": jq.state,
        "priority": jq.priority,
        "computeEnvironmentOrder": jq.compute_environments,
    })
}

fn job_to_json(j: &BatchJob) -> Value {
    json!({
        "jobId": j.job_id,
        "jobName": j.job_name,
        "jobQueue": j.job_queue,
        "status": j.status,
        "createdAt": j.created_at,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_compute_environment(
    State(state): State<Arc<BatchState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["computeEnvironmentName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing computeEnvironmentName".into()))?
            .to_string();

        let arn = format!("arn:aws:batch:{REGION}:{ACCOUNT_ID}:compute-environment/{name}");

        let type_ = payload["type"].as_str().unwrap_or("MANAGED").to_string();

        let ce = ComputeEnvironment {
            name: name.clone(),
            arn: arn.clone(),
            type_,
            state: "ENABLED".to_string(),
            status: "VALID".to_string(),
        };

        state.compute_environments.insert(name.clone(), ce);

        Ok(rest_json::ok(json!({
            "computeEnvironmentName": name,
            "computeEnvironmentArn": arn,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn delete_compute_environment(
    State(state): State<Arc<BatchState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["computeEnvironment"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing computeEnvironment".into()))?;

        state.compute_environments.remove(name).ok_or_else(|| {
            LawsError::NotFound(format!("Compute environment '{}' not found", name))
        })?;

        Ok(rest_json::ok(json!({})))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn describe_compute_environments(
    State(state): State<Arc<BatchState>>,
    Json(_payload): Json<Value>,
) -> Response {
    let envs: Vec<Value> = state
        .compute_environments
        .iter()
        .map(|entry| ce_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "computeEnvironments": envs }))
}

async fn create_job_queue(
    State(state): State<Arc<BatchState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["jobQueueName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing jobQueueName".into()))?
            .to_string();

        let arn = format!("arn:aws:batch:{REGION}:{ACCOUNT_ID}:job-queue/{name}");

        let priority = payload["priority"].as_u64().unwrap_or(1) as u32;

        let compute_environments: Vec<String> = payload["computeEnvironmentOrder"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["computeEnvironment"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let jq = JobQueue {
            name: name.clone(),
            arn: arn.clone(),
            state: "ENABLED".to_string(),
            priority,
            compute_environments,
        };

        state.job_queues.insert(name.clone(), jq);

        Ok(rest_json::ok(json!({
            "jobQueueName": name,
            "jobQueueArn": arn,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn describe_job_queues(
    State(state): State<Arc<BatchState>>,
    Json(_payload): Json<Value>,
) -> Response {
    let queues: Vec<Value> = state
        .job_queues
        .iter()
        .map(|entry| jq_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "jobQueues": queues }))
}

async fn submit_job(State(state): State<Arc<BatchState>>, Json(payload): Json<Value>) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let job_name = payload["jobName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing jobName".into()))?
            .to_string();

        let job_queue = payload["jobQueue"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing jobQueue".into()))?
            .to_string();

        let job_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let job = BatchJob {
            job_id: job_id.clone(),
            job_name: job_name.clone(),
            job_queue,
            status: "SUBMITTED".to_string(),
            created_at: now,
        };

        state.jobs.insert(job_id.clone(), job);

        Ok(rest_json::ok(json!({
            "jobId": job_id,
            "jobName": job_name,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn describe_jobs(
    State(state): State<Arc<BatchState>>,
    Json(payload): Json<Value>,
) -> Response {
    let job_ids = payload["jobs"].as_array().cloned().unwrap_or_default();

    let jobs: Vec<Value> = job_ids
        .iter()
        .filter_map(|id| {
            let id_str = id.as_str()?;
            state
                .jobs
                .get(id_str)
                .map(|entry| job_to_json(entry.value()))
        })
        .collect();

    rest_json::ok(json!({ "jobs": jobs }))
}

async fn cancel_job(State(state): State<Arc<BatchState>>, Json(payload): Json<Value>) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let job_id = payload["jobId"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing jobId".into()))?;

        let mut job = state
            .jobs
            .get_mut(job_id)
            .ok_or_else(|| LawsError::NotFound(format!("Job '{}' not found", job_id)))?;

        job.status = "FAILED".to_string();

        Ok(rest_json::ok(json!({})))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}
