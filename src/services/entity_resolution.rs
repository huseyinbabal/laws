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
pub struct MatchingWorkflow {
    pub workflow_name: String,
    pub arn: String,
    pub description: String,
    pub input_source_config: Value,
    pub resolution_techniques: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMapping {
    pub schema_name: String,
    pub arn: String,
    pub description: String,
    pub mapped_input_fields: Value,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct EntityResolutionState {
    pub workflows: DashMap<String, MatchingWorkflow>,
    pub schema_mappings: DashMap<String, SchemaMapping>,
}

impl Default for EntityResolutionState {
    fn default() -> Self {
        Self {
            workflows: DashMap::new(),
            schema_mappings: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<EntityResolutionState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/matchingworkflows",
            post(create_matching_workflow).get(list_matching_workflows),
        )
        .route(
            "/matchingworkflows/{workflow_name}",
            get(get_matching_workflow).delete(delete_matching_workflow),
        )
        .route(
            "/schemamappings",
            post(create_schema_mapping).get(list_schema_mappings),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_matching_workflow(
    State(state): State<Arc<EntityResolutionState>>,
    Json(body): Json<Value>,
) -> Response {
    let workflow_name = match body["workflowName"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest(
                "Missing workflowName".into(),
            ))
        }
    };

    if state.workflows.contains_key(&workflow_name) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "Matching workflow already exists: {workflow_name}"
        )));
    }

    let now = Utc::now().to_rfc3339();
    let arn =
        format!("arn:aws:entityresolution:{REGION}:{ACCOUNT_ID}:matchingworkflow/{workflow_name}");
    let description = body["description"].as_str().unwrap_or("").to_string();

    let workflow = MatchingWorkflow {
        workflow_name: workflow_name.clone(),
        arn: arn.clone(),
        description,
        input_source_config: body["inputSourceConfig"].clone(),
        resolution_techniques: body["resolutionTechniques"].clone(),
        created_at: now.clone(),
        updated_at: now,
    };

    let resp = json!({
        "workflowName": workflow.workflow_name,
        "workflowArn": workflow.arn,
        "description": workflow.description,
    });

    state.workflows.insert(workflow_name, workflow);
    rest_json::created(resp)
}

async fn list_matching_workflows(State(state): State<Arc<EntityResolutionState>>) -> Response {
    let items: Vec<Value> = state
        .workflows
        .iter()
        .map(|entry| {
            let w = entry.value();
            json!({
                "workflowName": w.workflow_name,
                "workflowArn": w.arn,
                "createdAt": w.created_at,
                "updatedAt": w.updated_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "workflowSummaries": items }))
}

async fn get_matching_workflow(
    State(state): State<Arc<EntityResolutionState>>,
    Path(workflow_name): Path<String>,
) -> Response {
    match state.workflows.get(&workflow_name) {
        Some(w) => rest_json::ok(json!({
            "workflowName": w.workflow_name,
            "workflowArn": w.arn,
            "description": w.description,
            "inputSourceConfig": w.input_source_config,
            "resolutionTechniques": w.resolution_techniques,
            "createdAt": w.created_at,
            "updatedAt": w.updated_at,
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Matching workflow not found: {workflow_name}"
        ))),
    }
}

async fn delete_matching_workflow(
    State(state): State<Arc<EntityResolutionState>>,
    Path(workflow_name): Path<String>,
) -> Response {
    match state.workflows.remove(&workflow_name) {
        Some(_) => rest_json::ok(json!({ "message": "Matching workflow deleted" })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Matching workflow not found: {workflow_name}"
        ))),
    }
}

async fn create_schema_mapping(
    State(state): State<Arc<EntityResolutionState>>,
    Json(body): Json<Value>,
) -> Response {
    let schema_name = match body["schemaName"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest(
                "Missing schemaName".into(),
            ))
        }
    };

    if state.schema_mappings.contains_key(&schema_name) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "Schema mapping already exists: {schema_name}"
        )));
    }

    let now = Utc::now().to_rfc3339();
    let arn = format!("arn:aws:entityresolution:{REGION}:{ACCOUNT_ID}:schemamapping/{schema_name}");
    let description = body["description"].as_str().unwrap_or("").to_string();

    let schema = SchemaMapping {
        schema_name: schema_name.clone(),
        arn: arn.clone(),
        description,
        mapped_input_fields: body["mappedInputFields"].clone(),
        created_at: now.clone(),
        updated_at: now,
    };

    let resp = json!({
        "schemaName": schema.schema_name,
        "schemaArn": schema.arn,
        "description": schema.description,
    });

    state.schema_mappings.insert(schema_name, schema);
    rest_json::created(resp)
}

async fn list_schema_mappings(State(state): State<Arc<EntityResolutionState>>) -> Response {
    let items: Vec<Value> = state
        .schema_mappings
        .iter()
        .map(|entry| {
            let s = entry.value();
            json!({
                "schemaName": s.schema_name,
                "schemaArn": s.arn,
                "createdAt": s.created_at,
                "updatedAt": s.updated_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "schemaList": items }))
}
