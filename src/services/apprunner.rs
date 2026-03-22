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
pub struct AppRunnerService {
    pub service_name: String,
    pub service_id: String,
    pub service_arn: String,
    pub service_url: String,
    pub status: String,
    pub source_type: String,
    pub created_at: f64,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AppRunnerState {
    pub services: DashMap<String, AppRunnerService>,
}

impl Default for AppRunnerState {
    fn default() -> Self {
        Self {
            services: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &AppRunnerState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AppRunner.")
        .unwrap_or(target);

    let result = match action {
        "CreateService" => create_service(state, payload),
        "DeleteService" => delete_service(state, payload),
        "DescribeService" => describe_service(state, payload),
        "ListServices" => list_services(state),
        "PauseService" => pause_service(state, payload),
        "ResumeService" => resume_service(state, payload),
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

fn now_epoch() -> f64 {
    chrono::Utc::now().timestamp() as f64
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_service(
    state: &AppRunnerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let service_name = payload["ServiceName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("ServiceName is required".to_string()))?
        .to_string();

    let service_id = uuid::Uuid::new_v4().to_string()[..8].to_string();

    let service_arn = format!(
        "arn:aws:apprunner:{REGION}:{ACCOUNT_ID}:service/{service_name}/{service_id}"
    );

    let service_url = format!(
        "{service_id}.{REGION}.awsapprunner.com"
    );

    let source_type = payload["SourceConfiguration"]["CodeRepository"]
        .as_object()
        .map(|_| "CODE_REPOSITORY")
        .unwrap_or("IMAGE_REPOSITORY")
        .to_string();

    let created_at = now_epoch();

    let service = AppRunnerService {
        service_name: service_name.clone(),
        service_id: service_id.clone(),
        service_arn: service_arn.clone(),
        service_url: service_url.clone(),
        status: "RUNNING".to_string(),
        source_type,
        created_at,
    };

    state.services.insert(service_arn.clone(), service);

    Ok(json_response(json!({
        "Service": {
            "ServiceName": service_name,
            "ServiceId": service_id,
            "ServiceArn": service_arn,
            "ServiceUrl": service_url,
            "Status": "RUNNING",
            "CreatedAt": created_at,
        },
        "OperationId": uuid::Uuid::new_v4().to_string(),
    })))
}

fn delete_service(
    state: &AppRunnerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let service_arn = payload["ServiceArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("ServiceArn is required".to_string())
        })?;

    let (_, service) = state
        .services
        .remove(service_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Service '{}' not found", service_arn))
        })?;

    Ok(json_response(json!({
        "Service": {
            "ServiceName": service.service_name,
            "ServiceId": service.service_id,
            "ServiceArn": service_arn,
            "ServiceUrl": service.service_url,
            "Status": "DELETED",
            "CreatedAt": service.created_at,
        },
        "OperationId": uuid::Uuid::new_v4().to_string(),
    })))
}

fn describe_service(
    state: &AppRunnerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let service_arn = payload["ServiceArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("ServiceArn is required".to_string())
        })?;

    let service = state
        .services
        .get(service_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Service '{}' not found", service_arn))
        })?;

    Ok(json_response(json!({
        "Service": {
            "ServiceName": service.service_name,
            "ServiceId": service.service_id,
            "ServiceArn": service.service_arn,
            "ServiceUrl": service.service_url,
            "Status": service.status,
            "SourceType": service.source_type,
            "CreatedAt": service.created_at,
        }
    })))
}

fn list_services(state: &AppRunnerState) -> Result<Response, LawsError> {
    let services: Vec<Value> = state
        .services
        .iter()
        .map(|entry| {
            let s = entry.value();
            json!({
                "ServiceName": s.service_name,
                "ServiceId": s.service_id,
                "ServiceArn": s.service_arn,
                "ServiceUrl": s.service_url,
                "Status": s.status,
                "CreatedAt": s.created_at,
            })
        })
        .collect();

    Ok(json_response(json!({
        "ServiceSummaryList": services,
    })))
}

fn pause_service(
    state: &AppRunnerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let service_arn = payload["ServiceArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("ServiceArn is required".to_string())
        })?;

    let mut service = state
        .services
        .get_mut(service_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Service '{}' not found", service_arn))
        })?;

    service.status = "PAUSED".to_string();

    Ok(json_response(json!({
        "Service": {
            "ServiceName": service.service_name,
            "ServiceId": service.service_id,
            "ServiceArn": service.service_arn,
            "ServiceUrl": service.service_url,
            "Status": "PAUSED",
            "CreatedAt": service.created_at,
        },
        "OperationId": uuid::Uuid::new_v4().to_string(),
    })))
}

fn resume_service(
    state: &AppRunnerState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let service_arn = payload["ServiceArn"]
        .as_str()
        .ok_or_else(|| {
            LawsError::InvalidRequest("ServiceArn is required".to_string())
        })?;

    let mut service = state
        .services
        .get_mut(service_arn)
        .ok_or_else(|| {
            LawsError::NotFound(format!("Service '{}' not found", service_arn))
        })?;

    service.status = "RUNNING".to_string();

    Ok(json_response(json!({
        "Service": {
            "ServiceName": service.service_name,
            "ServiceId": service.service_id,
            "ServiceArn": service.service_arn,
            "ServiceUrl": service.service_url,
            "Status": "RUNNING",
            "CreatedAt": service.created_at,
        },
        "OperationId": uuid::Uuid::new_v4().to_string(),
    })))
}
