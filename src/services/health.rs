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
pub struct HealthEvent {
    pub arn: String,
    pub service: String,
    pub event_type_code: String,
    pub event_type_category: String,
    pub region: String,
    pub status: String,
    pub start_time: String,
    pub description: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct HealthState {
    pub events: DashMap<String, HealthEvent>,
}

impl Default for HealthState {
    fn default() -> Self {
        let events = DashMap::new();
        // Seed with mock health event data
        let now = chrono::Utc::now().to_rfc3339();
        events.insert(
            "arn:aws:health:us-east-1::event/EC2/OPERATIONAL_ISSUE/mock-001".to_string(),
            HealthEvent {
                arn: "arn:aws:health:us-east-1::event/EC2/OPERATIONAL_ISSUE/mock-001".to_string(),
                service: "EC2".to_string(),
                event_type_code: "AWS_EC2_OPERATIONAL_ISSUE".to_string(),
                event_type_category: "issue".to_string(),
                region: REGION.to_string(),
                status: "closed".to_string(),
                start_time: now.clone(),
                description: "Mock EC2 operational issue for testing".to_string(),
            },
        );
        events.insert(
            "arn:aws:health:us-east-1::event/RDS/MAINTENANCE/mock-002".to_string(),
            HealthEvent {
                arn: "arn:aws:health:us-east-1::event/RDS/MAINTENANCE/mock-002".to_string(),
                service: "RDS".to_string(),
                event_type_code: "AWS_RDS_MAINTENANCE".to_string(),
                event_type_category: "scheduledChange".to_string(),
                region: REGION.to_string(),
                status: "upcoming".to_string(),
                start_time: now,
                description: "Mock RDS maintenance event for testing".to_string(),
            },
        );
        Self { events }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &HealthState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AWSHealth_20160804.")
        .unwrap_or(target);

    let result = match action {
        "DescribeEvents" => describe_events(state, payload),
        "DescribeEventDetails" => describe_event_details(state, payload),
        "DescribeAffectedEntities" => describe_affected_entities(state, payload),
        "DescribeEventTypes" => describe_event_types(state),
        "DescribeEventAggregates" => describe_event_aggregates(state),
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

fn describe_events(
    state: &HealthState,
    _payload: &Value,
) -> Result<Response, LawsError> {
    let events: Vec<Value> = state
        .events
        .iter()
        .map(|e| {
            let ev = e.value();
            json!({
                "arn": ev.arn,
                "service": ev.service,
                "eventTypeCode": ev.event_type_code,
                "eventTypeCategory": ev.event_type_category,
                "region": ev.region,
                "statusCode": ev.status,
                "startTime": ev.start_time,
            })
        })
        .collect();

    Ok(json_response(json!({
        "events": events
    })))
}

fn describe_event_details(
    state: &HealthState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let event_arns: Vec<&str> = payload["eventArns"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut successful_set = Vec::new();
    let mut failed_set = Vec::new();

    for arn in &event_arns {
        match state.events.get(*arn) {
            Some(ev) => {
                successful_set.push(json!({
                    "event": {
                        "arn": ev.arn,
                        "service": ev.service,
                        "eventTypeCode": ev.event_type_code,
                        "eventTypeCategory": ev.event_type_category,
                        "region": ev.region,
                        "statusCode": ev.status,
                        "startTime": ev.start_time,
                    },
                    "eventDescription": {
                        "latestDescription": ev.description
                    }
                }));
            }
            None => {
                failed_set.push(json!({
                    "eventArn": arn,
                    "errorName": "ResourceNotFoundException",
                    "errorMessage": "Event not found"
                }));
            }
        }
    }

    Ok(json_response(json!({
        "successfulSet": successful_set,
        "failedSet": failed_set
    })))
}

fn describe_affected_entities(
    state: &HealthState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let event_arn = payload["filter"]["eventArns"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let entities: Vec<Value> = if state.events.contains_key(event_arn) {
        vec![json!({
            "entityValue": format!("i-{}", &uuid::Uuid::new_v4().to_string()[..12]),
            "eventArn": event_arn,
            "awsAccountId": ACCOUNT_ID,
            "statusCode": "IMPAIRED"
        })]
    } else {
        vec![]
    };

    Ok(json_response(json!({
        "entities": entities
    })))
}

fn describe_event_types(
    _state: &HealthState,
) -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "eventTypes": [
            {
                "service": "EC2",
                "code": "AWS_EC2_OPERATIONAL_ISSUE",
                "category": "issue"
            },
            {
                "service": "RDS",
                "code": "AWS_RDS_MAINTENANCE",
                "category": "scheduledChange"
            },
            {
                "service": "LAMBDA",
                "code": "AWS_LAMBDA_OPERATIONAL_ISSUE",
                "category": "issue"
            }
        ]
    })))
}

fn describe_event_aggregates(
    state: &HealthState,
) -> Result<Response, LawsError> {
    let count = state.events.len();

    Ok(json_response(json!({
        "eventAggregates": [
            {
                "aggregateValue": "issue",
                "count": count
            }
        ]
    })))
}
