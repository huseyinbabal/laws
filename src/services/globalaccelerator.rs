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
pub struct Accelerator {
    pub accelerator_arn: String,
    pub name: String,
    pub ip_address_type: String,
    pub enabled: bool,
    pub dns_name: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Listener {
    pub listener_arn: String,
    pub accelerator_arn: String,
    pub protocol: String,
    pub from_port: u16,
    pub to_port: u16,
}

#[derive(Debug, Clone)]
pub struct EndpointGroup {
    pub endpoint_group_arn: String,
    pub listener_arn: String,
    pub endpoint_group_region: String,
    pub health_check_protocol: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct GlobalAcceleratorState {
    pub accelerators: DashMap<String, Accelerator>,
    pub listeners: DashMap<String, Listener>,
    pub endpoint_groups: DashMap<String, EndpointGroup>,
}

impl Default for GlobalAcceleratorState {
    fn default() -> Self {
        Self {
            accelerators: DashMap::new(),
            listeners: DashMap::new(),
            endpoint_groups: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &GlobalAcceleratorState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("GlobalAccelerator_V20180706.")
        .unwrap_or(target);

    let result = match action {
        "CreateAccelerator" => create_accelerator(state, payload),
        "DeleteAccelerator" => delete_accelerator(state, payload),
        "DescribeAccelerator" => describe_accelerator(state, payload),
        "ListAccelerators" => list_accelerators(state),
        "CreateListener" => create_listener(state, payload),
        "ListListeners" => list_listeners(state, payload),
        "CreateEndpointGroup" => create_endpoint_group(state, payload),
        other => Err(LawsError::InvalidRequest(format!(
            "Unknown action: {}",
            other
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

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_accelerator(
    state: &GlobalAcceleratorState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["Name"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
        .to_string();

    let ip_address_type = payload["IpAddressType"]
        .as_str()
        .unwrap_or("IPV4")
        .to_string();

    let enabled = payload["Enabled"].as_bool().unwrap_or(true);

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:globalaccelerator::{ACCOUNT_ID}:accelerator/{id}");
    let dns_name = format!("{id}.awsglobalaccelerator.com");
    let now = chrono::Utc::now().to_rfc3339();

    let accelerator = Accelerator {
        accelerator_arn: arn.clone(),
        name: name.clone(),
        ip_address_type: ip_address_type.clone(),
        enabled,
        dns_name: dns_name.clone(),
        status: "DEPLOYED".into(),
        created_at: now.clone(),
    };

    state.accelerators.insert(arn.clone(), accelerator);

    Ok(json_response(json!({
        "Accelerator": {
            "AcceleratorArn": arn,
            "Name": name,
            "IpAddressType": ip_address_type,
            "Enabled": enabled,
            "DnsName": dns_name,
            "Status": "DEPLOYED",
            "CreatedTime": now
        }
    })))
}

fn delete_accelerator(
    state: &GlobalAcceleratorState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let arn = payload["AcceleratorArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing AcceleratorArn".into()))?;

    state
        .accelerators
        .remove(arn)
        .ok_or_else(|| LawsError::NotFound(format!("Accelerator '{}' not found", arn)))?;

    Ok(json_response(json!({})))
}

fn describe_accelerator(
    state: &GlobalAcceleratorState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let arn = payload["AcceleratorArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing AcceleratorArn".into()))?;

    let acc = state
        .accelerators
        .get(arn)
        .ok_or_else(|| LawsError::NotFound(format!("Accelerator '{}' not found", arn)))?;

    Ok(json_response(json!({
        "Accelerator": {
            "AcceleratorArn": acc.accelerator_arn,
            "Name": acc.name,
            "IpAddressType": acc.ip_address_type,
            "Enabled": acc.enabled,
            "DnsName": acc.dns_name,
            "Status": acc.status,
            "CreatedTime": acc.created_at
        }
    })))
}

fn list_accelerators(
    state: &GlobalAcceleratorState,
) -> Result<Response, LawsError> {
    let accs: Vec<Value> = state
        .accelerators
        .iter()
        .map(|e| {
            let a = e.value();
            json!({
                "AcceleratorArn": a.accelerator_arn,
                "Name": a.name,
                "IpAddressType": a.ip_address_type,
                "Enabled": a.enabled,
                "DnsName": a.dns_name,
                "Status": a.status,
                "CreatedTime": a.created_at
            })
        })
        .collect();

    Ok(json_response(json!({
        "Accelerators": accs
    })))
}

fn create_listener(
    state: &GlobalAcceleratorState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let accelerator_arn = payload["AcceleratorArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing AcceleratorArn".into()))?
        .to_string();

    let protocol = payload["Protocol"]
        .as_str()
        .unwrap_or("TCP")
        .to_string();

    let from_port = payload["PortRanges"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v["FromPort"].as_u64())
        .unwrap_or(80) as u16;

    let to_port = payload["PortRanges"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v["ToPort"].as_u64())
        .unwrap_or(80) as u16;

    let id = uuid::Uuid::new_v4().to_string();
    let listener_arn = format!("{accelerator_arn}/listener/{id}");

    let listener = Listener {
        listener_arn: listener_arn.clone(),
        accelerator_arn: accelerator_arn.clone(),
        protocol: protocol.clone(),
        from_port,
        to_port,
    };

    state.listeners.insert(listener_arn.clone(), listener);

    Ok(json_response(json!({
        "Listener": {
            "ListenerArn": listener_arn,
            "Protocol": protocol,
            "PortRanges": [{"FromPort": from_port, "ToPort": to_port}]
        }
    })))
}

fn list_listeners(
    state: &GlobalAcceleratorState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let accelerator_arn = payload["AcceleratorArn"]
        .as_str()
        .unwrap_or("");

    let listeners: Vec<Value> = state
        .listeners
        .iter()
        .filter(|e| accelerator_arn.is_empty() || e.value().accelerator_arn == accelerator_arn)
        .map(|e| {
            let l = e.value();
            json!({
                "ListenerArn": l.listener_arn,
                "Protocol": l.protocol,
                "PortRanges": [{"FromPort": l.from_port, "ToPort": l.to_port}]
            })
        })
        .collect();

    Ok(json_response(json!({
        "Listeners": listeners
    })))
}

fn create_endpoint_group(
    state: &GlobalAcceleratorState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let listener_arn = payload["ListenerArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ListenerArn".into()))?
        .to_string();

    let endpoint_group_region = payload["EndpointGroupRegion"]
        .as_str()
        .unwrap_or(REGION)
        .to_string();

    let health_check_protocol = payload["HealthCheckProtocol"]
        .as_str()
        .unwrap_or("TCP")
        .to_string();

    let id = uuid::Uuid::new_v4().to_string();
    let eg_arn = format!("{listener_arn}/endpoint-group/{id}");

    let eg = EndpointGroup {
        endpoint_group_arn: eg_arn.clone(),
        listener_arn,
        endpoint_group_region: endpoint_group_region.clone(),
        health_check_protocol: health_check_protocol.clone(),
    };

    state.endpoint_groups.insert(eg_arn.clone(), eg);

    Ok(json_response(json!({
        "EndpointGroup": {
            "EndpointGroupArn": eg_arn,
            "EndpointGroupRegion": endpoint_group_region,
            "HealthCheckProtocol": health_check_protocol
        }
    })))
}
