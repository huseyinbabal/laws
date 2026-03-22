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
    pub name: String,
    pub arn: String,
    pub role_arn: String,
    pub stages: Value,
    pub version: u32,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Clone)]
pub struct PipelineExecution {
    pub pipeline_name: String,
    pub execution_id: String,
    pub status: String,
    pub start_time: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CodePipelineState {
    pub pipelines: DashMap<String, Pipeline>,
    pub executions: DashMap<String, PipelineExecution>,
}

impl Default for CodePipelineState {
    fn default() -> Self {
        Self {
            pipelines: DashMap::new(),
            executions: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &CodePipelineState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target
        .strip_prefix("CodePipeline_20150709.")
        .unwrap_or(target);

    let result = match action {
        "CreatePipeline" => create_pipeline(state, payload).await,
        "DeletePipeline" => delete_pipeline(state, payload).await,
        "GetPipeline" => get_pipeline(state, payload).await,
        "ListPipelines" => list_pipelines(state).await,
        "UpdatePipeline" => update_pipeline(state, payload).await,
        "StartPipelineExecution" => start_pipeline_execution(state, payload).await,
        "GetPipelineExecution" => get_pipeline_execution(state, payload).await,
        "ListPipelineExecutions" => list_pipeline_executions(state, payload).await,
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
        "name": p.name,
        "roleArn": p.role_arn,
        "stages": p.stages,
        "version": p.version,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

async fn create_pipeline(
    state: &CodePipelineState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let pipeline_input = &payload["pipeline"];
    let name = pipeline_input["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("pipeline.name is required".to_string()))?
        .to_string();

    if state.pipelines.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Pipeline already exists: {}",
            name
        )));
    }

    let arn = format!(
        "arn:aws:codepipeline:{}:{}:{}",
        REGION, ACCOUNT_ID, name
    );
    let role_arn = pipeline_input["roleArn"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let stages = pipeline_input["stages"].clone();
    let now = chrono::Utc::now().to_rfc3339();

    let pipeline = Pipeline {
        name: name.clone(),
        arn,
        role_arn,
        stages,
        version: 1,
        created: now.clone(),
        updated: now,
    };

    let resp = pipeline_to_json(&pipeline);
    state.pipelines.insert(name, pipeline);

    Ok(json_response(json!({
        "pipeline": resp,
    })))
}

async fn delete_pipeline(
    state: &CodePipelineState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("name is required".to_string()))?;

    state
        .pipelines
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Pipeline not found: {}", name)))?;

    // Remove associated executions
    state
        .executions
        .retain(|_, e| e.pipeline_name != name);

    Ok(json_response(json!({})))
}

async fn get_pipeline(
    state: &CodePipelineState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("name is required".to_string()))?;

    let pipeline = state
        .pipelines
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("Pipeline not found: {}", name)))?;

    Ok(json_response(json!({
        "pipeline": pipeline_to_json(&pipeline),
        "metadata": {
            "pipelineArn": pipeline.arn,
            "created": pipeline.created,
            "updated": pipeline.updated,
        },
    })))
}

async fn list_pipelines(state: &CodePipelineState) -> Result<Response, LawsError> {
    let pipelines: Vec<Value> = state
        .pipelines
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "name": p.name,
                "version": p.version,
                "created": p.created,
                "updated": p.updated,
            })
        })
        .collect();

    Ok(json_response(json!({
        "pipelines": pipelines,
    })))
}

async fn update_pipeline(
    state: &CodePipelineState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let pipeline_input = &payload["pipeline"];
    let name = pipeline_input["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("pipeline.name is required".to_string()))?;

    let mut pipeline = state
        .pipelines
        .get_mut(name)
        .ok_or_else(|| LawsError::NotFound(format!("Pipeline not found: {}", name)))?;

    if let Some(role_arn) = pipeline_input["roleArn"].as_str() {
        pipeline.role_arn = role_arn.to_string();
    }
    if !pipeline_input["stages"].is_null() {
        pipeline.stages = pipeline_input["stages"].clone();
    }
    pipeline.version += 1;
    pipeline.updated = chrono::Utc::now().to_rfc3339();

    let resp = pipeline_to_json(&pipeline);

    Ok(json_response(json!({
        "pipeline": resp,
    })))
}

async fn start_pipeline_execution(
    state: &CodePipelineState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("name is required".to_string()))?
        .to_string();

    if !state.pipelines.contains_key(&name) {
        return Err(LawsError::NotFound(format!(
            "Pipeline not found: {}",
            name
        )));
    }

    let execution_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let execution = PipelineExecution {
        pipeline_name: name,
        execution_id: execution_id.clone(),
        status: "InProgress".to_string(),
        start_time: now,
    };

    state.executions.insert(execution_id.clone(), execution);

    Ok(json_response(json!({
        "pipelineExecutionId": execution_id,
    })))
}

async fn get_pipeline_execution(
    state: &CodePipelineState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let pipeline_name = payload["pipelineName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("pipelineName is required".to_string()))?;
    let execution_id = payload["pipelineExecutionId"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("pipelineExecutionId is required".to_string())
        })?;

    let exec = state
        .executions
        .get(execution_id)
        .ok_or_else(|| {
            LawsError::NotFound(format!("PipelineExecution not found: {}", execution_id))
        })?;

    if exec.pipeline_name != pipeline_name {
        return Err(LawsError::NotFound(format!(
            "PipelineExecution not found for pipeline: {}",
            pipeline_name
        )));
    }

    Ok(json_response(json!({
        "pipelineExecution": {
            "pipelineName": exec.pipeline_name,
            "pipelineExecutionId": exec.execution_id,
            "status": exec.status,
            "startTime": exec.start_time,
        }
    })))
}

async fn list_pipeline_executions(
    state: &CodePipelineState,
    payload: &serde_json::Value,
) -> Result<Response, LawsError> {
    let pipeline_name = payload["pipelineName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("pipelineName is required".to_string()))?;

    if !state.pipelines.contains_key(pipeline_name) {
        return Err(LawsError::NotFound(format!(
            "Pipeline not found: {}",
            pipeline_name
        )));
    }

    let executions: Vec<Value> = state
        .executions
        .iter()
        .filter(|entry| entry.value().pipeline_name == pipeline_name)
        .map(|entry| {
            let e = entry.value();
            json!({
                "pipelineExecutionId": e.execution_id,
                "status": e.status,
                "startTime": e.start_time,
            })
        })
        .collect();

    Ok(json_response(json!({
        "pipelineExecutionSummaries": executions,
    })))
}
