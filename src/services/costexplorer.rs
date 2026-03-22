use axum::response::{IntoResponse, Response};
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct CostExplorerState;

impl Default for CostExplorerState {
    fn default() -> Self {
        Self
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(_state: &CostExplorerState, target: &str, payload: &Value) -> Response {
    let action = target
        .strip_prefix("AWSInsightsIndexService.")
        .unwrap_or(target);

    let result = match action {
        "GetCostAndUsage" => get_cost_and_usage(payload),
        "GetCostForecast" => get_cost_forecast(payload),
        "GetReservationUtilization" => get_reservation_utilization(payload),
        "GetSavingsPlansUtilization" => get_savings_plans_utilization(payload),
        "GetDimensionValues" => get_dimension_values(payload),
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

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn get_cost_and_usage(payload: &Value) -> Result<Response, LawsError> {
    let start = payload["TimePeriod"]["Start"]
        .as_str()
        .unwrap_or("2024-01-01");
    let end = payload["TimePeriod"]["End"]
        .as_str()
        .unwrap_or("2024-01-31");

    Ok(json_response(json!({
        "ResultsByTime": [
            {
                "TimePeriod": { "Start": start, "End": end },
                "Total": {
                    "UnblendedCost": { "Amount": "142.50", "Unit": "USD" },
                    "BlendedCost": { "Amount": "140.00", "Unit": "USD" },
                },
                "Groups": [],
                "Estimated": true,
            }
        ],
        "DimensionValueAttributes": [],
    })))
}

fn get_cost_forecast(payload: &Value) -> Result<Response, LawsError> {
    let start = payload["TimePeriod"]["Start"]
        .as_str()
        .unwrap_or("2024-02-01");
    let end = payload["TimePeriod"]["End"]
        .as_str()
        .unwrap_or("2024-02-28");

    Ok(json_response(json!({
        "Total": { "Amount": "155.00", "Unit": "USD" },
        "ForecastResultsByTime": [
            {
                "TimePeriod": { "Start": start, "End": end },
                "MeanValue": "155.00",
                "PredictionIntervalLowerBound": "120.00",
                "PredictionIntervalUpperBound": "190.00",
            }
        ],
    })))
}

fn get_reservation_utilization(_payload: &Value) -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "UtilizationsByTime": [
            {
                "TimePeriod": { "Start": "2024-01-01", "End": "2024-01-31" },
                "Total": {
                    "UtilizationPercentage": "85.5",
                    "PurchasedHours": "744",
                    "TotalActualHours": "636.12",
                    "UnusedHours": "107.88",
                    "OnDemandCostOfRIHoursUsed": "95.00",
                    "NetRISavings": "45.00",
                },
                "Groups": [],
            }
        ],
        "Total": {
            "UtilizationPercentage": "85.5",
            "PurchasedHours": "744",
            "TotalActualHours": "636.12",
            "UnusedHours": "107.88",
        },
    })))
}

fn get_savings_plans_utilization(_payload: &Value) -> Result<Response, LawsError> {
    Ok(json_response(json!({
        "SavingsPlansUtilizationsByTime": [
            {
                "TimePeriod": { "Start": "2024-01-01", "End": "2024-01-31" },
                "Utilization": {
                    "TotalCommitment": "100.00",
                    "UsedCommitment": "92.00",
                    "UnusedCommitment": "8.00",
                    "UtilizationPercentage": "92.0",
                },
                "Savings": {
                    "NetSavings": "35.00",
                    "OnDemandCostEquivalent": "127.00",
                },
                "AmortizedCommitment": {
                    "AmortizedRecurringCommitment": "0.00",
                    "AmortizedUpfrontCommitment": "100.00",
                    "TotalAmortizedCommitment": "100.00",
                },
            }
        ],
        "Total": {
            "Utilization": {
                "UtilizationPercentage": "92.0",
                "TotalCommitment": "100.00",
                "UsedCommitment": "92.00",
                "UnusedCommitment": "8.00",
            },
        },
    })))
}

fn get_dimension_values(payload: &Value) -> Result<Response, LawsError> {
    let dimension = payload["Dimension"].as_str().unwrap_or("SERVICE");

    let mock_values = match dimension {
        "SERVICE" => vec![
            "Amazon Elastic Compute Cloud - Compute",
            "Amazon Simple Storage Service",
            "Amazon Relational Database Service",
            "AWS Lambda",
        ],
        "REGION" => vec!["us-east-1", "us-west-2", "eu-west-1"],
        "INSTANCE_TYPE" => vec!["t3.micro", "t3.small", "m5.large", "r5.xlarge"],
        _ => vec!["value1", "value2"],
    };

    let dimension_values: Vec<Value> = mock_values
        .iter()
        .map(|v| json!({ "Value": v, "Attributes": {} }))
        .collect();

    Ok(json_response(json!({
        "DimensionValues": dimension_values,
        "ReturnSize": dimension_values.len(),
        "TotalSize": dimension_values.len(),
    })))
}
