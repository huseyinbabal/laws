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
pub struct Gateway {
    pub gateway_arn: String,
    pub gateway_id: String,
    pub gateway_name: String,
    pub gateway_type: String,
    pub gateway_state: String,
}

#[derive(Debug, Clone)]
pub struct Volume {
    pub volume_arn: String,
    pub volume_id: String,
    pub gateway_arn: String,
    pub volume_type: String,
    pub volume_status: String,
    pub volume_size_in_bytes: u64,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct StorageGatewayState {
    pub gateways: DashMap<String, Gateway>,
    pub volumes: DashMap<String, Volume>,
}

impl Default for StorageGatewayState {
    fn default() -> Self {
        Self {
            gateways: DashMap::new(),
            volumes: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &StorageGatewayState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("StorageGateway_20130630.")
        .unwrap_or(target);

    let result = match action {
        "ActivateGateway" => activate_gateway(state, payload),
        "ListGateways" => list_gateways(state),
        "DescribeGatewayInformation" => describe_gateway_information(state, payload),
        "DeleteGateway" => delete_gateway(state, payload),
        "CreateStorediSCSIVolume" => create_stored_iscsi_volume(state, payload),
        "ListVolumes" => list_volumes(state, payload),
        "DescribeStorediSCSIVolumes" => describe_stored_iscsi_volumes(state, payload),
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
// Operations
// ---------------------------------------------------------------------------

fn activate_gateway(state: &StorageGatewayState, body: &Value) -> Result<Response, LawsError> {
    let gateway_name = require_str(body, "GatewayName")?.to_owned();
    let gateway_type = body.get("GatewayType").and_then(|v| v.as_str()).unwrap_or("STORED").to_owned();
    let gateway_id = format!("sgw-{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
    let gateway_arn = format!("arn:aws:storagegateway:{REGION}:{ACCOUNT_ID}:gateway/{gateway_id}");

    let gateway = Gateway {
        gateway_arn: gateway_arn.clone(),
        gateway_id: gateway_id.clone(),
        gateway_name,
        gateway_type,
        gateway_state: "RUNNING".into(),
    };

    state.gateways.insert(gateway_arn.clone(), gateway);

    Ok(json_response(json!({
        "GatewayARN": gateway_arn
    })))
}

fn list_gateways(state: &StorageGatewayState) -> Result<Response, LawsError> {
    let gateways: Vec<Value> = state.gateways.iter().map(|entry| {
        let g = entry.value();
        json!({
            "GatewayARN": g.gateway_arn,
            "GatewayId": g.gateway_id,
            "GatewayName": g.gateway_name,
            "GatewayType": g.gateway_type,
            "GatewayOperationalState": g.gateway_state
        })
    }).collect();

    Ok(json_response(json!({
        "Gateways": gateways
    })))
}

fn describe_gateway_information(state: &StorageGatewayState, body: &Value) -> Result<Response, LawsError> {
    let gateway_arn = require_str(body, "GatewayARN")?;

    let g = state.gateways.get(gateway_arn)
        .ok_or_else(|| LawsError::NotFound(format!("gateway not found: {gateway_arn}")))?;

    Ok(json_response(json!({
        "GatewayARN": g.gateway_arn,
        "GatewayId": g.gateway_id,
        "GatewayName": g.gateway_name,
        "GatewayType": g.gateway_type,
        "GatewayState": g.gateway_state,
        "GatewayNetworkInterfaces": []
    })))
}

fn delete_gateway(state: &StorageGatewayState, body: &Value) -> Result<Response, LawsError> {
    let gateway_arn = require_str(body, "GatewayARN")?;
    state.gateways.remove(gateway_arn)
        .ok_or_else(|| LawsError::NotFound(format!("gateway not found: {gateway_arn}")))?;

    state.volumes.retain(|_, v| v.gateway_arn != gateway_arn);

    Ok(json_response(json!({
        "GatewayARN": gateway_arn
    })))
}

fn create_stored_iscsi_volume(state: &StorageGatewayState, body: &Value) -> Result<Response, LawsError> {
    let gateway_arn = require_str(body, "GatewayARN")?.to_owned();
    let disk_id = body.get("DiskId").and_then(|v| v.as_str()).unwrap_or("disk-0").to_owned();
    let preserve = body.get("PreserveExistingData").and_then(|v| v.as_bool()).unwrap_or(false);
    let target_name = require_str(body, "TargetName")?.to_owned();
    let network_interface = require_str(body, "NetworkInterfaceId")?.to_owned();

    let volume_id = format!("vol-{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
    let volume_arn = format!("arn:aws:storagegateway:{REGION}:{ACCOUNT_ID}:gateway/{}/volume/{volume_id}", gateway_arn.rsplit('/').next().unwrap_or(""));
    let volume_size = body.get("SnapshotId").map(|_| 107374182400u64).unwrap_or(107374182400);

    let volume = Volume {
        volume_arn: volume_arn.clone(),
        volume_id: volume_id.clone(),
        gateway_arn,
        volume_type: "STORED".into(),
        volume_status: "AVAILABLE".into(),
        volume_size_in_bytes: volume_size,
    };

    state.volumes.insert(volume_arn.clone(), volume);

    Ok(json_response(json!({
        "VolumeARN": volume_arn,
        "VolumeSizeInBytes": volume_size,
        "TargetARN": format!("arn:aws:storagegateway:{REGION}:{ACCOUNT_ID}:gateway/target/{target_name}")
    })))
}

fn list_volumes(state: &StorageGatewayState, body: &Value) -> Result<Response, LawsError> {
    let gateway_arn = body.get("GatewayARN").and_then(|v| v.as_str());

    let volumes: Vec<Value> = state.volumes.iter()
        .filter(|entry| {
            gateway_arn.map_or(true, |arn| entry.value().gateway_arn == arn)
        })
        .map(|entry| {
            let v = entry.value();
            json!({
                "VolumeARN": v.volume_arn,
                "VolumeId": v.volume_id,
                "GatewayARN": v.gateway_arn,
                "VolumeType": v.volume_type,
                "VolumeSizeInBytes": v.volume_size_in_bytes
            })
        })
        .collect();

    Ok(json_response(json!({
        "GatewayARN": gateway_arn.unwrap_or(""),
        "VolumeInfos": volumes
    })))
}

fn describe_stored_iscsi_volumes(state: &StorageGatewayState, body: &Value) -> Result<Response, LawsError> {
    let volume_arns = body.get("VolumeARNs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| LawsError::InvalidRequest("missing required field: VolumeARNs".into()))?;

    let volumes: Vec<Value> = volume_arns.iter()
        .filter_map(|arn| {
            let arn_str = arn.as_str()?;
            let v = state.volumes.get(arn_str)?;
            Some(json!({
                "VolumeARN": v.volume_arn,
                "VolumeId": v.volume_id,
                "VolumeType": v.volume_type,
                "VolumeStatus": v.volume_status,
                "VolumeSizeInBytes": v.volume_size_in_bytes,
                "PreservedExistingData": false
            }))
        })
        .collect();

    Ok(json_response(json!({
        "StorediSCSIVolumes": volumes
    })))
}
