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
pub struct ConfigRule {
    pub config_rule_name: String,
    pub config_rule_id: String,
    pub arn: String,
    pub source: Value,
    pub state: String,
    pub input_parameters: String,
}

#[derive(Debug, Clone)]
pub struct ConfigurationRecorder {
    pub name: String,
    pub role_arn: String,
    pub recording: bool,
}

#[derive(Debug, Clone)]
pub struct DeliveryChannel {
    pub name: String,
    pub s3_bucket_name: String,
    pub sns_topic_arn: Option<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ConfigServiceState {
    pub config_rules: DashMap<String, ConfigRule>,
    pub configuration_recorders: DashMap<String, ConfigurationRecorder>,
    pub delivery_channels: DashMap<String, DeliveryChannel>,
}

impl Default for ConfigServiceState {
    fn default() -> Self {
        Self {
            config_rules: DashMap::new(),
            configuration_recorders: DashMap::new(),
            delivery_channels: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &ConfigServiceState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("StarlingDoveService.")
        .unwrap_or(target);

    let result = match action {
        "PutConfigRule" => put_config_rule(state, payload),
        "DeleteConfigRule" => delete_config_rule(state, payload),
        "DescribeConfigRules" => describe_config_rules(state),
        "PutConfigurationRecorder" => put_configuration_recorder(state, payload),
        "DescribeConfigurationRecorders" => describe_configuration_recorders(state),
        "StartConfigurationRecorder" => start_configuration_recorder(state, payload),
        "StopConfigurationRecorder" => stop_configuration_recorder(state, payload),
        "PutDeliveryChannel" => put_delivery_channel(state, payload),
        "DescribeDeliveryChannels" => describe_delivery_channels(state),
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

fn config_rule_arn(name: &str) -> String {
    format!("arn:aws:config:{REGION}:{ACCOUNT_ID}:config-rule/{name}")
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn put_config_rule(state: &ConfigServiceState, payload: &Value) -> Result<Response, LawsError> {
    let rule = payload
        .get("ConfigRule")
        .ok_or_else(|| LawsError::InvalidRequest("Missing ConfigRule".into()))?;

    let name = rule["ConfigRuleName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ConfigRuleName".into()))?
        .to_string();

    let source = rule.get("Source").cloned().unwrap_or(json!({}));
    let input_parameters = rule["InputParameters"].as_str().unwrap_or("").to_string();

    let config_rule_id = uuid::Uuid::new_v4().to_string();
    let arn = config_rule_arn(&name);

    let cr = ConfigRule {
        config_rule_name: name.clone(),
        config_rule_id,
        arn,
        source,
        state: "ACTIVE".to_string(),
        input_parameters,
    };

    state.config_rules.insert(name, cr);

    Ok(json_response(json!({})))
}

fn delete_config_rule(state: &ConfigServiceState, payload: &Value) -> Result<Response, LawsError> {
    let name = payload["ConfigRuleName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ConfigRuleName".into()))?;

    state
        .config_rules
        .remove(name)
        .ok_or_else(|| LawsError::NotFound(format!("Config rule '{}' not found", name)))?;

    Ok(json_response(json!({})))
}

fn describe_config_rules(state: &ConfigServiceState) -> Result<Response, LawsError> {
    let rules: Vec<Value> = state
        .config_rules
        .iter()
        .map(|entry| {
            let r = entry.value();
            json!({
                "ConfigRuleName": r.config_rule_name,
                "ConfigRuleId": r.config_rule_id,
                "ConfigRuleArn": r.arn,
                "Source": r.source,
                "ConfigRuleState": r.state,
                "InputParameters": r.input_parameters,
            })
        })
        .collect();

    Ok(json_response(json!({ "ConfigRules": rules })))
}

fn put_configuration_recorder(
    state: &ConfigServiceState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let recorder = payload
        .get("ConfigurationRecorder")
        .ok_or_else(|| LawsError::InvalidRequest("Missing ConfigurationRecorder".into()))?;

    let name = recorder["name"].as_str().unwrap_or("default").to_string();

    let role_arn = recorder["roleARN"].as_str().unwrap_or("").to_string();

    let cr = ConfigurationRecorder {
        name: name.clone(),
        role_arn,
        recording: false,
    };

    state.configuration_recorders.insert(name, cr);

    Ok(json_response(json!({})))
}

fn describe_configuration_recorders(state: &ConfigServiceState) -> Result<Response, LawsError> {
    let recorders: Vec<Value> = state
        .configuration_recorders
        .iter()
        .map(|entry| {
            let r = entry.value();
            json!({
                "name": r.name,
                "roleARN": r.role_arn,
                "recording": r.recording,
            })
        })
        .collect();

    Ok(json_response(json!({
        "ConfigurationRecorders": recorders
    })))
}

fn start_configuration_recorder(
    state: &ConfigServiceState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["ConfigurationRecorderName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ConfigurationRecorderName".into()))?;

    let mut recorder = state.configuration_recorders.get_mut(name).ok_or_else(|| {
        LawsError::NotFound(format!("Configuration recorder '{}' not found", name))
    })?;

    recorder.recording = true;

    Ok(json_response(json!({})))
}

fn stop_configuration_recorder(
    state: &ConfigServiceState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["ConfigurationRecorderName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ConfigurationRecorderName".into()))?;

    let mut recorder = state.configuration_recorders.get_mut(name).ok_or_else(|| {
        LawsError::NotFound(format!("Configuration recorder '{}' not found", name))
    })?;

    recorder.recording = false;

    Ok(json_response(json!({})))
}

fn put_delivery_channel(
    state: &ConfigServiceState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let channel = payload
        .get("DeliveryChannel")
        .ok_or_else(|| LawsError::InvalidRequest("Missing DeliveryChannel".into()))?;

    let name = channel["name"].as_str().unwrap_or("default").to_string();

    let s3_bucket_name = channel["s3BucketName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing s3BucketName".into()))?
        .to_string();

    let sns_topic_arn = channel["snsTopicARN"].as_str().map(|s| s.to_string());

    let dc = DeliveryChannel {
        name: name.clone(),
        s3_bucket_name,
        sns_topic_arn,
    };

    state.delivery_channels.insert(name, dc);

    Ok(json_response(json!({})))
}

fn describe_delivery_channels(state: &ConfigServiceState) -> Result<Response, LawsError> {
    let channels: Vec<Value> = state
        .delivery_channels
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "name": d.name,
                "s3BucketName": d.s3_bucket_name,
                "snsTopicARN": d.sns_topic_arn,
            })
        })
        .collect();

    Ok(json_response(json!({
        "DeliveryChannels": channels
    })))
}
