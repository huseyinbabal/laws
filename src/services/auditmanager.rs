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
pub struct Assessment {
    pub id: String,
    pub arn: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub framework_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Control {
    pub id: String,
    pub arn: String,
    pub name: String,
    pub description: String,
    pub control_type: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AuditManagerState {
    pub assessments: DashMap<String, Assessment>,
    pub controls: DashMap<String, Control>,
}

impl Default for AuditManagerState {
    fn default() -> Self {
        Self {
            assessments: DashMap::new(),
            controls: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<AuditManagerState>) -> axum::Router {
    axum::Router::new()
        .route("/assessments", post(create_assessment).get(list_assessments))
        .route(
            "/assessments/{assessment_id}",
            get(get_assessment).delete(delete_assessment),
        )
        .route("/controls", post(create_control).get(list_controls))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn assessment_to_json(a: &Assessment) -> Value {
    json!({
        "id": a.id,
        "arn": a.arn,
        "name": a.name,
        "description": a.description,
        "status": a.status,
        "frameworkId": a.framework_id,
        "creationTime": a.created_at,
    })
}

fn control_to_json(c: &Control) -> Value {
    json!({
        "id": c.id,
        "arn": c.arn,
        "name": c.name,
        "description": c.description,
        "type": c.control_type,
        "createdAt": c.created_at,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateAssessmentRequest {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "frameworkId", default)]
    framework_id: Option<String>,
}

async fn create_assessment(
    State(state): State<Arc<AuditManagerState>>,
    Json(req): Json<CreateAssessmentRequest>,
) -> Response {
    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:auditmanager:{REGION}:{ACCOUNT_ID}:assessment/{id}"
    );
    let now = Utc::now().to_rfc3339();

    let assessment = Assessment {
        id: id.clone(),
        arn,
        name: req.name,
        description: req.description.unwrap_or_default(),
        status: "ACTIVE".to_string(),
        framework_id: req.framework_id.unwrap_or_default(),
        created_at: now,
    };

    let resp = assessment_to_json(&assessment);
    state.assessments.insert(id, assessment);

    rest_json::created(json!({ "assessment": resp }))
}

async fn list_assessments(State(state): State<Arc<AuditManagerState>>) -> Response {
    let items: Vec<Value> = state
        .assessments
        .iter()
        .map(|entry| assessment_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "assessmentMetadata": items }))
}

async fn get_assessment(
    State(state): State<Arc<AuditManagerState>>,
    Path(assessment_id): Path<String>,
) -> Response {
    match state.assessments.get(&assessment_id) {
        Some(a) => rest_json::ok(json!({ "assessment": assessment_to_json(a.value()) })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Assessment not found: {assessment_id}"
        ))),
    }
}

async fn delete_assessment(
    State(state): State<Arc<AuditManagerState>>,
    Path(assessment_id): Path<String>,
) -> Response {
    match state.assessments.remove(&assessment_id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Assessment not found: {assessment_id}"
        ))),
    }
}

#[derive(Deserialize)]
struct CreateControlRequest {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "controlType", default)]
    control_type: Option<String>,
}

async fn create_control(
    State(state): State<Arc<AuditManagerState>>,
    Json(req): Json<CreateControlRequest>,
) -> Response {
    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:auditmanager:{REGION}:{ACCOUNT_ID}:control/{id}"
    );
    let now = Utc::now().to_rfc3339();

    let control = Control {
        id: id.clone(),
        arn,
        name: req.name,
        description: req.description.unwrap_or_default(),
        control_type: req.control_type.unwrap_or_else(|| "Custom".to_string()),
        created_at: now,
    };

    let resp = control_to_json(&control);
    state.controls.insert(id, control);

    rest_json::created(json!({ "control": resp }))
}

async fn list_controls(State(state): State<Arc<AuditManagerState>>) -> Response {
    let items: Vec<Value> = state
        .controls
        .iter()
        .map(|entry| control_to_json(entry.value()))
        .collect();

    rest_json::ok(json!({ "controlMetadata": items }))
}
