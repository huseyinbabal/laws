use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
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
pub struct Detector {
    pub detector_id: String,
    pub status: String,
    pub created_at: String,
    pub finding_publishing_frequency: String,
    pub service_role: String,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub finding_id: String,
    pub detector_id: String,
    pub severity: f64,
    pub title: String,
    pub finding_type: String,
    pub description: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct GuardDutyState {
    pub detectors: DashMap<String, Detector>,
}

impl Default for GuardDutyState {
    fn default() -> Self {
        Self {
            detectors: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<GuardDutyState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/detector",
            axum::routing::post(create_detector).get(list_detectors),
        )
        .route(
            "/detector/{id}",
            axum::routing::get(get_detector).delete(delete_detector),
        )
        .route(
            "/detector/{id}/findings",
            axum::routing::post(create_findings),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn detector_to_json(d: &Detector) -> Value {
    json!({
        "detectorId": d.detector_id,
        "status": d.status,
        "createdAt": d.created_at,
        "findingPublishingFrequency": d.finding_publishing_frequency,
        "serviceRole": d.service_role,
    })
}

fn finding_to_json(f: &Finding) -> Value {
    json!({
        "id": f.finding_id,
        "detectorId": f.detector_id,
        "severity": f.severity,
        "title": f.title,
        "type": f.finding_type,
        "description": f.description,
        "createdAt": f.created_at,
        "accountId": ACCOUNT_ID,
        "region": REGION,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_detector(
    State(state): State<Arc<GuardDutyState>>,
    Json(payload): Json<Value>,
) -> Response {
    let detector_id = uuid::Uuid::new_v4().to_string().replace("-", "")[..32].to_string();
    let finding_frequency = payload["findingPublishingFrequency"]
        .as_str()
        .unwrap_or("SIX_HOURS")
        .to_string();

    let service_role = format!(
        "arn:aws:iam::{ACCOUNT_ID}:role/aws-service-role/guardduty.amazonaws.com/AWSServiceRoleForAmazonGuardDuty"
    );

    let detector = Detector {
        detector_id: detector_id.clone(),
        status: "ENABLED".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        finding_publishing_frequency: finding_frequency,
        service_role,
        findings: Vec::new(),
    };

    state.detectors.insert(detector_id.clone(), detector);

    rest_json::created(json!({ "detectorId": detector_id }))
}

async fn list_detectors(State(state): State<Arc<GuardDutyState>>) -> Response {
    let ids: Vec<String> = state
        .detectors
        .iter()
        .map(|entry| entry.value().detector_id.clone())
        .collect();

    rest_json::ok(json!({ "detectorIds": ids }))
}

async fn get_detector(
    State(state): State<Arc<GuardDutyState>>,
    Path(id): Path<String>,
) -> Response {
    match state.detectors.get(&id) {
        Some(d) => rest_json::ok(detector_to_json(&d)),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Detector not found: {id}")))
        }
    }
}

async fn delete_detector(
    State(state): State<Arc<GuardDutyState>>,
    Path(id): Path<String>,
) -> Response {
    match state.detectors.remove(&id) {
        Some(_) => rest_json::no_content(),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Detector not found: {id}")))
        }
    }
}

async fn create_findings(
    State(state): State<Arc<GuardDutyState>>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let mut detector = match state.detectors.get_mut(&id) {
        Some(d) => d,
        None => {
            return rest_json::error_response(&LawsError::NotFound(format!(
                "Detector not found: {id}"
            )));
        }
    };

    let finding_types = payload["findingTypes"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    for ft_val in &finding_types {
        let finding_type = ft_val
            .as_str()
            .unwrap_or("UnauthorizedAccess:EC2/MaliciousIPCaller.Custom")
            .to_string();
        let finding_id = uuid::Uuid::new_v4().to_string().replace("-", "")[..32].to_string();

        let finding = Finding {
            finding_id,
            detector_id: id.clone(),
            severity: 5.0,
            title: format!("{} finding", finding_type),
            finding_type,
            description: "Sample finding generated by GuardDuty mock".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        detector.findings.push(finding);
    }

    rest_json::ok(json!({}))
}
