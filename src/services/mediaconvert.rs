use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, post};
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
pub struct MediaConvertQueue {
    pub name: String,
    pub arn: String,
    pub queue_type: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct MediaConvertJob {
    pub id: String,
    pub arn: String,
    pub queue: String,
    pub status: String,
    pub role: String,
    pub settings: Value,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct MediaConvertState {
    pub jobs: DashMap<String, MediaConvertJob>,
    pub queues: DashMap<String, MediaConvertQueue>,
}

impl Default for MediaConvertState {
    fn default() -> Self {
        Self {
            jobs: DashMap::new(),
            queues: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<MediaConvertState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/2017-08-29/queues",
            post(create_queue).get(list_queues),
        )
        .route(
            "/2017-08-29/queues/{name}",
            delete(delete_queue),
        )
        .route(
            "/2017-08-29/jobs",
            post(create_job).get(list_jobs),
        )
        .route(
            "/2017-08-29/jobs/{id}",
            get(get_job),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_queue(
    State(state): State<Arc<MediaConvertState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["Name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
            .to_string();

        let queue_type = payload["PricingPlan"]
            .as_str()
            .unwrap_or("ON_DEMAND")
            .to_string();

        let arn = format!(
            "arn:aws:mediaconvert:{REGION}:{ACCOUNT_ID}:queues/{name}"
        );
        let now = chrono::Utc::now().to_rfc3339();

        let queue = MediaConvertQueue {
            name: name.clone(),
            arn: arn.clone(),
            queue_type: queue_type.clone(),
            status: "ACTIVE".to_string(),
            created_at: now.clone(),
        };

        state.queues.insert(name.clone(), queue);

        Ok(rest_json::created(json!({
            "Queue": {
                "Name": name,
                "Arn": arn,
                "Type": queue_type,
                "Status": "ACTIVE",
                "CreatedAt": now,
            }
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_queues(
    State(state): State<Arc<MediaConvertState>>,
) -> Response {
    let queues: Vec<Value> = state
        .queues
        .iter()
        .map(|entry| {
            let q = entry.value();
            json!({
                "Name": q.name,
                "Arn": q.arn,
                "Type": q.queue_type,
                "Status": q.status,
                "CreatedAt": q.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "Queues": queues }))
}

async fn delete_queue(
    State(state): State<Arc<MediaConvertState>>,
    Path(name): Path<String>,
) -> Response {
    match state.queues.remove(&name) {
        Some(_) => rest_json::ok(json!({})),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Queue '{}' not found",
            name
        ))),
    }
}

async fn create_job(
    State(state): State<Arc<MediaConvertState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let role = payload["Role"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let queue = payload["Queue"]
            .as_str()
            .unwrap_or("Default")
            .to_string();

        let settings = payload
            .get("Settings")
            .cloned()
            .unwrap_or(json!({}));

        let id = uuid::Uuid::new_v4().to_string();
        let arn = format!(
            "arn:aws:mediaconvert:{REGION}:{ACCOUNT_ID}:jobs/{id}"
        );
        let now = chrono::Utc::now().to_rfc3339();

        let job = MediaConvertJob {
            id: id.clone(),
            arn: arn.clone(),
            queue: queue.clone(),
            status: "SUBMITTED".to_string(),
            role: role.clone(),
            settings: settings.clone(),
            created_at: now.clone(),
        };

        state.jobs.insert(id.clone(), job);

        Ok(rest_json::created(json!({
            "Job": {
                "Id": id,
                "Arn": arn,
                "Queue": queue,
                "Status": "SUBMITTED",
                "Role": role,
                "Settings": settings,
                "CreatedAt": now,
            }
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_jobs(
    State(state): State<Arc<MediaConvertState>>,
) -> Response {
    let jobs: Vec<Value> = state
        .jobs
        .iter()
        .map(|entry| {
            let j = entry.value();
            json!({
                "Id": j.id,
                "Arn": j.arn,
                "Queue": j.queue,
                "Status": j.status,
                "Role": j.role,
                "CreatedAt": j.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "Jobs": jobs }))
}

async fn get_job(
    State(state): State<Arc<MediaConvertState>>,
    Path(id): Path<String>,
) -> Response {
    match state.jobs.get(&id) {
        Some(job) => rest_json::ok(json!({
            "Job": {
                "Id": job.id,
                "Arn": job.arn,
                "Queue": job.queue,
                "Status": job.status,
                "Role": job.role,
                "Settings": job.settings,
                "CreatedAt": job.created_at,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Job '{}' not found",
            id
        ))),
    }
}
