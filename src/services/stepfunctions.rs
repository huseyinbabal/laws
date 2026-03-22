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
pub struct StateMachine {
    pub name: String,
    pub arn: String,
    pub definition: String,
    pub role_arn: String,
    pub status: String,
    pub creation_date: f64,
}

#[derive(Debug, Clone)]
pub struct Execution {
    pub execution_arn: String,
    pub state_machine_arn: String,
    pub name: String,
    pub status: String,
    pub start_date: f64,
    pub input: String,
    pub output: Option<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct StepFunctionsState {
    pub state_machines: DashMap<String, StateMachine>,
    pub executions: DashMap<String, Execution>,
}

impl Default for StepFunctionsState {
    fn default() -> Self {
        Self {
            state_machines: DashMap::new(),
            executions: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &StepFunctionsState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AWSStepFunctions.")
        .unwrap_or(target);

    let result = match action {
        "CreateStateMachine" => create_state_machine(state, payload),
        "DeleteStateMachine" => delete_state_machine(state, payload),
        "ListStateMachines" => list_state_machines(state),
        "DescribeStateMachine" => describe_state_machine(state, payload),
        "StartExecution" => start_execution(state, payload),
        "StopExecution" => stop_execution(state, payload),
        "DescribeExecution" => describe_execution(state, payload),
        "ListExecutions" => list_executions(state, payload),
        "GetExecutionHistory" => get_execution_history(state, payload),
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

fn now_epoch() -> f64 {
    chrono::Utc::now().timestamp() as f64
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_state_machine(
    state: &StepFunctionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("name is required".to_string()))?
        .to_string();

    let definition = payload["definition"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("definition is required".to_string()))?
        .to_string();

    let role_arn = payload["roleArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("roleArn is required".to_string()))?
        .to_string();

    let arn = format!(
        "arn:aws:states:{REGION}:{ACCOUNT_ID}:stateMachine:{name}"
    );

    if state.state_machines.contains_key(&arn) {
        return Err(LawsError::AlreadyExists(format!(
            "State machine '{}' already exists",
            name
        )));
    }

    let creation_date = now_epoch();

    let sm = StateMachine {
        name,
        arn: arn.clone(),
        definition,
        role_arn,
        status: "ACTIVE".to_string(),
        creation_date,
    };

    state.state_machines.insert(arn.clone(), sm);

    Ok(json_response(json!({
        "stateMachineArn": arn,
        "creationDate": creation_date,
    })))
}

fn delete_state_machine(
    state: &StepFunctionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let arn = payload["stateMachineArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("stateMachineArn is required".to_string())
        })?;

    state
        .state_machines
        .remove(arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("State machine '{}' not found", arn))
        })?;

    Ok(json_response(json!({})))
}

fn list_state_machines(state: &StepFunctionsState) -> Result<Response, LawsError> {
    let machines: Vec<Value> = state
        .state_machines
        .iter()
        .map(|entry| {
            let sm = entry.value();
            json!({
                "stateMachineArn": sm.arn,
                "name": sm.name,
                "type": "STANDARD",
                "creationDate": sm.creation_date,
            })
        })
        .collect();

    Ok(json_response(json!({ "stateMachines": machines })))
}

fn describe_state_machine(
    state: &StepFunctionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let arn = payload["stateMachineArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("stateMachineArn is required".to_string())
        })?;

    let sm = state
        .state_machines
        .get(arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("State machine '{}' not found", arn))
        })?;

    Ok(json_response(json!({
        "stateMachineArn": sm.arn,
        "name": sm.name,
        "definition": sm.definition,
        "roleArn": sm.role_arn,
        "status": sm.status,
        "type": "STANDARD",
        "creationDate": sm.creation_date,
    })))
}

fn start_execution(
    state: &StepFunctionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let sm_arn = payload["stateMachineArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("stateMachineArn is required".to_string())
        })?;

    let sm = state
        .state_machines
        .get(sm_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("State machine '{}' not found", sm_arn))
        })?;
    let sm_name = sm.name.clone();

    let exec_name = payload["name"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let input = payload["input"]
        .as_str()
        .unwrap_or("{}")
        .to_string();

    let exec_arn = format!(
        "arn:aws:states:{REGION}:{ACCOUNT_ID}:execution:{sm_name}:{exec_name}"
    );

    let start_date = now_epoch();

    // Simulate instant completion
    let execution = Execution {
        execution_arn: exec_arn.clone(),
        state_machine_arn: sm_arn.to_string(),
        name: exec_name,
        status: "SUCCEEDED".to_string(),
        start_date,
        input: input.clone(),
        output: Some(input),
    };

    state.executions.insert(exec_arn.clone(), execution);

    Ok(json_response(json!({
        "executionArn": exec_arn,
        "startDate": start_date,
    })))
}

fn stop_execution(
    state: &StepFunctionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let exec_arn = payload["executionArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("executionArn is required".to_string())
        })?;

    let mut exec = state
        .executions
        .get_mut(exec_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Execution '{}' not found", exec_arn))
        })?;

    exec.status = "ABORTED".to_string();
    let stop_date = now_epoch();

    Ok(json_response(json!({
        "stopDate": stop_date,
    })))
}

fn describe_execution(
    state: &StepFunctionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let exec_arn = payload["executionArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("executionArn is required".to_string())
        })?;

    let exec = state
        .executions
        .get(exec_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Execution '{}' not found", exec_arn))
        })?;

    let mut resp = json!({
        "executionArn": exec.execution_arn,
        "stateMachineArn": exec.state_machine_arn,
        "name": exec.name,
        "status": exec.status,
        "startDate": exec.start_date,
        "input": exec.input,
    });

    if let Some(ref output) = exec.output {
        resp["output"] = json!(output);
    }

    Ok(json_response(resp))
}

fn list_executions(
    state: &StepFunctionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let sm_arn = payload["stateMachineArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("stateMachineArn is required".to_string())
        })?;

    let executions: Vec<Value> = state
        .executions
        .iter()
        .filter(|entry| entry.value().state_machine_arn == sm_arn)
        .map(|entry| {
            let e = entry.value();
            json!({
                "executionArn": e.execution_arn,
                "stateMachineArn": e.state_machine_arn,
                "name": e.name,
                "status": e.status,
                "startDate": e.start_date,
            })
        })
        .collect();

    Ok(json_response(json!({ "executions": executions })))
}

fn get_execution_history(
    state: &StepFunctionsState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let exec_arn = payload["executionArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("executionArn is required".to_string())
        })?;

    // Verify execution exists
    if !state.executions.contains_key(exec_arn) {
        return Err(LawsError::NotFound(format!(
            "Execution '{}' not found",
            exec_arn
        )));
    }

    // Simplified: return empty events list
    Ok(json_response(json!({ "events": [] })))
}
