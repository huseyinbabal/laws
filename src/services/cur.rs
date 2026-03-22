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
pub struct ReportDefinition {
    pub report_name: String,
    pub time_unit: String,
    pub format: String,
    pub compression: String,
    pub s3_bucket: String,
    pub s3_prefix: String,
    pub s3_region: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CurState {
    pub reports: DashMap<String, ReportDefinition>,
}

impl Default for CurState {
    fn default() -> Self {
        Self {
            reports: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &CurState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AWSOrigamiServiceGatewayService.")
        .unwrap_or(target);

    let result = match action {
        "PutReportDefinition" => put_report_definition(state, payload),
        "DeleteReportDefinition" => delete_report_definition(state, payload),
        "DescribeReportDefinitions" => describe_report_definitions(state),
        "ModifyReportDefinition" => modify_report_definition(state, payload),
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

fn report_to_json(r: &ReportDefinition) -> Value {
    json!({
        "ReportName": r.report_name,
        "TimeUnit": r.time_unit,
        "Format": r.format,
        "Compression": r.compression,
        "S3Bucket": r.s3_bucket,
        "S3Prefix": r.s3_prefix,
        "S3Region": r.s3_region,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn put_report_definition(state: &CurState, payload: &Value) -> Result<Response, LawsError> {
    let def = payload
        .get("ReportDefinition")
        .ok_or_else(|| LawsError::InvalidRequest("Missing ReportDefinition".into()))?;

    let report_name = def["ReportName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ReportName".into()))?
        .to_string();

    if state.reports.contains_key(&report_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Report definition already exists: {report_name}"
        )));
    }

    let report = ReportDefinition {
        report_name: report_name.clone(),
        time_unit: def["TimeUnit"].as_str().unwrap_or("DAILY").to_string(),
        format: def["Format"].as_str().unwrap_or("textORcsv").to_string(),
        compression: def["Compression"].as_str().unwrap_or("ZIP").to_string(),
        s3_bucket: def["S3Bucket"].as_str().unwrap_or("").to_string(),
        s3_prefix: def["S3Prefix"].as_str().unwrap_or("").to_string(),
        s3_region: def["S3Region"].as_str().unwrap_or(REGION).to_string(),
    };

    state.reports.insert(report_name, report);
    Ok(json_response(json!({})))
}

fn delete_report_definition(state: &CurState, payload: &Value) -> Result<Response, LawsError> {
    let report_name = payload["ReportName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ReportName".into()))?;

    state.reports.remove(report_name).ok_or_else(|| {
        LawsError::NotFound(format!("Report definition not found: {report_name}"))
    })?;

    Ok(json_response(
        json!({ "ResponseMessage": "Report definition deleted" }),
    ))
}

fn describe_report_definitions(state: &CurState) -> Result<Response, LawsError> {
    let definitions: Vec<Value> = state
        .reports
        .iter()
        .map(|entry| report_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "ReportDefinitions": definitions })))
}

fn modify_report_definition(state: &CurState, payload: &Value) -> Result<Response, LawsError> {
    let report_name = payload["ReportName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ReportName".into()))?;

    let mut report = state.reports.get_mut(report_name).ok_or_else(|| {
        LawsError::NotFound(format!("Report definition not found: {report_name}"))
    })?;

    if let Some(def) = payload.get("ReportDefinition") {
        if let Some(v) = def["TimeUnit"].as_str() {
            report.time_unit = v.to_string();
        }
        if let Some(v) = def["Format"].as_str() {
            report.format = v.to_string();
        }
        if let Some(v) = def["Compression"].as_str() {
            report.compression = v.to_string();
        }
        if let Some(v) = def["S3Bucket"].as_str() {
            report.s3_bucket = v.to_string();
        }
        if let Some(v) = def["S3Prefix"].as_str() {
            report.s3_prefix = v.to_string();
        }
        if let Some(v) = def["S3Region"].as_str() {
            report.s3_region = v.to_string();
        }
    }

    Ok(json_response(json!({})))
}
