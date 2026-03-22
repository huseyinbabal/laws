use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#[allow(dead_code)]
const ACCOUNT_ID: &str = "000000000000";
#[allow(dead_code)]
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SupportCase {
    pub case_id: String,
    pub display_id: String,
    pub subject: String,
    pub status: String,
    pub service_code: String,
    pub category_code: String,
    pub severity_code: String,
    pub communication_body: String,
    pub language: String,
    pub submitted_by: String,
    pub time_created: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct SupportState {
    pub cases: DashMap<String, SupportCase>,
}

impl Default for SupportState {
    fn default() -> Self {
        Self {
            cases: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &SupportState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AWSSupport_20130415.")
        .unwrap_or(target);

    let result = match action {
        "CreateCase" => create_case(state, payload),
        "DescribeCases" => describe_cases(state),
        "ResolveCase" => resolve_case(state, payload),
        "DescribeServices" => describe_services(),
        "DescribeTrustedAdvisorChecks" => describe_trusted_advisor_checks(),
        "DescribeTrustedAdvisorCheckResult" => describe_trusted_advisor_check_result(payload),
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

fn create_case(state: &SupportState, payload: &Value) -> Result<Response, LawsError> {
    let subject = payload["subject"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing subject".into()))?
        .to_string();

    let service_code = payload["serviceCode"]
        .as_str()
        .unwrap_or("general-info")
        .to_string();

    let category_code = payload["categoryCode"]
        .as_str()
        .unwrap_or("other")
        .to_string();

    let severity_code = payload["severityCode"]
        .as_str()
        .unwrap_or("low")
        .to_string();

    let communication_body = payload["communicationBody"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let language = payload["language"].as_str().unwrap_or("en").to_string();

    let case_id = format!("case-{}", &uuid::Uuid::new_v4().to_string()[..12]);
    let display_id = format!("{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let time_created = chrono::Utc::now().to_rfc3339();

    let case = SupportCase {
        case_id: case_id.clone(),
        display_id,
        subject,
        status: "opened".to_string(),
        service_code,
        category_code,
        severity_code,
        communication_body,
        language,
        submitted_by: "user@example.com".to_string(),
        time_created,
    };

    state.cases.insert(case_id.clone(), case);

    Ok(json_response(json!({
        "caseId": case_id,
    })))
}

fn describe_cases(state: &SupportState) -> Result<Response, LawsError> {
    let cases: Vec<Value> = state
        .cases
        .iter()
        .map(|entry| {
            let c = entry.value();
            json!({
                "caseId": c.case_id,
                "displayId": c.display_id,
                "subject": c.subject,
                "status": c.status,
                "serviceCode": c.service_code,
                "categoryCode": c.category_code,
                "severityCode": c.severity_code,
                "submittedBy": c.submitted_by,
                "timeCreated": c.time_created,
                "language": c.language,
                "recentCommunications": {
                    "communications": [
                        {
                            "body": c.communication_body,
                            "submittedBy": c.submitted_by,
                            "timeCreated": c.time_created,
                        }
                    ],
                },
            })
        })
        .collect();

    Ok(json_response(json!({
        "cases": cases,
    })))
}

fn resolve_case(state: &SupportState, payload: &Value) -> Result<Response, LawsError> {
    let case_id = payload["caseId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing caseId".into()))?;

    let mut case = state
        .cases
        .get_mut(case_id)
        .ok_or_else(|| LawsError::NotFound(format!("Case '{}' not found", case_id)))?;

    case.status = "resolved".to_string();

    Ok(json_response(json!({
        "initialCaseStatus": "opened",
        "finalCaseStatus": "resolved",
    })))
}

fn describe_services() -> Result<Response, LawsError> {
    let services = vec![
        json!({
            "code": "amazon-ec2",
            "name": "Amazon EC2",
            "categories": [
                {"code": "general-guidance", "name": "General Guidance"},
                {"code": "instance-issue", "name": "Instance Issue"},
            ],
        }),
        json!({
            "code": "amazon-s3",
            "name": "Amazon S3",
            "categories": [
                {"code": "general-guidance", "name": "General Guidance"},
                {"code": "permissions", "name": "Permissions"},
            ],
        }),
        json!({
            "code": "general-info",
            "name": "General Info and Getting Started",
            "categories": [
                {"code": "other", "name": "Other"},
            ],
        }),
    ];

    Ok(json_response(json!({
        "services": services,
    })))
}

fn describe_trusted_advisor_checks() -> Result<Response, LawsError> {
    let checks = vec![
        json!({
            "id": "Qch7DwouX1",
            "name": "Security Groups - Specific Ports Unrestricted",
            "description": "Checks security groups for rules that allow unrestricted access to specific ports.",
            "category": "security",
            "metadata": ["Region", "Security Group Name", "Security Group ID", "Protocol", "Port", "Status", "IP Address"],
        }),
        json!({
            "id": "HCP4007jGY",
            "name": "MFA on Root Account",
            "description": "Checks the root account and warns if multi-factor authentication (MFA) is not enabled.",
            "category": "security",
            "metadata": ["Status"],
        }),
        json!({
            "id": "1iG5NDGVre",
            "name": "S3 Bucket Permissions",
            "description": "Checks buckets in Amazon S3 that have open access permissions.",
            "category": "security",
            "metadata": ["Region", "Bucket Name", "ACL Allows List", "ACL Allows Upload/Delete", "Status"],
        }),
        json!({
            "id": "DAvU99Dc4C",
            "name": "Low Utilization Amazon EC2 Instances",
            "description": "Checks EC2 instances that appear to be underutilized.",
            "category": "cost_optimizing",
            "metadata": ["Region", "Instance ID", "Instance Name", "Instance Type", "Estimated Monthly Savings", "CPU Utilization"],
        }),
        json!({
            "id": "Ti39halfu8",
            "name": "Amazon RDS Idle DB Instances",
            "description": "Checks the configuration of RDS DB instances for idle instances.",
            "category": "cost_optimizing",
            "metadata": ["Region", "DB Instance Name", "Multi-AZ", "Instance Type", "Storage Provisioned", "Days Since Last Connection"],
        }),
    ];

    Ok(json_response(json!({
        "checks": checks,
    })))
}

fn describe_trusted_advisor_check_result(payload: &Value) -> Result<Response, LawsError> {
    let check_id = payload["checkId"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing checkId".into()))?;

    Ok(json_response(json!({
        "result": {
            "checkId": check_id,
            "status": "ok",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "resourcesSummary": {
                "resourcesProcessed": 10,
                "resourcesFlagged": 0,
                "resourcesIgnored": 0,
                "resourcesSuppressed": 0,
            },
            "categorySpecificSummary": {
                "costOptimizing": {
                    "estimatedMonthlySavings": 0.0,
                    "estimatedPercentMonthlySavings": 0.0,
                }
            },
            "flaggedResources": [],
        }
    })))
}
