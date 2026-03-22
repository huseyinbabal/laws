use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use http::StatusCode;
use serde_json::{json, Value};

use crate::error::LawsError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Budget {
    pub budget_name: String,
    pub budget_type: String,
    pub budget_limit: Value,
    pub time_unit: String,
    pub notifications: Vec<Notification>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub notification_type: String,
    pub comparison_operator: String,
    pub threshold: f64,
    pub threshold_type: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct BudgetsState {
    pub budgets: DashMap<String, Budget>,
}

impl Default for BudgetsState {
    fn default() -> Self {
        Self {
            budgets: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request handler
// ---------------------------------------------------------------------------

pub async fn handle_request(
    state: &BudgetsState,
    target: &str,
    payload: &Value,
) -> Response {
    let action = target
        .strip_prefix("AWSBudgetServiceGateway.")
        .unwrap_or(target);

    let result = match action {
        "CreateBudget" => create_budget(state, payload),
        "DeleteBudget" => delete_budget(state, payload),
        "DescribeBudget" => describe_budget(state, payload),
        "DescribeBudgets" => describe_budgets(state),
        "CreateNotification" => create_notification(state, payload),
        "DescribeNotificationsForBudget" => describe_notifications(state, payload),
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

fn budget_to_json(b: &Budget) -> Value {
    json!({
        "BudgetName": b.budget_name,
        "BudgetType": b.budget_type,
        "BudgetLimit": b.budget_limit,
        "TimeUnit": b.time_unit,
    })
}

fn notification_to_json(n: &Notification) -> Value {
    json!({
        "NotificationType": n.notification_type,
        "ComparisonOperator": n.comparison_operator,
        "Threshold": n.threshold,
        "ThresholdType": n.threshold_type,
    })
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn create_budget(state: &BudgetsState, payload: &Value) -> Result<Response, LawsError> {
    let budget = &payload["Budget"];
    let budget_name = budget["BudgetName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("BudgetName is required".to_string()))?
        .to_string();

    if state.budgets.contains_key(&budget_name) {
        return Err(LawsError::AlreadyExists(format!(
            "Budget '{}' already exists",
            budget_name
        )));
    }

    let budget_type = budget["BudgetType"]
        .as_str()
        .unwrap_or("COST")
        .to_string();

    let budget_limit = budget["BudgetLimit"].clone();
    let time_unit = budget["TimeUnit"]
        .as_str()
        .unwrap_or("MONTHLY")
        .to_string();

    let now = chrono::Utc::now().to_rfc3339();

    let b = Budget {
        budget_name: budget_name.clone(),
        budget_type,
        budget_limit,
        time_unit,
        notifications: Vec::new(),
        created_at: now,
    };

    state.budgets.insert(budget_name, b);

    Ok(json_response(json!({})))
}

fn delete_budget(state: &BudgetsState, payload: &Value) -> Result<Response, LawsError> {
    let budget_name = payload["BudgetName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("BudgetName is required".to_string()))?;

    state
        .budgets
        .remove(budget_name)
        .ok_or_else(|| LawsError::NotFound(format!("Budget '{}' not found", budget_name)))?;

    Ok(json_response(json!({})))
}

fn describe_budget(state: &BudgetsState, payload: &Value) -> Result<Response, LawsError> {
    let budget_name = payload["BudgetName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("BudgetName is required".to_string()))?;

    let budget = state
        .budgets
        .get(budget_name)
        .ok_or_else(|| LawsError::NotFound(format!("Budget '{}' not found", budget_name)))?;

    Ok(json_response(json!({ "Budget": budget_to_json(budget.value()) })))
}

fn describe_budgets(state: &BudgetsState) -> Result<Response, LawsError> {
    let budgets: Vec<Value> = state
        .budgets
        .iter()
        .map(|entry| budget_to_json(entry.value()))
        .collect();

    Ok(json_response(json!({ "Budgets": budgets })))
}

fn create_notification(state: &BudgetsState, payload: &Value) -> Result<Response, LawsError> {
    let budget_name = payload["BudgetName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("BudgetName is required".to_string()))?;

    let mut budget = state
        .budgets
        .get_mut(budget_name)
        .ok_or_else(|| LawsError::NotFound(format!("Budget '{}' not found", budget_name)))?;

    let notif = &payload["Notification"];
    let notification = Notification {
        notification_type: notif["NotificationType"]
            .as_str()
            .unwrap_or("ACTUAL")
            .to_string(),
        comparison_operator: notif["ComparisonOperator"]
            .as_str()
            .unwrap_or("GREATER_THAN")
            .to_string(),
        threshold: notif["Threshold"].as_f64().unwrap_or(80.0),
        threshold_type: notif["ThresholdType"]
            .as_str()
            .unwrap_or("PERCENTAGE")
            .to_string(),
    };

    budget.notifications.push(notification);

    Ok(json_response(json!({})))
}

fn describe_notifications(state: &BudgetsState, payload: &Value) -> Result<Response, LawsError> {
    let budget_name = payload["BudgetName"]
        .as_str()
        .ok_or_else(|| LawsError::InvalidRequest("BudgetName is required".to_string()))?;

    let budget = state
        .budgets
        .get(budget_name)
        .ok_or_else(|| LawsError::NotFound(format!("Budget '{}' not found", budget_name)))?;

    let notifications: Vec<Value> = budget
        .notifications
        .iter()
        .map(notification_to_json)
        .collect();

    Ok(json_response(json!({ "Notifications": notifications })))
}
