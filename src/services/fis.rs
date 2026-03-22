use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentTemplate {
    pub id: String,
    pub description: String,
    pub targets: Value,
    pub actions: Value,
    pub stop_conditions: Value,
    pub role_arn: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: String,
    pub template_id: String,
    pub state: ExperimentState,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentState {
    pub status: String,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct FisState {
    pub templates: DashMap<String, ExperimentTemplate>,
    pub experiments: DashMap<String, Experiment>,
}

impl Default for FisState {
    fn default() -> Self {
        Self {
            templates: DashMap::new(),
            experiments: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<FisState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/experimentTemplates",
            post(create_experiment_template).get(list_experiment_templates),
        )
        .route(
            "/experimentTemplates/{template_id}",
            get(get_experiment_template).delete(delete_experiment_template),
        )
        .route("/experiments", post(start_experiment).get(list_experiments))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_experiment_template(
    State(state): State<Arc<FisState>>,
    Json(body): Json<Value>,
) -> Response {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let description = body["description"].as_str().unwrap_or("").to_string();
    let role_arn = body["roleArn"].as_str().unwrap_or("").to_string();

    let template = ExperimentTemplate {
        id: id.clone(),
        description,
        targets: body["targets"].clone(),
        actions: body["actions"].clone(),
        stop_conditions: body["stopConditions"].clone(),
        role_arn,
        created_at: now.clone(),
        updated_at: now,
    };

    let resp = json!({
        "experimentTemplate": {
            "id": template.id,
            "description": template.description,
            "targets": template.targets,
            "actions": template.actions,
            "stopConditions": template.stop_conditions,
            "roleArn": template.role_arn,
            "creationTime": template.created_at,
            "lastUpdateTime": template.updated_at,
        }
    });

    state.templates.insert(id, template);
    rest_json::created(resp)
}

async fn list_experiment_templates(State(state): State<Arc<FisState>>) -> Response {
    let items: Vec<Value> = state
        .templates
        .iter()
        .map(|entry| {
            let t = entry.value();
            json!({
                "id": t.id,
                "description": t.description,
                "creationTime": t.created_at,
                "lastUpdateTime": t.updated_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "experimentTemplates": items }))
}

async fn get_experiment_template(
    State(state): State<Arc<FisState>>,
    Path(template_id): Path<String>,
) -> Response {
    match state.templates.get(&template_id) {
        Some(t) => rest_json::ok(json!({
            "experimentTemplate": {
                "id": t.id,
                "description": t.description,
                "targets": t.targets,
                "actions": t.actions,
                "stopConditions": t.stop_conditions,
                "roleArn": t.role_arn,
                "creationTime": t.created_at,
                "lastUpdateTime": t.updated_at,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Experiment template not found: {template_id}"
        ))),
    }
}

async fn delete_experiment_template(
    State(state): State<Arc<FisState>>,
    Path(template_id): Path<String>,
) -> Response {
    match state.templates.remove(&template_id) {
        Some((_, t)) => rest_json::ok(json!({
            "experimentTemplate": {
                "id": t.id,
                "description": t.description,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Experiment template not found: {template_id}"
        ))),
    }
}

async fn start_experiment(State(state): State<Arc<FisState>>, Json(body): Json<Value>) -> Response {
    let template_id = match body["experimentTemplateId"].as_str() {
        Some(id) => id.to_string(),
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest(
                "Missing experimentTemplateId".into(),
            ))
        }
    };

    if !state.templates.contains_key(&template_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Experiment template not found: {template_id}"
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let experiment = Experiment {
        id: id.clone(),
        template_id: template_id.clone(),
        state: ExperimentState {
            status: "running".to_string(),
            reason: "Experiment started".to_string(),
        },
        created_at: now,
    };

    let resp = json!({
        "experiment": {
            "id": experiment.id,
            "experimentTemplateId": experiment.template_id,
            "state": {
                "status": experiment.state.status,
                "reason": experiment.state.reason,
            },
            "creationTime": experiment.created_at,
        }
    });

    state.experiments.insert(id, experiment);
    rest_json::created(resp)
}

async fn list_experiments(State(state): State<Arc<FisState>>) -> Response {
    let items: Vec<Value> = state
        .experiments
        .iter()
        .map(|entry| {
            let e = entry.value();
            json!({
                "id": e.id,
                "experimentTemplateId": e.template_id,
                "state": {
                    "status": e.state.status,
                    "reason": e.state.reason,
                },
                "creationTime": e.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "experiments": items }))
}
