use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NotebookInstance {
    pub name: String,
    pub arn: String,
    pub instance_type: String,
    pub status: String,
    pub role_arn: String,
    pub created: String,
}

#[derive(Debug, Clone)]
pub struct TrainingJob {
    pub name: String,
    pub arn: String,
    pub status: String,
    pub algorithm_specification: Value,
    pub role_arn: String,
    pub created: String,
    pub training_start_time: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SageMakerEndpoint {
    pub name: String,
    pub arn: String,
    pub status: String,
    pub endpoint_config_name: String,
    pub created: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct SageMakerState {
    pub notebook_instances: DashMap<String, NotebookInstance>,
    pub training_jobs: DashMap<String, TrainingJob>,
    pub endpoints: DashMap<String, SageMakerEndpoint>,
}

impl Default for SageMakerState {
    fn default() -> Self {
        Self {
            notebook_instances: DashMap::new(),
            training_jobs: DashMap::new(),
            endpoints: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &SageMakerState, target: &str, payload: &serde_json::Value) -> Response {
    let action = target.strip_prefix("SageMaker.").unwrap_or(target);

    let result = match action {
        "CreateNotebookInstance" => create_notebook_instance(state, payload),
        "DeleteNotebookInstance" => delete_notebook_instance(state, payload),
        "DescribeNotebookInstance" => describe_notebook_instance(state, payload),
        "ListNotebookInstances" => list_notebook_instances(state),
        "StartNotebookInstance" => start_notebook_instance(state, payload),
        "StopNotebookInstance" => stop_notebook_instance(state, payload),
        "CreateTrainingJob" => create_training_job(state, payload),
        "DescribeTrainingJob" => describe_training_job(state, payload),
        "ListTrainingJobs" => list_training_jobs(state),
        "CreateEndpoint" => create_endpoint(state, payload),
        "DeleteEndpoint" => delete_endpoint(state, payload),
        "DescribeEndpoint" => describe_endpoint(state, payload),
        "ListEndpoints" => list_endpoints(state),
        other => Err(LawsError::InvalidRequest(format!("unknown action: {other}"))),
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
    (StatusCode::OK, [("Content-Type", "application/x-amz-json-1.1")], serde_json::to_string(&body).unwrap_or_default()).into_response()
}

fn require_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, LawsError> {
    body.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest(format!("missing required field: {field}")))
}

// ---------------------------------------------------------------------------
// Notebook Instance Operations
// ---------------------------------------------------------------------------

fn create_notebook_instance(state: &SageMakerState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "NotebookInstanceName")?.to_owned();
    let instance_type = body
        .get("InstanceType")
        .and_then(|v| v.as_str())
        .unwrap_or("ml.t2.medium")
        .to_owned();
    let role_arn = body
        .get("RoleArn")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    if state.notebook_instances.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "notebook instance already exists: {name}"
        )));
    }

    let arn = format!(
        "arn:aws:sagemaker:{REGION}:{ACCOUNT_ID}:notebook-instance/{name}"
    );
    let created = chrono::Utc::now().to_rfc3339();

    let instance = NotebookInstance {
        name: name.clone(),
        arn: arn.clone(),
        instance_type,
        status: "InService".into(),
        role_arn,
        created,
    };

    state.notebook_instances.insert(name, instance);

    Ok(json_response(json!({
        "NotebookInstanceArn": arn
    })))
}

fn delete_notebook_instance(state: &SageMakerState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "NotebookInstanceName")?;
    state
        .notebook_instances
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("notebook instance not found: {name}")))?;

    Ok(json_response(json!({})))
}

fn describe_notebook_instance(state: &SageMakerState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "NotebookInstanceName")?;
    let instance = state
        .notebook_instances
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("notebook instance not found: {name}")))?;

    let i = instance.value();
    Ok(json_response(json!({
        "NotebookInstanceArn": i.arn,
        "NotebookInstanceName": i.name,
        "NotebookInstanceStatus": i.status,
        "InstanceType": i.instance_type,
        "RoleArn": i.role_arn,
        "CreationTime": i.created
    })))
}

fn list_notebook_instances(state: &SageMakerState) -> Result<Response, LawsError> {
    let instances: Vec<Value> = state
        .notebook_instances
        .iter()
        .map(|entry| {
            let i = entry.value();
            json!({
                "NotebookInstanceArn": i.arn,
                "NotebookInstanceName": i.name,
                "NotebookInstanceStatus": i.status,
                "InstanceType": i.instance_type,
                "CreationTime": i.created
            })
        })
        .collect();

    Ok(json_response(json!({
        "NotebookInstances": instances
    })))
}

