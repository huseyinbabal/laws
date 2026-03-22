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
pub struct ComprehendJob {
    pub job_id: String,
    pub job_name: String,
    pub status: String,
    pub input_data_config: Value,
    pub output_data_config: Value,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ComprehendState {
    pub jobs: DashMap<String, ComprehendJob>,
}

impl Default for ComprehendState {
    fn default() -> Self {
        Self {
            jobs: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &ComprehendState, target: &str, payload: &serde_json::Value) -> Response {
    let action = target
        .strip_prefix("Comprehend_20171127.")
        .unwrap_or(target);

    let result = match action {
        "DetectSentiment" => detect_sentiment(),
        "DetectEntities" => detect_entities(),
        "DetectKeyPhrases" => detect_key_phrases(),
        "DetectDominantLanguage" => detect_dominant_language(),
        "BatchDetectSentiment" => batch_detect_sentiment(payload),
        "StartEntitiesDetectionJob" => start_entities_detection_job(state, payload),
        "ListEntitiesDetectionJobs" => list_entities_detection_jobs(state),
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

fn detect_sentiment() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "Sentiment": "NEUTRAL",
        "SentimentScore": {
            "Positive": 0.1,
            "Negative": 0.1,
            "Neutral": 0.7,
            "Mixed": 0.1
        }
    })))
}

fn detect_entities() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "Entities": []
    })))
}

fn detect_key_phrases() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "KeyPhrases": []
    })))
}

fn detect_dominant_language() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "Languages": [
            {"LanguageCode": "en", "Score": 0.99}
        ]
    })))
}

fn batch_detect_sentiment(body: &Value) -> Result<Response, LawsError> {
    let text_list = body
        .get("TextList")
        .and_then(|v| v.as_array())
        .ok_or_else(|| LawsError::InvalidRequest("missing TextList".into()))?;

    let results: Vec<Value> = text_list
        .iter()
        .enumerate()
        .map(|(i, _)| {
            json!({
                "Index": i,
                "Sentiment": "NEUTRAL",
                "SentimentScore": {
                    "Positive": 0.1,
                    "Negative": 0.1,
                    "Neutral": 0.7,
                    "Mixed": 0.1
                }
            })
        })
        .collect();

    Ok(json_response(json!({
        "ResultList": results,
        "ErrorList": []
    })))
}

fn start_entities_detection_job(state: &ComprehendState, body: &Value) -> Result<Response, LawsError> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let job_name = body
        .get("JobName")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_owned();
    let input_data_config = body
        .get("InputDataConfig")
        .cloned()
        .unwrap_or(json!({}));
    let output_data_config = body
        .get("OutputDataConfig")
        .cloned()
        .unwrap_or(json!({}));

    let job = ComprehendJob {
        job_id: job_id.clone(),
        job_name,
        status: "COMPLETED".into(),
        input_data_config,
        output_data_config,
    };

    state.jobs.insert(job_id.clone(), job);

    Ok(json_response(json!({
        "JobId": job_id,
        "JobStatus": "SUBMITTED"
    })))
}

fn list_entities_detection_jobs(state: &ComprehendState) -> Result<Response, LawsError> {
    let job_list: Vec<Value> = state
        .jobs
        .iter()
        .map(|entry| {
            let j = entry.value();
            json!({
                "JobId": j.job_id,
                "JobName": j.job_name,
                "JobStatus": j.status,
                "InputDataConfig": j.input_data_config,
                "OutputDataConfig": j.output_data_config
            })
        })
        .collect();

    Ok(json_response(json!({
        "EntitiesDetectionJobPropertiesList": job_list
    })))
}
