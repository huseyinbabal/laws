use axum::body::Bytes;
use axum::http::{HeaderMap, Uri};
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::query::{parse_query_request, xml_error_response, xml_response};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CfStack {
    pub stack_name: String,
    pub stack_id: String,
    pub status: String,
    pub arn: String,
    pub template_body: String,
    pub parameters: Vec<(String, String)>,
    pub creation_time: String,
    pub outputs: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CloudFormationState {
    pub stacks: DashMap<String, CfStack>,
}

impl Default for CloudFormationState {
    fn default() -> Self {
        Self {
            stacks: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &CloudFormationState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target
        .strip_prefix("CloudFormation_20100515.")
        .unwrap_or(target);

    let result = match action {
        "CreateStack" => create_stack(state, payload),
        "DeleteStack" => delete_stack(state, payload),
        "DescribeStacks" => describe_stacks(state, payload),
        "ListStacks" => list_stacks(state, payload),
        "UpdateStack" => update_stack(state, payload),
        "DescribeStackResources" => describe_stack_resources(state, payload),
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

fn stack_to_json(stack: &CfStack) -> Value {
    let parameters: Vec<Value> = stack
        .parameters
        .iter()
        .map(|(k, v)| {
            json!({
                "ParameterKey": k,
                "ParameterValue": v
            })
        })
        .collect();

    let outputs: Vec<Value> = stack
        .outputs
        .iter()
        .map(|(k, v)| {
            json!({
                "OutputKey": k,
                "OutputValue": v
            })
        })
        .collect();

    json!({
        "StackName": stack.stack_name,
        "StackId": stack.stack_id,
        "StackStatus": stack.status,
        "StackArn": stack.arn,
        "CreationTime": stack.creation_time,
        "Parameters": parameters,
        "Outputs": outputs
    })
}

fn parse_parameters(payload: &Value) -> Vec<(String, String)> {
    payload["Parameters"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|p| {
                    let key = p["ParameterKey"].as_str()?.to_string();
                    let value = p["ParameterValue"].as_str()?.to_string();
                    Some((key, value))
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_stack(state: &CloudFormationState, payload: &Value) -> Result<Response, LawsError> {
    let stack_name = payload["StackName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("StackName is required".to_string()))?
        .to_string();

    if state.stacks.contains_key(&stack_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Stack '{}' already exists",
            stack_name
        )));
    }

    let stack_id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:cloudformation:{REGION}:{ACCOUNT_ID}:stack/{stack_name}/{stack_id}"
    );
    let template_body = payload["TemplateBody"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let parameters = parse_parameters(payload);
    let now = chrono::Utc::now().to_rfc3339();

    let stack = CfStack {
        stack_name: stack_name.clone(),
        stack_id: stack_id.clone(),
        status: "CREATE_COMPLETE".to_string(),
        arn,
        template_body,
        parameters,
        creation_time: now,
        outputs: Vec::new(),
    };

    let resp = stack_to_json(&stack);
    state.stacks.insert(stack_name, stack);

    Ok(json_response(json!({ "Stack": resp, "StackId": stack_id })))
}

fn delete_stack(state: &CloudFormationState, payload: &Value) -> Result<Response, LawsError> {
    let stack_name = payload["StackName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("StackName is required".to_string()))?;

    state
        .stacks
        .remove(stack_name)
        .ok_or_else(|| LawsError::NotFound(format!("Stack '{}' not found", stack_name)))?;

    Ok(json_response(json!({})))
}

fn describe_stacks(state: &CloudFormationState, payload: &Value) -> Result<Response, LawsError> {
    let filter_name = payload["StackName"].as_str();

    let stacks: Vec<Value> = state
        .stacks
        .iter()
        .filter(|entry| {
            filter_name
                .map(|name| entry.key() == name)
                .unwrap_or(true)
        })
        .map(|entry| stack_to_json(entry.value()))
        .collect();

    if let Some(name) = filter_name {
        if stacks.is_empty() {
            return Err(LawsError::NotFound(format!(
                "Stack '{}' not found",
                name
            )));
        }
    }

    Ok(json_response(json!({ "Stacks": stacks })))
}

fn list_stacks(state: &CloudFormationState, _payload: &Value) -> Result<Response, LawsError> {
    let summaries: Vec<Value> = state
        .stacks
        .iter()
        .map(|entry| {
            let s = entry.value();
            json!({
                "StackName": s.stack_name,
                "StackId": s.stack_id,
                "StackStatus": s.status,
                "CreationTime": s.creation_time
            })
        })
        .collect();

    Ok(json_response(json!({ "StackSummaries": summaries })))
}

fn update_stack(state: &CloudFormationState, payload: &Value) -> Result<Response, LawsError> {
    let stack_name = payload["StackName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("StackName is required".to_string()))?;

    let mut stack = state
        .stacks
        .get_mut(stack_name)
        .ok_or_else(|| LawsError::NotFound(format!("Stack '{}' not found", stack_name)))?;

    if let Some(template) = payload["TemplateBody"].as_str() {
        stack.template_body = template.to_string();
    }

    let new_params = parse_parameters(payload);
    if !new_params.is_empty() {
        stack.parameters = new_params;
    }

    stack.status = "UPDATE_COMPLETE".to_string();
    let resp = stack_to_json(&stack);

    Ok(json_response(json!({ "Stack": resp })))
}

fn describe_stack_resources(
    state: &CloudFormationState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let stack_name = payload["StackName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("StackName is required".to_string()))?;

    let _stack = state
        .stacks
        .get(stack_name)
        .ok_or_else(|| LawsError::NotFound(format!("Stack '{}' not found", stack_name)))?;

    // Return empty resources list for the mock
    Ok(json_response(json!({ "StackResources": [] })))
}

// ---------------------------------------------------------------------------
// XML helpers for query protocol
// ---------------------------------------------------------------------------

fn stack_to_xml(stack: &CfStack) -> String {
    let params_xml: String = stack
        .parameters
        .iter()
        .map(|(k, v)| {
            format!(
                "<member><ParameterKey>{k}</ParameterKey><ParameterValue>{v}</ParameterValue></member>"
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let outputs_xml: String = stack
        .outputs
        .iter()
        .map(|(k, v)| {
            format!(
                "<member><OutputKey>{k}</OutputKey><OutputValue>{v}</OutputValue></member>"
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        "<member>\
            <StackName>{name}</StackName>\
            <StackId>{id}</StackId>\
            <StackStatus>{status}</StackStatus>\
            <CreationTime>{time}</CreationTime>\
            <Parameters>{params_xml}</Parameters>\
            <Outputs>{outputs_xml}</Outputs>\
        </member>",
        name = stack.stack_name,
        id = stack.stack_id,
        status = stack.status,
        time = stack.creation_time,
    )
}

// ---------------------------------------------------------------------------
// Query protocol handler (XML responses for taws compatibility)
// ---------------------------------------------------------------------------

pub fn handle_query_request(
    state: &CloudFormationState,
    headers: &HeaderMap,
    body: &Bytes,
    uri: &Uri,
) -> Response {
    let req = match parse_query_request(uri, headers, body) {
        Ok(r) => r,
        Err(e) => return xml_error_response(&e),
    };

    let result = match req.action.as_str() {
        "CreateStack" => query_create_stack(state, &req.params),
        "DeleteStack" => query_delete_stack(state, &req.params),
        "DescribeStacks" => query_describe_stacks(state, &req.params),
        "ListStacks" => query_list_stacks(state, &req.params),
        "UpdateStack" => query_update_stack(state, &req.params),
        "DescribeStackResources" => query_describe_stack_resources(state, &req.params),
        _ => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            req.action
        ))),
    };

    match result {
        Ok(resp) => resp,
        Err(e) => xml_error_response(&e),
    }
}

// ---------------------------------------------------------------------------
// Query protocol operations (XML)
// ---------------------------------------------------------------------------

fn query_create_stack(
    state: &CloudFormationState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let stack_name = params
        .get("StackName")
        .ok_or_else(|| LawsError::InvalidRequest("StackName is required".to_string()))?
        .to_string();

    if state.stacks.contains_key(&stack_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Stack '{}' already exists",
            stack_name
        )));
    }

    let stack_id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:cloudformation:{REGION}:{ACCOUNT_ID}:stack/{stack_name}/{stack_id}"
    );
    let template_body = params.get("TemplateBody").cloned().unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();

    // Parse parameters from query params (Parameters.member.N.ParameterKey / ParameterValue)
    let mut parameters = Vec::new();
    for i in 1.. {
        let key_param = format!("Parameters.member.{i}.ParameterKey");
        let val_param = format!("Parameters.member.{i}.ParameterValue");
        if let (Some(k), Some(v)) = (params.get(&key_param), params.get(&val_param)) {
            parameters.push((k.clone(), v.clone()));
        } else {
            break;
        }
    }

    let stack = CfStack {
        stack_name: stack_name.clone(),
        stack_id: stack_id.clone(),
        status: "CREATE_COMPLETE".to_string(),
        arn,
        template_body,
        parameters,
        creation_time: now,
        outputs: Vec::new(),
    };

    let xml = stack_to_xml(&stack);
    state.stacks.insert(stack_name, stack);

    Ok(xml_response("CreateStack", &format!("<StackId>{stack_id}</StackId>")))
}

fn query_delete_stack(
    state: &CloudFormationState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let stack_name = params
        .get("StackName")
        .ok_or_else(|| LawsError::InvalidRequest("StackName is required".to_string()))?;

    state
        .stacks
        .remove(stack_name)
        .ok_or_else(|| LawsError::NotFound(format!("Stack '{}' not found", stack_name)))?;

    Ok(xml_response("DeleteStack", ""))
}

fn query_describe_stacks(
    state: &CloudFormationState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let filter_name = params.get("StackName").map(|s| s.as_str());

    let stacks: Vec<String> = state
        .stacks
        .iter()
        .filter(|entry| {
            filter_name
                .map(|name| entry.key() == name)
                .unwrap_or(true)
        })
        .map(|entry| stack_to_xml(entry.value()))
        .collect();

    if let Some(name) = filter_name {
        if stacks.is_empty() {
            return Err(LawsError::NotFound(format!(
                "Stack '{}' not found",
                name
            )));
        }
    }

    let inner = format!("<Stacks>{}</Stacks>", stacks.join(""));
    Ok(xml_response("DescribeStacks", &inner))
}

fn query_list_stacks(
    state: &CloudFormationState,
    _params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let summaries: Vec<String> = state
        .stacks
        .iter()
        .map(|entry| {
            let s = entry.value();
            format!(
                "<member>\
                    <StackName>{name}</StackName>\
                    <StackId>{id}</StackId>\
                    <StackStatus>{status}</StackStatus>\
                    <CreationTime>{time}</CreationTime>\
                </member>",
                name = s.stack_name,
                id = s.stack_id,
                status = s.status,
                time = s.creation_time,
            )
        })
        .collect();

    let inner = format!("<StackSummaries>{}</StackSummaries>", summaries.join(""));
    Ok(xml_response("ListStacks", &inner))
}

fn query_update_stack(
    state: &CloudFormationState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let stack_name = params
        .get("StackName")
        .ok_or_else(|| LawsError::InvalidRequest("StackName is required".to_string()))?;

    let mut stack = state
        .stacks
        .get_mut(stack_name)
        .ok_or_else(|| LawsError::NotFound(format!("Stack '{}' not found", stack_name)))?;

    if let Some(template) = params.get("TemplateBody") {
        stack.template_body = template.clone();
    }

    stack.status = "UPDATE_COMPLETE".to_string();
    let stack_id = stack.stack_id.clone();

    Ok(xml_response("UpdateStack", &format!("<StackId>{stack_id}</StackId>")))
}

fn query_describe_stack_resources(
    state: &CloudFormationState,
    params: &std::collections::HashMap<String, String>,
) -> Result<Response, LawsError> {
    let stack_name = params
        .get("StackName")
        .ok_or_else(|| LawsError::InvalidRequest("StackName is required".to_string()))?;

    let _stack = state
        .stacks
        .get(stack_name)
        .ok_or_else(|| LawsError::NotFound(format!("Stack '{}' not found", stack_name)))?;

    Ok(xml_response("DescribeStackResources", "<StackResources/>"))
}