fn start_notebook_instance(state: &SageMakerState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "NotebookInstanceName")?;
    let mut instance = state
        .notebook_instances
        .get_mut(name)
        .ok_or_else(|| LawsError::NotFound(format!("notebook instance not found: {name}")))?;

    instance.status = "InService".into();
    Ok(json_response(json!({})))
}

fn stop_notebook_instance(state: &SageMakerState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "NotebookInstanceName")?;
    let mut instance = state
        .notebook_instances
        .get_mut(name)
        .ok_or_else(|| LawsError::NotFound(format!("notebook instance not found: {name}")))?;

    instance.status = "Stopped".into();
    Ok(json_response(json!({})))
}

// ---------------------------------------------------------------------------
// Training Job Operations
// ---------------------------------------------------------------------------

fn create_training_job(state: &SageMakerState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "TrainingJobName")?.to_owned();
    let role_arn = body
        .get("RoleArn")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let algorithm_specification = body
        .get("AlgorithmSpecification")
        .cloned()
        .unwrap_or(json!({}));

    if state.training_jobs.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "training job already exists: {name}"
        )));
    }

    let arn = format!(
        "arn:aws:sagemaker:{REGION}:{ACCOUNT_ID}:training-job/{name}"
    );
    let created = chrono::Utc::now().to_rfc3339();

    let job = TrainingJob {
        name: name.clone(),
        arn: arn.clone(),
        status: "Completed".into(),
        algorithm_specification,
        role_arn,
        created: created.clone(),
        training_start_time: Some(created),
    };

    state.training_jobs.insert(name, job);

    Ok(json_response(json!({
        "TrainingJobArn": arn
    })))
}

fn describe_training_job(state: &SageMakerState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "TrainingJobName")?;
    let job = state
        .training_jobs
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("training job not found: {name}")))?;

    let j = job.value();
    Ok(json_response(json!({
        "TrainingJobArn": j.arn,
        "TrainingJobName": j.name,
        "TrainingJobStatus": j.status,
        "AlgorithmSpecification": j.algorithm_specification,
        "RoleArn": j.role_arn,
        "CreationTime": j.created,
        "TrainingStartTime": j.training_start_time
    })))
}

fn list_training_jobs(state: &SageMakerState) -> Result<Response, LawsError> {
    let jobs: Vec<Value> = state
        .training_jobs
        .iter()
        .map(|entry| {
            let j = entry.value();
            json!({
                "TrainingJobArn": j.arn,
                "TrainingJobName": j.name,
                "TrainingJobStatus": j.status,
                "CreationTime": j.created
            })
        })
        .collect();

    Ok(json_response(json!({
        "TrainingJobSummaries": jobs
    })))
}

// ---------------------------------------------------------------------------
// Endpoint Operations
// ---------------------------------------------------------------------------

fn create_endpoint(state: &SageMakerState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "EndpointName")?.to_owned();
    let endpoint_config_name = body
        .get("EndpointConfigName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    if state.endpoints.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "endpoint already exists: {name}"
        )));
    }

    let arn = format!(
        "arn:aws:sagemaker:{REGION}:{ACCOUNT_ID}:endpoint/{name}"
    );
    let created = chrono::Utc::now().to_rfc3339();

    let endpoint = SageMakerEndpoint {
        name: name.clone(),
        arn: arn.clone(),
        status: "InService".into(),
        endpoint_config_name,
        created,
    };

    state.endpoints.insert(name, endpoint);

    Ok(json_response(json!({
        "EndpointArn": arn
    })))
}

fn delete_endpoint(state: &SageMakerState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "EndpointName")?;
    state
        .endpoints
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("endpoint not found: {name}")))?;

    Ok(json_response(json!({})))
}

fn describe_endpoint(state: &SageMakerState, body: &Value) -> Result<Response, LawsError> {
    let name = require_str(body, "EndpointName")?;
    let endpoint = state
        .endpoints
        .get(name)
        .ok_or_else(|| LawsError::NotFound(format!("endpoint not found: {name}")))?;

    let e = endpoint.value();
    Ok(json_response(json!({
        "EndpointArn": e.arn,
        "EndpointName": e.name,
        "EndpointStatus": e.status,
        "EndpointConfigName": e.endpoint_config_name,
        "CreationTime": e.created
    })))
}

fn list_endpoints(state: &SageMakerState) -> Result<Response, LawsError> {
    let endpoints: Vec<Value> = state
        .endpoints
        .iter()
        .map(|entry| {
            let e = entry.value();
            json!({
                "EndpointArn": e.arn,
                "EndpointName": e.name,
                "EndpointStatus": e.status,
                "CreationTime": e.created
            })
        })
        .collect();

    Ok(json_response(json!({
        "Endpoints": endpoints
    })))
}
