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
pub struct Pipeline {
    pub pipeline_id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub definition: Value,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DataPipelineState {
    pub pipelines: DashMap<String, Pipeline>,
}

impl Default for DataPipelineState {
    fn default() -> Self {
        Self {
            pipelines: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &DataPipelineState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("DataPipeline.").unwrap_or(target);

    let result = match action {
        "CreatePipeline" => create_pipeline(state, payload),
        "DeletePipeline" => delete_pipeline(state, payload),
        "DescribePipelines" => describe_pipelines(state, payload),
        "ListPipelines" => list_pipelines(state),
        "ActivatePipeline" => activate_pipeline(state, payload),
        "PutPipelineDefinition" => put_pipeline_definition(state, payload),
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

fn pipeline_to_json(p: &Pipeline) -> Value {
    json!({
        "pipelineId": p.pipeline_id,
        "name": p.name,
        "description": p.description,
        "pipelineStatus": p.status,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_pipeline(state: &DataPipelineState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing name".into()))?
        .to_string();

    let pipeline_id = uuid::Uuid::new_v4().to_string();
    let description = payload["description"].as_str().unwrap_or("").to_string();

    let pipeline = Pipeline {
        pipeline_id: pipeline_id.clone(),
        name,
        description,
        status: "PENDING".to_string(),
        definition: Value::Null,
    };

    state.pipelines.insert(pipeline_id.clone(), pipeline);
    Ok(json_response(json!({ "pipelineId": pipeline_id })))
}

fn delete_pipeline(state: &DataPipelineState, payload: &Value) -> Result<Response, LawsError> {
    let pipeline_id = payload["pipelineId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing pipelineId".into()))?;

    state
        .pipelines
        .remove(pipeline_id)
        .ok_or_else(|| LawsError::NotFound(format!("Pipeline not found: {pipeline_id}")))?;

    Ok(json_response(json!({})))
}

fn describe_pipelines(state: &DataPipelineState, payload: &Value) -> Result<Response, LawsError> {
    let ids = payload["pipelineIds"]
        .as_array()
        .ok_or_else(|| LawsError::InvalidRequest("Missing pipelineIds".into()))?;

    let mut pipeline_list = Vec::new();
    for id_val in ids {
        if let Some(id) = id_val.as_str() {
            if let Some(p) = state.pipelines.get(id) {
                pipeline_list.push(pipeline_to_json(p.value()));
            }
        }
    }

    Ok(json_response(
        json!({ "pipelineDescriptionList": pipeline_list }),
    ))
}

fn list_pipelines(state: &DataPipelineState) -> Result<Response, LawsError> {
    let ids: Vec<Value> = state
        .pipelines
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({ "id": p.pipeline_id, "name": p.name })
        })
        .collect();

    Ok(json_response(
        json!({ "pipelineIdList": ids, "hasMoreResults": false }),
    ))
}

fn activate_pipeline(state: &DataPipelineState, payload: &Value) -> Result<Response, LawsError> {
    let pipeline_id = payload["pipelineId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing pipelineId".into()))?;

    let mut pipeline = state
        .pipelines
        .get_mut(pipeline_id)
        .ok_or_else(|| LawsError::NotFound(format!("Pipeline not found: {pipeline_id}")))?;

    pipeline.status = "SCHEDULING".to_string();
    Ok(json_response(json!({})))
}

fn put_pipeline_definition(
    state: &DataPipelineState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let pipeline_id = payload["pipelineId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing pipelineId".into()))?;

    let definition = payload
        .get("pipelineObjects")
        .cloned()
        .unwrap_or(Value::Null);

    let mut pipeline = state
        .pipelines
        .get_mut(pipeline_id)
        .ok_or_else(|| LawsError::NotFound(format!("Pipeline not found: {pipeline_id}")))?;

    pipeline.definition = definition;
    Ok(json_response(json!({ "errored": false })))
}
