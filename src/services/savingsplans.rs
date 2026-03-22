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
pub struct SavingsPlan {
    pub savings_plan_id: String,
    pub savings_plan_arn: String,
    pub savings_plan_type: String,
    pub payment_option: String,
    pub commitment: String,
    pub term_duration_in_seconds: i64,
    pub state: String,
    pub currency: String,
    pub start: String,
    pub end: String,
    pub tags: Value,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct SavingsPlansState {
    pub plans: DashMap<String, SavingsPlan>,
}

impl Default for SavingsPlansState {
    fn default() -> Self {
        Self {
            plans: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle_request(state: &SavingsPlansState, target: &str, payload: &Value) -> Response {
    let action = target.strip_prefix("AWSSavingsPlan.").unwrap_or(target);

    let result = match action {
        "CreateSavingsPlan" => create_savings_plan(state, payload),
        "DescribeSavingsPlans" => describe_savings_plans(state),
        "DescribeSavingsPlansOfferingRates" => describe_offering_rates(),
        "ListTagsForResource" => list_tags_for_resource(state, payload),
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
        [("Content-Type", "application/x-amz-json-1.0")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_savings_plan(state: &SavingsPlansState, payload: &Value) -> Result<Response, LawsError> {
    let savings_plan_type = payload["savingsPlanOfferingId"]
        .as_str()
        .unwrap_or("Compute")
        .to_string();

    let commitment = payload["commitment"].as_str().unwrap_or("1.0").to_string();

    let payment_option = payload["paymentOption"]
        .as_str()
        .unwrap_or("No Upfront")
        .to_string();

    let term_duration = payload["termInYears"]
        .as_str()
        .map(|t| if t == "3" { 94608000i64 } else { 31536000i64 })
        .unwrap_or(31536000);

    let tags = payload["tags"].clone();

    let id = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:savingsplans:{REGION}:{ACCOUNT_ID}:savingsplan/{id}");

    let now = chrono::Utc::now();
    let start = now.to_rfc3339();
    let end = (now + chrono::Duration::seconds(term_duration)).to_rfc3339();

    let plan = SavingsPlan {
        savings_plan_id: id.clone(),
        savings_plan_arn: arn.clone(),
        savings_plan_type: savings_plan_type.clone(),
        payment_option: payment_option.clone(),
        commitment: commitment.clone(),
        term_duration_in_seconds: term_duration,
        state: "active".to_string(),
        currency: "USD".to_string(),
        start: start.clone(),
        end: end.clone(),
        tags: if tags.is_null() { json!({}) } else { tags },
    };

    state.plans.insert(id.clone(), plan);

    Ok(json_response(json!({
        "savingsPlanId": id,
    })))
}

fn describe_savings_plans(state: &SavingsPlansState) -> Result<Response, LawsError> {
    let plans: Vec<Value> = state
        .plans
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "savingsPlanId": p.savings_plan_id,
                "savingsPlanArn": p.savings_plan_arn,
                "savingsPlanType": p.savings_plan_type,
                "paymentOption": p.payment_option,
                "commitment": p.commitment,
                "termDurationInSeconds": p.term_duration_in_seconds,
                "state": p.state,
                "currency": p.currency,
                "start": p.start,
                "end": p.end,
                "tags": p.tags,
            })
        })
        .collect();

    Ok(json_response(json!({
        "savingsPlans": plans,
    })))
}

fn describe_offering_rates() -> Result<Response, LawsError> {
    let rates = vec![
        json!({
            "savingsPlanOffering": {
                "offeringId": "mock-offering-001",
                "planType": "Compute",
                "paymentOption": "No Upfront",
                "durationSeconds": 31536000,
            },
            "rate": "0.0100000000",
            "unit": "Hrs",
            "productType": "EC2",
            "serviceCode": "AmazonEC2",
            "usageType": "BoxUsage:t3.micro",
            "operation": "RunInstances",
        }),
        json!({
            "savingsPlanOffering": {
                "offeringId": "mock-offering-002",
                "planType": "Compute",
                "paymentOption": "Partial Upfront",
                "durationSeconds": 31536000,
            },
            "rate": "0.0080000000",
            "unit": "Hrs",
            "productType": "EC2",
            "serviceCode": "AmazonEC2",
            "usageType": "BoxUsage:t3.small",
            "operation": "RunInstances",
        }),
    ];

    Ok(json_response(json!({
        "searchResults": rates,
    })))
}

fn list_tags_for_resource(
    state: &SavingsPlansState,
    payload: &Value,
) -> Result<Response, LawsError> {
    let resource_arn = payload["resourceArn"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("Missing resourceArn".into()))?;

    // Find the plan by ARN
    let tags = state
        .plans
        .iter()
        .find(|entry| entry.value().savings_plan_arn == resource_arn)
        .map(|entry| entry.value().tags.clone())
        .unwrap_or(json!({}));

    Ok(json_response(json!({
        "tags": tags,
    })))
}
