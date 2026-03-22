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
pub struct SlackChannelConfiguration {
    pub chat_configuration_arn: String,
    pub slack_team_id: String,
    pub slack_channel_id: String,
    pub slack_channel_name: String,
    pub sns_topic_arns: Vec<String>,
    pub iam_role_arn: String,
    pub logging_level: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct TeamsChannelConfiguration {
    pub chat_configuration_arn: String,
    pub team_id: String,
    pub channel_id: String,
    pub channel_name: String,
    pub team_name: String,
    pub sns_topic_arns: Vec<String>,
    pub iam_role_arn: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ChatbotState {
    pub slack_configs: DashMap<String, SlackChannelConfiguration>,
    pub teams_configs: DashMap<String, TeamsChannelConfiguration>,
}

impl Default for ChatbotState {
    fn default() -> Self {
        Self {
            slack_configs: DashMap::new(),
            teams_configs: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &ChatbotState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("WheatleyOrchestration_20171011.")
        .unwrap_or(target);

    let result = match action {
        "CreateSlackChannelConfiguration" => create_slack_channel_configuration(state, payload),
        "DeleteSlackChannelConfiguration" => delete_slack_channel_configuration(state, payload),
        "DescribeSlackChannelConfigurations" => describe_slack_channel_configurations(state),
        "ListMicrosoftTeamsChannelConfigurations" => {
            list_microsoft_teams_channel_configurations(state)
        }
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

fn create_slack_channel_configuration(
    state: &ChatbotState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let slack_team_id = payload["SlackTeamId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing SlackTeamId".into()))?
        .to_string();
    let slack_channel_id = payload["SlackChannelId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing SlackChannelId".into()))?
        .to_string();
    let slack_channel_name = payload["SlackChannelName"]
        .as_str()
        .unwrap_or("general")
        .to_string();
    let iam_role_arn = payload["IamRoleArn"]
        .as_str()
        .unwrap_or(&format!(
            "arn:aws:iam::{ACCOUNT_ID}:role/chatbot-role"
        ))
        .to_string();
    let sns_topic_arns: Vec<String> = payload["SnsTopicArns"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let logging_level = payload["LoggingLevel"]
        .as_str()
        .unwrap_or("ERROR")
        .to_string();

    let config_id = uuid::Uuid::new_v4().to_string();
    let chat_configuration_arn = format!(
        "arn:aws:chatbot::{ACCOUNT_ID}:chat-configuration/slack-channel/{config_id}"
    );
    let now = Utc::now().to_rfc3339();

    let config = SlackChannelConfiguration {
        chat_configuration_arn: chat_configuration_arn.clone(),
        slack_team_id,
        slack_channel_id,
        slack_channel_name,
        sns_topic_arns,
        iam_role_arn,
        logging_level,
        created_at: now,
    };

    let resp = slack_config_to_json(&config);
    state.slack_configs.insert(config_id, config);

    Ok(json_response(
        StatusCode::OK,
        json!({ "ChannelConfiguration": resp }),
    ))
}

fn delete_slack_channel_configuration(
    state: &ChatbotState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let chat_configuration_arn = payload["ChatConfigurationArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ChatConfigurationArn".into()))?;

    let mut found = false;
    state.slack_configs.retain(|_, v| {
        if v.chat_configuration_arn == chat_configuration_arn {
            found = true;
            false
        } else {
            true
        }
    });

    if !found {
        return Err(LawsError::NotFound(format!(
            "SlackChannelConfiguration not found: {chat_configuration_arn}"
        )));
    }

    Ok(json_response(StatusCode::OK, json!({})))
}

fn describe_slack_channel_configurations(
    state: &ChatbotState,
) -> Result<Response, LawsError> {
    let configs: Vec<Value> = state
        .slack_configs
        .iter()
        .map(|entry| slack_config_to_json(entry.value()))
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "SlackChannelConfigurations": configs }),
    ))
}

fn list_microsoft_teams_channel_configurations(
    state: &ChatbotState,
) -> Result<Response, LawsError> {
    let configs: Vec<Value> = state
        .teams_configs
        .iter()
        .map(|entry| {
            let c = entry.value();
            json!({
                "ChatConfigurationArn": c.chat_configuration_arn,
                "TeamId": c.team_id,
                "ChannelId": c.channel_id,
                "ChannelName": c.channel_name,
                "TeamName": c.team_name,
                "SnsTopicArns": c.sns_topic_arns,
                "IamRoleArn": c.iam_role_arn,
            })
        })
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "TeamChannelConfigurations": configs }),
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn slack_config_to_json(c: &SlackChannelConfiguration) -> Value {
    json!({
        "ChatConfigurationArn": c.chat_configuration_arn,
        "SlackTeamId": c.slack_team_id,
        "SlackChannelId": c.slack_channel_id,
        "SlackChannelName": c.slack_channel_name,
        "SnsTopicArns": c.sns_topic_arns,
        "IamRoleArn": c.iam_role_arn,
        "LoggingLevel": c.logging_level,
    })
}
