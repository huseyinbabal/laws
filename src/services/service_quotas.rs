use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
#[allow(dead_code)]
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ServiceQuota {
    pub service_code: String,
    pub service_name: String,
    pub quota_code: String,
    pub quota_name: String,
    pub value: f64,
    pub adjustable: bool,
    pub global_quota: bool,
    pub unit: String,
}

#[derive(Debug, Clone)]
pub struct QuotaChangeRequest {
    pub id: String,
    pub service_code: String,
    pub quota_code: String,
    pub desired_value: f64,
    pub status: String,
    pub created: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ServiceQuotasState {
    pub quotas: DashMap<String, ServiceQuota>,
    pub requests: DashMap<String, QuotaChangeRequest>,
}

impl Default for ServiceQuotasState {
    fn default() -> Self {
        let state = Self {
            quotas: DashMap::new(),
            requests: DashMap::new(),
        };

        // Seed with some default quotas
        let defaults = vec![
            ServiceQuota {
                service_code: "ec2".to_string(),
                service_name: "Amazon Elastic Compute Cloud (Amazon EC2)".to_string(),
                quota_code: "L-1216C47A".to_string(),
                quota_name: "Running On-Demand Standard instances".to_string(),
                value: 1152.0,
                adjustable: true,
                global_quota: false,
                unit: "None".to_string(),
            },
            ServiceQuota {
                service_code: "s3".to_string(),
                service_name: "Amazon Simple Storage Service (Amazon S3)".to_string(),
                quota_code: "L-DC2B2D3D".to_string(),
                quota_name: "Buckets".to_string(),
                value: 100.0,
                adjustable: true,
                global_quota: false,
                unit: "None".to_string(),
            },
            ServiceQuota {
                service_code: "lambda".to_string(),
                service_name: "AWS Lambda".to_string(),
                quota_code: "L-B99A9384".to_string(),
                quota_name: "Concurrent executions".to_string(),
                value: 1000.0,
                adjustable: true,
                global_quota: false,
                unit: "None".to_string(),
            },
        ];

        for q in defaults {
            let key = format!("{}:{}", q.service_code, q.quota_code);
            state.quotas.insert(key, q);
        }

        state
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &ServiceQuotasState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("ServiceQuotasV20190624.")
        .unwrap_or(target);

    let result = match action {
        "ListServices" => list_services(),
        "ListServiceQuotas" => list_service_quotas(state, payload),
        "GetServiceQuota" => get_service_quota(state, payload),
        "RequestServiceQuotaIncrease" => request_quota_increase(state, payload),
        "ListRequestedServiceQuotaChangeHistory" => list_change_history(state),
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

fn list_services() -> Result<Response, LawsError> {
    let services = vec![
        json!({
            "ServiceCode": "ec2",
            "ServiceName": "Amazon Elastic Compute Cloud (Amazon EC2)",
        }),
        json!({
            "ServiceCode": "s3",
            "ServiceName": "Amazon Simple Storage Service (Amazon S3)",
        }),
        json!({
            "ServiceCode": "lambda",
            "ServiceName": "AWS Lambda",
        }),
        json!({
            "ServiceCode": "rds",
            "ServiceName": "Amazon Relational Database Service (Amazon RDS)",
        }),
        json!({
            "ServiceCode": "iam",
            "ServiceName": "AWS Identity and Access Management (IAM)",
        }),
    ];

    Ok(json_response(json!({
        "Services": services,
    })))
}

fn list_service_quotas(state: &ServiceQuotasState, payload: &Value) -> Result<Response, LawsError> {
    let service_code = payload["ServiceCode"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ServiceCode".into()))?;

    let quotas: Vec<Value> = state
        .quotas
        .iter()
        .filter(|entry| entry.value().service_code == service_code)
        .map(|entry| {
            let q = entry.value();
            json!({
                "ServiceCode": q.service_code,
                "ServiceName": q.service_name,
                "QuotaCode": q.quota_code,
                "QuotaName": q.quota_name,
                "Value": q.value,
                "Adjustable": q.adjustable,
                "GlobalQuota": q.global_quota,
                "Unit": q.unit,
                "QuotaArn": format!(
                    "arn:aws:servicequotas:us-east-1:{}:{}/{}",
                    ACCOUNT_ID, q.service_code, q.quota_code
                ),
            })
        })
        .collect();

    Ok(json_response(json!({
        "Quotas": quotas,
    })))
}

fn get_service_quota(state: &ServiceQuotasState, payload: &Value) -> Result<Response, LawsError> {
    let service_code = payload["ServiceCode"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ServiceCode".into()))?;

    let quota_code = payload["QuotaCode"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing QuotaCode".into()))?;

    let key = format!("{}:{}", service_code, quota_code);

    let quota = state.quotas.get(&key).ok_or_else(|| {
        LawsError::NotFound(format!(
            "Quota '{}' not found for service '{}'",
            quota_code, service_code
        ))
    })?;

    Ok(json_response(json!({
        "Quota": {
            "ServiceCode": quota.service_code,
            "ServiceName": quota.service_name,
            "QuotaCode": quota.quota_code,
            "QuotaName": quota.quota_name,
            "Value": quota.value,
            "Adjustable": quota.adjustable,
            "GlobalQuota": quota.global_quota,
            "Unit": quota.unit,
            "QuotaArn": format!(
                "arn:aws:servicequotas:us-east-1:{}:{}/{}",
                ACCOUNT_ID, quota.service_code, quota.quota_code
            ),
        }
    })))
}

fn request_quota_increase(
    state: &ServiceQuotasState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let service_code = payload["ServiceCode"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing ServiceCode".into()))?
        .to_string();

    let quota_code = payload["QuotaCode"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing QuotaCode".into()))?
        .to_string();

    let desired_value = payload["DesiredValue"]
        .as_f64()
        .ok_or_else(|| LawsError::InvalidRequest("Missing DesiredValue".into()))?;

    let id = uuid::Uuid::new_v4().to_string();
    let created = chrono::Utc::now().to_rfc3339();

    let request = QuotaChangeRequest {
        id: id.clone(),
        service_code: service_code.clone(),
        quota_code: quota_code.clone(),
        desired_value,
        status: "PENDING".to_string(),
        created: created.clone(),
    };

    state.requests.insert(id.clone(), request);

    Ok(json_response(json!({
        "RequestedQuota": {
            "Id": id,
            "ServiceCode": service_code,
            "QuotaCode": quota_code,
            "DesiredValue": desired_value,
            "Status": "PENDING",
            "Created": created,
        }
    })))
}

fn list_change_history(state: &ServiceQuotasState) -> Result<Response, LawsError> {
    let history: Vec<Value> = state
        .requests
        .iter()
        .map(|entry| {
            let r = entry.value();
            json!({
                "Id": r.id,
                "ServiceCode": r.service_code,
                "QuotaCode": r.quota_code,
                "DesiredValue": r.desired_value,
                "Status": r.status,
                "Created": r.created,
            })
        })
        .collect();

    Ok(json_response(json!({
        "RequestedQuotas": history,
    })))
}
