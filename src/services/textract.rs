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
pub struct TextractJob {
    pub job_id: String,
    pub status: String,
    pub job_tag: Option<String>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct TextractState {
    pub jobs: DashMap<String, TextractJob>,
}

impl Default for TextractState {
    fn default() -> Self {
        Self {
            jobs: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &TextractState,
    target: &str,
    payload: &serde_json::Value,
) -> Response {
    let action = target.strip_prefix("Textract.").unwrap_or(target);

    let result = match action {
        "DetectDocumentText" => detect_document_text(),
        "AnalyzeDocument" => analyze_document(),
        "StartDocumentTextDetection" => start_document_text_detection(state, payload),
        "GetDocumentTextDetection" => get_document_text_detection(state, payload),
        "StartDocumentAnalysis" => start_document_analysis(state, payload),
        "GetDocumentAnalysis" => get_document_analysis(state, payload),
        other => Err(LawsError::InvalidRequest(format!(
            "unknown action: {other}"
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

fn require_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, LawsError> {
    body.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| LawsError::InvalidRequest(format!("missing required field: {field}")))
}

fn mock_blocks() -> Value {
    let block_id = uuid::Uuid::new_v4().to_string();
    json!([{
        "BlockType": "PAGE",
        "Id": block_id,
        "Text": "",
        "Confidence": 99.9,
        "Geometry": {
            "BoundingBox": {
                "Width": 1.0,
                "Height": 1.0,
                "Left": 0.0,
                "Top": 0.0
            }
        }
    }])
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn detect_document_text() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "Blocks": mock_blocks()
    })))
}

fn analyze_document() -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "Blocks": mock_blocks()
    })))
}

fn start_document_text_detection(
    state: &TextractState,
    body: &Value,
) -> Result<Response, LawsError> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let job_tag = body
        .get("JobTag")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());

    let job = TextractJob {
        job_id: job_id.clone(),
        status: "SUCCEEDED".into(),
        job_tag,
    };

    state.jobs.insert(job_id.clone(), job);

    Ok(json_response(json!({
        "JobId": job_id
    })))
}

fn get_document_text_detection(state: &TextractState, body: &Value) -> Result<Response, LawsError> {
    let job_id = require_str(body, "JobId")?;
    let job = state
        .jobs
        .get(job_id)
        .ok_or_else(|| LawsError::NotFound(format!("job not found: {job_id}")))?;

    Ok(json_response(json!({
        "JobStatus": job.status,
        "Blocks": mock_blocks()
    })))
}

fn start_document_analysis(state: &TextractState, body: &Value) -> Result<Response, LawsError> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let job_tag = body
        .get("JobTag")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());

    let job = TextractJob {
        job_id: job_id.clone(),
        status: "SUCCEEDED".into(),
        job_tag,
    };

    state.jobs.insert(job_id.clone(), job);

    Ok(json_response(json!({
        "JobId": job_id
    })))
}

fn get_document_analysis(state: &TextractState, body: &Value) -> Result<Response, LawsError> {
    let job_id = require_str(body, "JobId")?;
    let job = state
        .jobs
        .get(job_id)
        .ok_or_else(|| LawsError::NotFound(format!("job not found: {job_id}")))?;

    Ok(json_response(json!({
        "JobStatus": job.status,
        "Blocks": mock_blocks()
    })))
}
