use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Json;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::LawsError;
use crate::protocol::rest_json;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ACCOUNT_ID: &str = "000000000000";
const REGION: &str = "us-east-1";

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub id: String,
    pub sns_topic_arn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub id: String,
    pub name: String,
    pub severity: String,
    pub status: String,
    pub insight_type: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DevOpsGuruState {
    pub channels: DashMap<String, NotificationChannel>,
    pub insights: DashMap<String, Insight>,
}

impl Default for DevOpsGuruState {
    fn default() -> Self {
        Self {
            channels: DashMap::new(),
            insights: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<DevOpsGuruState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/channels",
            post(add_notification_channel).get(list_notification_channels),
        )
        .route("/insights", post(list_insights))
        .route("/insights/{insight_id}", get(describe_insight))
        .route("/insights/search", post(search_insights))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn add_notification_channel(
    State(state): State<Arc<DevOpsGuruState>>,
    Json(body): Json<Value>,
) -> Response {
    let config = match body.get("Config") {
        Some(c) => c,
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest("Missing Config".into()))
        }
    };

    let sns_topic_arn = match config["Sns"]["TopicArn"].as_str() {
        Some(t) => t.to_string(),
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest(
                "Missing Sns.TopicArn".into(),
            ))
        }
    };

    let id = uuid::Uuid::new_v4().to_string();

    let channel = NotificationChannel {
        id: id.clone(),
        sns_topic_arn,
    };

    state.channels.insert(id.clone(), channel);
    rest_json::created(json!({ "Id": id }))
}

async fn list_notification_channels(State(state): State<Arc<DevOpsGuruState>>) -> Response {
    let items: Vec<Value> = state
        .channels
        .iter()
        .map(|entry| {
            let c = entry.value();
            json!({
                "Id": c.id,
                "Config": {
                    "Sns": { "TopicArn": c.sns_topic_arn },
                },
            })
        })
        .collect();

    rest_json::ok(json!({ "Channels": items }))
}

async fn list_insights(
    State(state): State<Arc<DevOpsGuruState>>,
    Json(body): Json<Value>,
) -> Response {
    let status_filter = body["StatusFilter"]["Any"]["Status"].as_str().unwrap_or("");

    let items: Vec<Value> = state
        .insights
        .iter()
        .filter(|entry| status_filter.is_empty() || entry.value().status == status_filter)
        .map(|entry| {
            let i = entry.value();
            json!({
                "Id": i.id,
                "Name": i.name,
                "Severity": i.severity,
                "Status": i.status,
                "InsightTimeRange": { "StartTime": i.created_at },
            })
        })
        .collect();

    rest_json::ok(json!({
        "ProactiveInsights": [],
        "ReactiveInsights": items,
    }))
}

async fn describe_insight(
    State(state): State<Arc<DevOpsGuruState>>,
    Path(insight_id): Path<String>,
) -> Response {
    match state.insights.get(&insight_id) {
        Some(i) => rest_json::ok(json!({
            "ReactiveInsight": {
                "Id": i.id,
                "Name": i.name,
                "Severity": i.severity,
                "Status": i.status,
                "InsightTimeRange": { "StartTime": i.created_at },
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Insight not found: {insight_id}"
        ))),
    }
}

async fn search_insights(
    State(state): State<Arc<DevOpsGuruState>>,
    Json(body): Json<Value>,
) -> Response {
    let insight_type = body["Type"].as_str().unwrap_or("REACTIVE");

    let items: Vec<Value> = state
        .insights
        .iter()
        .filter(|entry| entry.value().insight_type == insight_type)
        .map(|entry| {
            let i = entry.value();
            json!({
                "Id": i.id,
                "Name": i.name,
                "Severity": i.severity,
                "Status": i.status,
            })
        })
        .collect();

    rest_json::ok(json!({
        "ProactiveInsights": [],
        "ReactiveInsights": items,
    }))
}
