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
pub struct LandingZone {
    pub arn: String,
    pub identifier: String,
    pub version: String,
    pub status: String,
    pub manifest: Value,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct EnabledControl {
    pub arn: String,
    pub control_identifier: String,
    pub target_identifier: String,
    pub status: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ControlTowerState {
    pub landing_zones: DashMap<String, LandingZone>,
    pub enabled_controls: DashMap<String, EnabledControl>,
}

impl Default for ControlTowerState {
    fn default() -> Self {
        Self {
            landing_zones: DashMap::new(),
            enabled_controls: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &ControlTowerState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("ControltowerService.")
        .unwrap_or(target);

    let result = match action {
        "CreateLandingZone" => create_landing_zone(state, payload),
        "GetLandingZone" => get_landing_zone(state, payload),
        "ListLandingZones" => list_landing_zones(state),
        "EnableControl" => enable_control(state, payload),
        "DisableControl" => disable_control(state, payload),
        "ListEnabledControls" => list_enabled_controls(state, payload),
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

fn landing_zone_to_json(lz: &LandingZone) -> Value {
    json!({
        "arn": lz.arn,
        "identifier": lz.identifier,
        "version": lz.version,
        "status": lz.status,
        "manifest": lz.manifest,
        "createdAt": lz.created_at,
    })
}

fn enabled_control_to_json(ec: &EnabledControl) -> Value {
    json!({
        "arn": ec.arn,
        "controlIdentifier": ec.control_identifier,
        "targetIdentifier": ec.target_identifier,
        "statusSummary": { "status": ec.status },
        "createdAt": ec.created_at,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_landing_zone(
    state: &ControlTowerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let version = payload["version"]
        .as_str()
        .unwrap_or("3.3")
        .to_string();

    let manifest = payload["manifest"].clone();

    let identifier = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:controltower:{REGION}:{ACCOUNT_ID}:landingzone/{identifier}"
    );
    let now = chrono::Utc::now().to_rfc3339();

    let lz = LandingZone {
        arn: arn.clone(),
        identifier: identifier.clone(),
        version,
        status: "ACTIVE".to_string(),
        manifest,
        created_at: now,
    };

    state.landing_zones.insert(identifier.clone(), lz);

    let operation_id = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "arn": arn,
        "operationIdentifier": operation_id,
    })))
}

fn get_landing_zone(
    state: &ControlTowerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let identifier = payload["landingZoneIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("landingZoneIdentifier is required".to_string())
        })?;

    let lz = state
        .landing_zones
        .get(identifier)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Landing zone '{}' not found", identifier))
        })?;

    Ok(json_response(json!({
        "landingZone": landing_zone_to_json(lz.value()),
    })))
}

fn list_landing_zones(state: &ControlTowerState) -> Result<Response, LawsError> {
    let zones: Vec<Value> = state
        .landing_zones
        .iter()
        .map(|entry| {
            let lz = entry.value();
            json!({
                "arn": lz.arn,
                "identifier": lz.identifier,
            })
        })
        .collect();

    Ok(json_response(json!({ "landingZones": zones })))
}

fn enable_control(
    state: &ControlTowerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let control_identifier = payload["controlIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("controlIdentifier is required".to_string())
        })?
        .to_string();

    let target_identifier = payload["targetIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("targetIdentifier is required".to_string())
        })?
        .to_string();

    let key = format!("{}:{}", control_identifier, target_identifier);

    if state.enabled_controls.contains_key(&key) {
        return Err(LawsError::AlreadyExists(format!(
            "Control '{}' is already enabled on target '{}'",
            control_identifier, target_identifier
        )));
    }

    let ec_id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:controltower:{REGION}:{ACCOUNT_ID}:enabledcontrol/{ec_id}"
    );
    let now = chrono::Utc::now().to_rfc3339();

    let ec = EnabledControl {
        arn: arn.clone(),
        control_identifier,
        target_identifier,
        status: "SUCCEEDED".to_string(),
        created_at: now,
    };

    state.enabled_controls.insert(key, ec);

    let operation_id = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "arn": arn,
        "operationIdentifier": operation_id,
    })))
}

fn disable_control(
    state: &ControlTowerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let control_identifier = payload["controlIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("controlIdentifier is required".to_string())
        })?;

    let target_identifier = payload["targetIdentifier"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("targetIdentifier is required".to_string())
        })?;

    let key = format!("{}:{}", control_identifier, target_identifier);

    state
        .enabled_controls
        .remove(&key)
        .ok_or_else(|| {
            LawsError::NotFound(format!(
                "Enabled control not found for '{}' on '{}'",
                control_identifier, target_identifier
            ))
        })?;

    let operation_id = uuid::Uuid::new_v4().to_string();

    Ok(json_response(json!({
        "operationIdentifier": operation_id,
    })))
}

fn list_enabled_controls(
    state: &ControlTowerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let target_identifier = payload["targetIdentifier"]
        .as_str();

    let controls: Vec<Value> = state
        .enabled_controls
        .iter()
        .filter(|entry| {
            if let Some(target) = target_identifier {
                entry.value().target_identifier == target
            } else {
                true
            }
        })
        .map(|entry| enabled_control_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "enabledControls": controls })))
}
