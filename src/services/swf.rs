use axum::response::{IntoResponse, Response};
use chrono::Utc;
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
pub struct Domain {
    pub name: String,
    pub status: String,
    pub description: String,
    pub arn: String,
    pub retention_period: i64,
}

#[derive(Debug, Clone)]
pub struct WorkflowType {
    pub domain: String,
    pub name: String,
    pub version: String,
    pub status: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    pub domain: String,
    pub workflow_id: String,
    pub run_id: String,
    pub workflow_type_name: String,
    pub workflow_type_version: String,
    pub status: String,
    pub start_timestamp: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct SwfState {
    pub domains: DashMap<String, Domain>,
    pub workflow_types: DashMap<String, WorkflowType>,
    pub executions: DashMap<String, WorkflowExecution>,
}

impl Default for SwfState {
    fn default() -> Self {
        Self {
            domains: DashMap::new(),
            workflow_types: DashMap::new(),
            executions: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &SwfState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("SimpleWorkflowService.")
        .unwrap_or(target);

    let result = match action {
        "RegisterDomain" => register_domain(state, payload),
        "ListDomains" => list_domains(state, payload),
        "RegisterWorkflowType" => register_workflow_type(state, payload),
        "ListWorkflowTypes" => list_workflow_types(state, payload),
        "StartWorkflowExecution" => start_workflow_execution(state, payload),
        "ListOpenWorkflowExecutions" => list_open_workflow_executions(state, payload),
        "TerminateWorkflowExecution" => terminate_workflow_execution(state, payload),
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

fn json_response(status: StatusCode, body: Value) -> Response {
    (status, axum::Json(body)).into_response()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn register_domain(state: &SwfState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing name".into()))?
        .to_string();

    if state.domains.contains_key(&name) {
        return Err(LawsError::AlreadyExists(format!(
            "Domain already exists: {name}"
        )));
    }

    let description = payload["description"].as_str().unwrap_or("").to_string();
    let retention_period = payload["workflowExecutionRetentionPeriodInDays"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(30);

    let arn = format!("arn:aws:swf:{REGION}:{ACCOUNT_ID}:/domain/{name}");

    let domain = Domain {
        name: name.clone(),
        status: "REGISTERED".to_string(),
        description,
        arn,
        retention_period,
    };

    state.domains.insert(name, domain);
    Ok(json_response(StatusCode::OK, json!({})))
}

fn list_domains(state: &SwfState, payload: &Value) -> Result<Response, LawsError> {
    let registration_status = payload["registrationStatus"]
        .as_str()
        .unwrap_or("REGISTERED");

    let domains: Vec<Value> = state
        .domains
        .iter()
        .filter(|entry| entry.value().status == registration_status)
        .map(|entry| {
            let d = entry.value();
            json!({
                "name": d.name,
                "status": d.status,
                "description": d.description,
                "arn": d.arn,
            })
        })
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "domainInfos": domains }),
    ))
}

fn register_workflow_type(state: &SwfState, payload: &Value) -> Result<Response, LawsError> {
    let domain = payload["domain"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing domain".into()))?
        .to_string();
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing name".into()))?
        .to_string();
    let version = payload["version"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing version".into()))?
        .to_string();

    let key = format!("{domain}:{name}:{version}");
    if state.workflow_types.contains_key(&key) {
        return Err(LawsError::AlreadyExists(format!(
            "WorkflowType already exists: {key}"
        )));
    }

    let description = payload["description"].as_str().unwrap_or("").to_string();

    let wt = WorkflowType {
        domain,
        name,
        version,
        status: "REGISTERED".to_string(),
        description,
    };

    state.workflow_types.insert(key, wt);
    Ok(json_response(StatusCode::OK, json!({})))
}

fn list_workflow_types(state: &SwfState, payload: &Value) -> Result<Response, LawsError> {
    let domain = payload["domain"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing domain".into()))?;

    let types: Vec<Value> = state
        .workflow_types
        .iter()
        .filter(|entry| entry.value().domain == domain)
        .map(|entry| {
            let wt = entry.value();
            json!({
                "workflowType": {
                    "name": wt.name,
                    "version": wt.version,
                },
                "status": wt.status,
                "description": wt.description,
            })
        })
        .collect();

    Ok(json_response(StatusCode::OK, json!({ "typeInfos": types })))
}

fn start_workflow_execution(state: &SwfState, payload: &Value) -> Result<Response, LawsError> {
    let domain = payload["domain"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing domain".into()))?
        .to_string();
    let workflow_id = payload["workflowId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing workflowId".into()))?
        .to_string();
    let wt_name = payload["workflowType"]["name"]
        .as_str()
        .unwrap_or("default")
        .to_string();
    let wt_version = payload["workflowType"]["version"]
        .as_str()
        .unwrap_or("1.0")
        .to_string();

    let run_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let execution = WorkflowExecution {
        domain,
        workflow_id: workflow_id.clone(),
        run_id: run_id.clone(),
        workflow_type_name: wt_name,
        workflow_type_version: wt_version,
        status: "OPEN".to_string(),
        start_timestamp: now,
    };

    let key = format!("{}:{}", workflow_id, run_id);
    state.executions.insert(key, execution);

    Ok(json_response(StatusCode::OK, json!({ "runId": run_id })))
}

fn list_open_workflow_executions(state: &SwfState, payload: &Value) -> Result<Response, LawsError> {
    let domain = payload["domain"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing domain".into()))?;

    let executions: Vec<Value> = state
        .executions
        .iter()
        .filter(|entry| {
            let e = entry.value();
            e.domain == domain && e.status == "OPEN"
        })
        .map(|entry| {
            let e = entry.value();
            json!({
                "execution": {
                    "workflowId": e.workflow_id,
                    "runId": e.run_id,
                },
                "workflowType": {
                    "name": e.workflow_type_name,
                    "version": e.workflow_type_version,
                },
                "startTimestamp": e.start_timestamp,
                "executionStatus": e.status,
            })
        })
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "executionInfos": executions }),
    ))
}

fn terminate_workflow_execution(state: &SwfState, payload: &Value) -> Result<Response, LawsError> {
    let domain = payload["domain"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing domain".into()))?;
    let workflow_id = payload["workflowId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing workflowId".into()))?;

    let mut found = false;
    for mut entry in state.executions.iter_mut() {
        let e = entry.value_mut();
        if e.domain == domain && e.workflow_id == workflow_id && e.status == "OPEN" {
            e.status = "TERMINATED".to_string();
            found = true;
            break;
        }
    }

    if !found {
        return Err(LawsError::NotFound(format!(
            "No open execution found for workflow: {workflow_id}"
        )));
    }

    Ok(json_response(StatusCode::OK, json!({})))
}
