use std::sync::Arc;

use axum::extract::State;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Json;
use dashmap::DashMap;
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

#[derive(Debug, Clone)]
pub struct SecurityHubFinding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub status: String,
    pub product_arn: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct SecurityHubState {
    pub hub_enabled: DashMap<String, bool>,
    pub findings: DashMap<String, SecurityHubFinding>,
}

impl Default for SecurityHubState {
    fn default() -> Self {
        Self {
            hub_enabled: DashMap::new(),
            findings: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<SecurityHubState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/productSubscriptions",
            post(enable_security_hub).get(get_enabled_standards),
        )
        .route("/findings/import", post(batch_import_findings))
        .route("/findings", post(get_findings))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn enable_security_hub(
    State(state): State<Arc<SecurityHubState>>,
    Json(_payload): Json<Value>,
) -> Response {
    let hub_arn = format!(
        "arn:aws:securityhub:{REGION}:{ACCOUNT_ID}:hub/default"
    );

    state.hub_enabled.insert("default".to_string(), true);

    rest_json::ok(json!({
        "HubArn": hub_arn,
    }))
}

async fn get_enabled_standards(
    State(state): State<Arc<SecurityHubState>>,
) -> Response {
    let enabled = state.hub_enabled.contains_key("default");

    rest_json::ok(json!({
        "StandardsSubscriptions": if enabled {
            json!([{
                "StandardsArn": format!("arn:aws:securityhub:{}::standards/aws-foundational-security-best-practices/v/1.0.0", REGION),
                "StandardsSubscriptionArn": format!("arn:aws:securityhub:{REGION}:{ACCOUNT_ID}:subscription/aws-foundational-security-best-practices/v/1.0.0"),
                "StandardsStatus": "READY",
            }])
        } else {
            json!([])
        },
    }))
}

async fn batch_import_findings(
    State(state): State<Arc<SecurityHubState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let findings_arr = payload["Findings"]
            .as_array()
            .ok_or_else(|| LawsError::InvalidRequest("Findings array is required".into()))?;

        let mut success_count = 0u64;

        for finding in findings_arr {
            let id = finding["Id"]
                .as_str()
                .unwrap_or(&uuid::Uuid::new_v4().to_string())
                .to_string();

            let title = finding["Title"]
                .as_str()
                .unwrap_or("")
                .to_string();

            let description = finding["Description"]
                .as_str()
                .unwrap_or("")
                .to_string();

            let severity = finding["Severity"]["Label"]
                .as_str()
                .unwrap_or("INFORMATIONAL")
                .to_string();

            let product_arn = finding["ProductArn"]
                .as_str()
                .unwrap_or("")
                .to_string();

            let now = chrono::Utc::now().to_rfc3339();

            let f = SecurityHubFinding {
                id: id.clone(),
                title,
                description,
                severity,
                status: "NEW".to_string(),
                product_arn,
                created_at: now,
            };

            state.findings.insert(id, f);
            success_count += 1;
        }

        Ok(rest_json::ok(json!({
            "FailedCount": 0,
            "SuccessCount": success_count,
            "FailedFindings": [],
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn get_findings(
    State(state): State<Arc<SecurityHubState>>,
    Json(_payload): Json<Value>,
) -> Response {
    let findings: Vec<Value> = state
        .findings
        .iter()
        .map(|entry| {
            let f = entry.value();
            json!({
                "Id": f.id,
                "Title": f.title,
                "Description": f.description,
                "Severity": { "Label": f.severity },
                "Status": f.status,
                "ProductArn": f.product_arn,
                "CreatedAt": f.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "Findings": findings }))
}
