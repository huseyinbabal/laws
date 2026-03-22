use std::sync::Arc;

use axum::extract::State;
use axum::response::Response;
use axum::routing::post;
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
pub struct DetectiveGraph {
    pub graph_arn: String,
    pub created_time: f64,
}

#[derive(Debug, Clone)]
pub struct DetectiveMember {
    pub graph_arn: String,
    pub account_id: String,
    pub email_address: String,
    pub status: String,
    pub invited_time: f64,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DetectiveState {
    pub graphs: DashMap<String, DetectiveGraph>,
    pub members: DashMap<String, DetectiveMember>,
}

impl Default for DetectiveState {
    fn default() -> Self {
        Self {
            graphs: DashMap::new(),
            members: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<DetectiveState>) -> axum::Router {
    axum::Router::new()
        .route("/graph", post(create_graph))
        .route("/graphs/list", post(list_graphs))
        .route("/graph/removal", post(delete_graph))
        .route("/graph/members", post(create_members))
        .route("/graph/members/list", post(list_members))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_epoch() -> f64 {
    chrono::Utc::now().timestamp() as f64
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_graph(
    State(state): State<Arc<DetectiveState>>,
    Json(_payload): Json<Value>,
) -> Response {
    let graph_id = uuid::Uuid::new_v4().to_string();
    let graph_arn = format!(
        "arn:aws:detective:{REGION}:{ACCOUNT_ID}:graph:{graph_id}"
    );
    let created_time = now_epoch();

    let graph = DetectiveGraph {
        graph_arn: graph_arn.clone(),
        created_time,
    };

    state.graphs.insert(graph_arn.clone(), graph);

    rest_json::created(json!({
        "GraphArn": graph_arn,
    }))
}

async fn list_graphs(
    State(state): State<Arc<DetectiveState>>,
    Json(_payload): Json<Value>,
) -> Response {
    let graphs: Vec<Value> = state
        .graphs
        .iter()
        .map(|entry| {
            let g = entry.value();
            json!({
                "Arn": g.graph_arn,
                "CreatedTime": g.created_time,
            })
        })
        .collect();

    rest_json::ok(json!({ "GraphList": graphs }))
}

async fn delete_graph(
    State(state): State<Arc<DetectiveState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let graph_arn = payload["GraphArn"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing GraphArn".into()))?;

        state
            .graphs
            .remove(graph_arn)
            .ok_or_else(|| {
                LawsError::NotFound(format!("Graph '{}' not found", graph_arn))
            })?;

        // Remove associated members
        state.members.retain(|_, m| m.graph_arn != graph_arn);

        Ok(rest_json::no_content())
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn create_members(
    State(state): State<Arc<DetectiveState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let graph_arn = payload["GraphArn"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing GraphArn".into()))?
            .to_string();

        if !state.graphs.contains_key(&graph_arn) {
            return Err(LawsError::NotFound(format!(
                "Graph '{}' not found",
                graph_arn
            )));
        }

        let accounts = payload["Accounts"]
            .as_array()
            .ok_or_else(|| LawsError::InvalidRequest("Missing Accounts".into()))?;

        let mut members = Vec::new();
        let mut unprocessed = Vec::new();
        let invited_time = now_epoch();

        for account in accounts {
            let account_id = match account["AccountId"].as_str() {
                Some(id) => id.to_string(),
                None => {
                    unprocessed.push(json!({
                        "AccountId": account["AccountId"],
                        "Reason": "Missing AccountId",
                    }));
                    continue;
                }
            };

            let email = account["EmailAddress"]
                .as_str()
                .unwrap_or("")
                .to_string();

            let member_key = format!("{}:{}", graph_arn, account_id);

            let member = DetectiveMember {
                graph_arn: graph_arn.clone(),
                account_id: account_id.clone(),
                email_address: email.clone(),
                status: "INVITED".to_string(),
                invited_time,
            };

            members.push(json!({
                "AccountId": account_id,
                "EmailAddress": email,
                "Status": "INVITED",
                "InvitedTime": invited_time,
            }));

            state.members.insert(member_key, member);
        }

        Ok(rest_json::ok(json!({
            "Members": members,
            "UnprocessedAccounts": unprocessed,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_members(
    State(state): State<Arc<DetectiveState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let graph_arn = payload["GraphArn"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing GraphArn".into()))?;

        if !state.graphs.contains_key(graph_arn) {
            return Err(LawsError::NotFound(format!(
                "Graph '{}' not found",
                graph_arn
            )));
        }

        let members: Vec<Value> = state
            .members
            .iter()
            .filter(|entry| entry.graph_arn == graph_arn)
            .map(|entry| {
                let m = entry.value();
                json!({
                    "AccountId": m.account_id,
                    "EmailAddress": m.email_address,
                    "GraphArn": m.graph_arn,
                    "Status": m.status,
                    "InvitedTime": m.invited_time,
                })
            })
            .collect();

        Ok(rest_json::ok(json!({ "MemberDetails": members })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}
