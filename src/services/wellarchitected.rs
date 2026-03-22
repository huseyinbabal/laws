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
pub struct Workload {
    pub workload_id: String,
    pub workload_name: String,
    pub arn: String,
    pub description: String,
    pub environment: String,
    pub lenses: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub lens_reviews: Vec<LensReview>,
    pub lens_shares: Vec<LensShare>,
}

#[derive(Debug, Clone)]
pub struct LensReview {
    pub lens_alias: String,
    pub lens_arn: String,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct LensShare {
    pub share_id: String,
    pub shared_with: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct WellArchitectedState {
    pub workloads: DashMap<String, Workload>,
}

impl Default for WellArchitectedState {
    fn default() -> Self {
        Self {
            workloads: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<WellArchitectedState>) -> axum::Router {
    axum::Router::new()
        .route("/workloads", post(create_workload).get(list_workloads))
        .route(
            "/workloads/{workload_id}",
            get(get_workload).delete(delete_workload),
        )
        .route(
            "/workloads/{workload_id}/lensShares",
            post(create_lens_share),
        )
        .route(
            "/workloads/{workload_id}/lensReviews",
            get(list_lens_reviews),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateWorkloadRequest {
    #[serde(alias = "WorkloadName")]
    workload_name: String,
    #[serde(alias = "Description", default)]
    description: Option<String>,
    #[serde(alias = "Environment", default)]
    environment: Option<String>,
    #[serde(alias = "Lenses", default)]
    lenses: Option<Vec<String>>,
}

async fn create_workload(
    State(state): State<Arc<WellArchitectedState>>,
    Json(req): Json<CreateWorkloadRequest>,
) -> Response {
    let workload_id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:wellarchitected:{REGION}:{ACCOUNT_ID}:workload/{workload_id}");
    let now = Utc::now().to_rfc3339();

    let workload = Workload {
        workload_id: workload_id.clone(),
        workload_name: req.workload_name.clone(),
        arn: arn.clone(),
        description: req.description.unwrap_or_default(),
        environment: req.environment.unwrap_or_else(|| "PRODUCTION".into()),
        lenses: req.lenses.unwrap_or_default(),
        created_at: now.clone(),
        updated_at: now,
        lens_reviews: Vec::new(),
        lens_shares: Vec::new(),
    };

    state.workloads.insert(workload_id.clone(), workload);

    rest_json::created(json!({
        "WorkloadId": workload_id,
        "WorkloadArn": arn,
    }))
}

async fn list_workloads(State(state): State<Arc<WellArchitectedState>>) -> Response {
    let summaries: Vec<Value> = state
        .workloads
        .iter()
        .map(|entry| {
            let w = entry.value();
            json!({
                "WorkloadId": w.workload_id,
                "WorkloadArn": w.arn,
                "WorkloadName": w.workload_name,
                "UpdatedAt": w.updated_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "WorkloadSummaries": summaries }))
}

async fn get_workload(
    State(state): State<Arc<WellArchitectedState>>,
    Path(workload_id): Path<String>,
) -> Response {
    match state.workloads.get(&workload_id) {
        Some(w) => rest_json::ok(json!({
            "Workload": {
                "WorkloadId": w.workload_id,
                "WorkloadArn": w.arn,
                "WorkloadName": w.workload_name,
                "Description": w.description,
                "Environment": w.environment,
                "Lenses": w.lenses,
                "CreatedAt": w.created_at,
                "UpdatedAt": w.updated_at,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Workload not found: {workload_id}"
        ))),
    }
}

async fn delete_workload(
    State(state): State<Arc<WellArchitectedState>>,
    Path(workload_id): Path<String>,
) -> Response {
    match state.workloads.remove(&workload_id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Workload not found: {workload_id}"
        ))),
    }
}

#[derive(Deserialize)]
struct CreateLensShareRequest {
    #[serde(alias = "SharedWith")]
    shared_with: String,
    #[serde(alias = "LensAlias")]
    lens_alias: String,
}

async fn create_lens_share(
    State(state): State<Arc<WellArchitectedState>>,
    Path(workload_id): Path<String>,
    Json(req): Json<CreateLensShareRequest>,
) -> Response {
    match state.workloads.get_mut(&workload_id) {
        Some(mut w) => {
            let share_id = uuid::Uuid::new_v4().to_string();
            w.lens_shares.push(LensShare {
                share_id: share_id.clone(),
                shared_with: req.shared_with,
                status: "PENDING".to_string(),
            });
            // Also add a lens review stub if it doesn't exist
            if !w
                .lens_reviews
                .iter()
                .any(|r| r.lens_alias == req.lens_alias)
            {
                let now = Utc::now().to_rfc3339();
                w.lens_reviews.push(LensReview {
                    lens_alias: req.lens_alias.clone(),
                    lens_arn: format!(
                        "arn:aws:wellarchitected:{REGION}:{ACCOUNT_ID}:lens/{}",
                        req.lens_alias
                    ),
                    status: "NOT_STARTED".to_string(),
                    updated_at: now,
                });
            }
            rest_json::created(json!({ "ShareId": share_id }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Workload not found: {workload_id}"
        ))),
    }
}

async fn list_lens_reviews(
    State(state): State<Arc<WellArchitectedState>>,
    Path(workload_id): Path<String>,
) -> Response {
    match state.workloads.get(&workload_id) {
        Some(w) => {
            let reviews: Vec<Value> = w
                .lens_reviews
                .iter()
                .map(|r| {
                    json!({
                        "LensAlias": r.lens_alias,
                        "LensArn": r.lens_arn,
                        "LensStatus": r.status,
                        "UpdatedAt": r.updated_at,
                    })
                })
                .collect();
            rest_json::ok(json!({ "LensReviewSummaries": reviews }))
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Workload not found: {workload_id}"
        ))),
    }
}
