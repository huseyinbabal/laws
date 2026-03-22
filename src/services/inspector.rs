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
pub struct AssessmentTemplate {
    pub arn: String,
    pub name: String,
    pub assessment_target_arn: String,
    pub duration_in_seconds: u32,
    pub rules_package_arns: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct InspectorFinding {
    pub arn: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub assessment_run_arn: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct InspectorState {
    pub assessment_templates: DashMap<String, AssessmentTemplate>,
    pub findings: DashMap<String, InspectorFinding>,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            assessment_templates: DashMap::new(),
            findings: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &InspectorState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("InspectorService.")
        .unwrap_or(target);

    let result = match action {
        "CreateAssessmentTemplate" => create_assessment_template(state, payload),
        "DeleteAssessmentTemplate" => delete_assessment_template(state, payload),
        "ListAssessmentTemplates" => list_assessment_templates(state),
        "ListFindings" => list_findings(state),
        "DescribeFindings" => describe_findings(state, payload),
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
    (
        status,
        [("Content-Type", "application/x-amz-json-1.1")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

fn template_to_json(t: &AssessmentTemplate) -> Value {
    json!({
        "arn": t.arn,
        "name": t.name,
        "assessmentTargetArn": t.assessment_target_arn,
        "durationInSeconds": t.duration_in_seconds,
        "rulesPackageArns": t.rules_package_arns,
        "createdAt": t.created_at,
    })
}

fn finding_to_json(f: &InspectorFinding) -> Value {
    json!({
        "arn": f.arn,
        "title": f.title,
        "description": f.description,
        "severity": f.severity,
        "assessmentRunArn": f.assessment_run_arn,
        "createdAt": f.created_at,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_assessment_template(
    state: &InspectorState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let name = payload["assessmentTemplateName"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("assessmentTemplateName is required".to_string())
        })?
        .to_string();

    let target_arn = payload["assessmentTargetArn"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let duration = payload["durationInSeconds"]
        .as_u64()
        .unwrap_or(3600) as u32;

    let rules_arns: Vec<String> = payload["rulesPackageArns"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let template_id = uuid::Uuid::new_v4().to_string();
    let arn = format!(
        "arn:aws:inspector:{REGION}:{ACCOUNT_ID}:target/0-abc123/template/{template_id}"
    );

    let template = AssessmentTemplate {
        arn: arn.clone(),
        name,
        assessment_target_arn: target_arn,
        duration_in_seconds: duration,
        rules_package_arns: rules_arns,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    state.assessment_templates.insert(arn.clone(), template);

    Ok(json_response(
        StatusCode::OK,
        json!({ "assessmentTemplateArn": arn }),
    ))
}

fn delete_assessment_template(
    state: &InspectorState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let arn = payload["assessmentTemplateArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("assessmentTemplateArn is required".to_string())
        })?;

    state
        .assessment_templates
        .remove(arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Assessment template not found: {}", arn))
        })?;

    Ok(json_response(StatusCode::OK, json!({})))
}

fn list_assessment_templates(state: &InspectorState) -> Result<Response, LawsError> {
    let arns: Vec<String> = state
        .assessment_templates
        .iter()
        .map(|entry| entry.value().arn.clone())
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "assessmentTemplateArns": arns }),
    ))
}

fn list_findings(state: &InspectorState) -> Result<Response, LawsError> {
    let arns: Vec<String> = state
        .findings
        .iter()
        .map(|entry| entry.value().arn.clone())
        .collect();

    Ok(json_response(
        StatusCode::OK,
        json!({ "findingArns": arns }),
    ))
}

fn describe_findings(
    state: &InspectorState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let finding_arns = payload["findingArns"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut found = Vec::new();
    let mut failed = Vec::new();

    for arn_val in &finding_arns {
        let arn = arn_val.as_str().unwrap_or_default();
        match state.findings.get(arn) {
            Some(f) => found.push(finding_to_json(f.value())),
            None => failed.push(json!({
                "arn": arn,
                "failureCode": "ITEM_DOES_NOT_EXIST",
            })),
        }
    }

    Ok(json_response(StatusCode::OK, json!({
        "findings": found,
        "failedItems": failed,
    })))
}
