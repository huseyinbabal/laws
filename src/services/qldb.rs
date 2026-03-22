use std::sync::Arc;

use axum::extract::{Path, State};
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
pub struct QldbLedger {
    pub name: String,
    pub arn: String,
    pub state: String,
    pub creation_date_time: String,
    pub permissions_mode: String,
    pub deletion_protection: bool,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct QldbState {
    pub ledgers: DashMap<String, QldbLedger>,
}

impl Default for QldbState {
    fn default() -> Self {
        Self {
            ledgers: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<QldbState>) -> axum::Router {
    axum::Router::new()
        .route("/ledgers", post(create_ledger).get(list_ledgers))
        .route(
            "/ledgers/{name}",
            get(describe_ledger)
                .delete(delete_ledger)
                .put(update_ledger),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_ledger(
    State(state): State<Arc<QldbState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["Name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
            .to_string();

        let permissions_mode = payload["PermissionsMode"]
            .as_str()
            .unwrap_or("ALLOW_ALL")
            .to_string();

        let deletion_protection = payload["DeletionProtection"].as_bool().unwrap_or(true);

        let arn = format!("arn:aws:qldb:{REGION}:{ACCOUNT_ID}:ledger/{name}");
        let now = chrono::Utc::now().to_rfc3339();

        let ledger = QldbLedger {
            name: name.clone(),
            arn: arn.clone(),
            state: "ACTIVE".to_string(),
            creation_date_time: now.clone(),
            permissions_mode: permissions_mode.clone(),
            deletion_protection,
        };

        state.ledgers.insert(name.clone(), ledger);

        Ok(rest_json::created(json!({
            "Name": name,
            "Arn": arn,
            "State": "CREATING",
            "CreationDateTime": now,
            "PermissionsMode": permissions_mode,
            "DeletionProtection": deletion_protection,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_ledgers(State(state): State<Arc<QldbState>>) -> Response {
    let ledgers: Vec<Value> = state
        .ledgers
        .iter()
        .map(|entry| {
            let l = entry.value();
            json!({
                "Name": l.name,
                "State": l.state,
                "CreationDateTime": l.creation_date_time,
            })
        })
        .collect();

    rest_json::ok(json!({ "Ledgers": ledgers }))
}

async fn describe_ledger(
    State(state): State<Arc<QldbState>>,
    Path(name): Path<String>,
) -> Response {
    match state.ledgers.get(&name) {
        Some(ledger) => rest_json::ok(json!({
            "Name": ledger.name,
            "Arn": ledger.arn,
            "State": ledger.state,
            "CreationDateTime": ledger.creation_date_time,
            "PermissionsMode": ledger.permissions_mode,
            "DeletionProtection": ledger.deletion_protection,
        })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Ledger '{}' not found", name)))
        }
    }
}

async fn delete_ledger(State(state): State<Arc<QldbState>>, Path(name): Path<String>) -> Response {
    match state.ledgers.remove(&name) {
        Some(_) => rest_json::ok(json!({})),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Ledger '{}' not found", name)))
        }
    }
}

async fn update_ledger(
    State(state): State<Arc<QldbState>>,
    Path(name): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    match state.ledgers.get_mut(&name) {
        Some(mut ledger) => {
            if let Some(mode) = payload["PermissionsMode"].as_str() {
                ledger.permissions_mode = mode.to_string();
            }
            if let Some(dp) = payload["DeletionProtection"].as_bool() {
                ledger.deletion_protection = dp;
            }

            rest_json::ok(json!({
                "Name": ledger.name,
                "Arn": ledger.arn,
                "State": ledger.state,
                "CreationDateTime": ledger.creation_date_time,
                "PermissionsMode": ledger.permissions_mode,
                "DeletionProtection": ledger.deletion_protection,
            }))
        }
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Ledger '{}' not found", name)))
        }
    }
}
