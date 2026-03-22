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
pub struct Detector {
    pub detector_id: String,
    pub detector_version_id: String,
    pub description: String,
    pub status: String,
    pub arn: String,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub model_id: String,
    pub model_type: String,
    pub description: String,
    pub arn: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct FraudDetectorState {
    pub detectors: DashMap<String, Detector>,
    pub models: DashMap<String, Model>,
}

impl Default for FraudDetectorState {
    fn default() -> Self {
        Self {
            detectors: DashMap::new(),
            models: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &FraudDetectorState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AWSHawksNestServiceFacade.")
        .unwrap_or(target);

    let result = match action {
        "CreateDetectorVersion" => create_detector_version(state, payload),
        "GetDetectors" => get_detectors(state, payload),
        "CreateModel" => create_model(state, payload),
        "GetModels" => get_models(state, payload),
        "CreateRule" => create_rule(state, payload),
        "GetRules" => get_rules(state, payload),
        "GetEventPrediction" => get_event_prediction(state, payload),
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

fn detector_to_json(d: &Detector) -> Value {
    json!({
        "detectorId": d.detector_id,
        "detectorVersionId": d.detector_version_id,
        "description": d.description,
        "status": d.status,
        "arn": d.arn,
    })
}

fn model_to_json(m: &Model) -> Value {
    json!({
        "modelId": m.model_id,
        "modelType": m.model_type,
        "description": m.description,
        "arn": m.arn,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_detector_version(
    state: &FraudDetectorState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let detector_id = payload["detectorId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing detectorId".into()))?
        .to_string();

    let description = payload["description"].as_str().unwrap_or("").to_string();
    let version_id = "1".to_string();
    let arn = format!("arn:aws:frauddetector:{REGION}:{ACCOUNT_ID}:detector/{detector_id}");

    let detector = Detector {
        detector_id: detector_id.clone(),
        detector_version_id: version_id.clone(),
        description,
        status: "DRAFT".to_string(),
        arn,
    };

    state.detectors.insert(detector_id.clone(), detector);
    Ok(json_response(json!({
        "detectorId": detector_id,
        "detectorVersionId": version_id,
        "status": "DRAFT",
    })))
}

fn get_detectors(state: &FraudDetectorState, payload: &Value) -> Result<Response, LawsError> {
    let detectors: Vec<Value> = if let Some(id) = payload["detectorId"].as_str() {
        state
            .detectors
            .get(id)
            .map(|d| vec![detector_to_json(d.value())])
            .unwrap_or_default()
    } else {
        state
            .detectors
            .iter()
            .map(|entry| detector_to_json(entry.value()))
            .collect()
    };

    Ok(json_response(json!({ "detectors": detectors })))
}

fn create_model(state: &FraudDetectorState, payload: &Value) -> Result<Response, LawsError> {
    let model_id = payload["modelId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing modelId".into()))?
        .to_string();

    let model_type = payload["modelType"]
        .as_str()
        .unwrap_or("ONLINE_FRAUD_INSIGHTS")
        .to_string();

    let description = payload["description"].as_str().unwrap_or("").to_string();
    let arn = format!("arn:aws:frauddetector:{REGION}:{ACCOUNT_ID}:model/{model_id}");

    let model = Model {
        model_id: model_id.clone(),
        model_type,
        description,
        arn,
    };

    state.models.insert(model_id, model);
    Ok(json_response(json!({})))
}

fn get_models(state: &FraudDetectorState, payload: &Value) -> Result<Response, LawsError> {
    let models: Vec<Value> = if let Some(id) = payload["modelId"].as_str() {
        state
            .models
            .get(id)
            .map(|m| vec![model_to_json(m.value())])
            .unwrap_or_default()
    } else {
        state
            .models
            .iter()
            .map(|entry| model_to_json(entry.value()))
            .collect()
    };

    Ok(json_response(json!({ "models": models })))
}

fn create_rule(_state: &FraudDetectorState, payload: &Value) -> Result<Response, LawsError> {
    let rule_id = payload["ruleId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ruleId".into()))?;

    let detector_id = payload["detectorId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing detectorId".into()))?;

    Ok(json_response(json!({
        "rule": {
            "ruleId": rule_id,
            "detectorId": detector_id,
            "ruleVersion": "1",
        }
    })))
}

fn get_rules(_state: &FraudDetectorState, payload: &Value) -> Result<Response, LawsError> {
    let _detector_id = payload["detectorId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing detectorId".into()))?;

    // Rules are returned as empty list since we don't persist them separately
    Ok(json_response(json!({ "ruleDetails": [] })))
}

fn get_event_prediction(
    _state: &FraudDetectorState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let _detector_id = payload["detectorId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing detectorId".into()))?;

    let _event_id = payload["eventId"].as_str().unwrap_or("unknown");

    Ok(json_response(json!({
        "modelScores": [],
        "ruleResults": [],
        "externalModelOutputs": [],
    })))
}
