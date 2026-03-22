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
pub struct Flow {
    pub flow_name: String,
    pub arn: String,
    pub description: String,
    pub source_connector_type: String,
    pub destination_connector_type: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AppFlowState {
    pub flows: DashMap<String, Flow>,
}

impl Default for AppFlowState {
    fn default() -> Self {
        Self {
            flows: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &AppFlowState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("SandstoneConfigurationServiceLambda.")
        .unwrap_or(target);

    let result = match action {
        "CreateFlow" => create_flow(state, payload),
        "DeleteFlow" => delete_flow(state, payload),
        "DescribeFlow" => describe_flow(state, payload),
        "ListFlows" => list_flows(state),
        "StartFlow" => start_flow(state, payload),
        "StopFlow" => stop_flow(state, payload),
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

fn flow_to_json(f: &Flow) -> Value {
    json!({
        "flowName": f.flow_name,
        "flowArn": f.arn,
        "description": f.description,
        "sourceConnectorType": f.source_connector_type,
        "destinationConnectorType": f.destination_connector_type,
        "flowStatus": f.status,
        "createdAt": f.created_at,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_flow(state: &AppFlowState, payload: &Value) -> Result<Response, LawsError> {
    let flow_name = payload["flowName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("flowName is required".to_string()))?
        .to_string();

    if state.flows.contains_key(&flow_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Flow '{}' already exists",
            flow_name
        )));
    }

    let arn = format!("arn:aws:appflow:{REGION}:{ACCOUNT_ID}:flow/{flow_name}");
    let now = chrono::Utc::now().to_rfc3339();

    let source = payload["sourceFlowConfig"]["connectorType"]
        .as_str()
        .unwrap_or("Salesforce")
        .to_string();

    let destination = payload["destinationFlowConfigList"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v["connectorType"].as_str())
        .unwrap_or("S3")
        .to_string();

    let description = payload["description"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let flow = Flow {
        flow_name: flow_name.clone(),
        arn: arn.clone(),
        description,
        source_connector_type: source,
        destination_connector_type: destination,
        status: "Active".to_string(),
        created_at: now,
    };

    state.flows.insert(flow_name, flow);

    Ok(json_response(json!({ "flowArn": arn, "flowStatus": "Active" })))
}

fn delete_flow(state: &AppFlowState, payload: &Value) -> Result<Response, LawsError> {
    let flow_name = payload["flowName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("flowName is required".to_string()))?;

    state
        .flows
        .remove(flow_name)
        .ok_or_else(|| LawsError::NotFound(format!("Flow '{}' not found", flow_name)))?;

    Ok(json_response(json!({})))
}

fn describe_flow(state: &AppFlowState, payload: &Value) -> Result<Response, LawsError> {
    let flow_name = payload["flowName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("flowName is required".to_string()))?;

    let flow = state
        .flows
        .get(flow_name)
        .ok_or_else(|| LawsError::NotFound(format!("Flow '{}' not found", flow_name)))?;

    Ok(json_response(flow_to_json(flow.value())))
}

fn list_flows(state: &AppFlowState) -> Result<Response, LawsError> {
    let flows: Vec<Value> = state
        .flows
        .iter()
        .map(|entry| flow_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "flows": flows })))
}

fn start_flow(state: &AppFlowState, payload: &Value) -> Result<Response, LawsError> {
    let flow_name = payload["flowName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("flowName is required".to_string()))?;

    let mut flow = state
        .flows
        .get_mut(flow_name)
        .ok_or_else(|| LawsError::NotFound(format!("Flow '{}' not found", flow_name)))?;

    flow.status = "Active".to_string();

    let execution_id = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "flowArn": flow.arn,
        "flowStatus": "Active",
        "executionId": execution_id,
    })))
}

fn stop_flow(state: &AppFlowState, payload: &Value) -> Result<Response, LawsError> {
    let flow_name = payload["flowName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("flowName is required".to_string()))?;

    let mut flow = state
        .flows
        .get_mut(flow_name)
        .ok_or_else(|| LawsError::NotFound(format!("Flow '{}' not found", flow_name)))?;

    flow.status = "Suspended".to_string();

    Ok(json_response(json!({
        "flowArn": flow.arn,
        "flowStatus": "Suspended",
    })))
}
