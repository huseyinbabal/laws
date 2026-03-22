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
pub struct PinpointApp {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub created: String,
    pub campaigns: DashMap<String, PinpointCampaign>,
}

#[derive(Debug, Clone)]
pub struct PinpointCampaign {
    pub id: String,
    pub name: String,
    pub arn: String,
    pub application_id: String,
    pub state: String,
    pub created: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct PinpointState {
    pub apps: DashMap<String, PinpointApp>,
}

impl Default for PinpointState {
    fn default() -> Self {
        Self {
            apps: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<PinpointState>) -> axum::Router {
    axum::Router::new()
        .route("/v1/apps", post(create_app).get(get_apps))
        .route("/v1/apps/{id}", get(get_app).delete(delete_app))
        .route(
            "/v1/apps/{id}/campaigns",
            post(create_campaign).get(get_campaigns),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_app(
    State(state): State<Arc<PinpointState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["Name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Name is required".into()))?
            .to_string();

        let id = uuid::Uuid::new_v4().to_string();
        let arn = format!("arn:aws:mobiletargeting:{REGION}:{ACCOUNT_ID}:apps/{id}");
        let now = chrono::Utc::now().to_rfc3339();

        let app = PinpointApp {
            id: id.clone(),
            name: name.clone(),
            arn: arn.clone(),
            created: now,
            campaigns: DashMap::new(),
        };

        state.apps.insert(id.clone(), app);

        Ok(rest_json::created(json!({
            "ApplicationResponse": {
                "Id": id,
                "Name": name,
                "Arn": arn,
            }
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn get_apps(State(state): State<Arc<PinpointState>>) -> Response {
    let items: Vec<Value> = state
        .apps
        .iter()
        .map(|entry| {
            let a = entry.value();
            json!({
                "Id": a.id,
                "Name": a.name,
                "Arn": a.arn,
            })
        })
        .collect();

    rest_json::ok(json!({
        "ApplicationsResponse": {
            "Item": items,
        }
    }))
}

async fn get_app(State(state): State<Arc<PinpointState>>, Path(id): Path<String>) -> Response {
    match state.apps.get(&id) {
        Some(app) => rest_json::ok(json!({
            "ApplicationResponse": {
                "Id": app.id,
                "Name": app.name,
                "Arn": app.arn,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!("App '{}' not found", id))),
    }
}

async fn delete_app(State(state): State<Arc<PinpointState>>, Path(id): Path<String>) -> Response {
    match state.apps.remove(&id) {
        Some((_, app)) => rest_json::ok(json!({
            "ApplicationResponse": {
                "Id": app.id,
                "Name": app.name,
                "Arn": app.arn,
            }
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!("App '{}' not found", id))),
    }
}

async fn create_campaign(
    State(state): State<Arc<PinpointState>>,
    Path(app_id): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let app = state
            .apps
            .get(&app_id)
            .ok_or_else(|| LawsError::NotFound(format!("App '{}' not found", app_id)))?;

        let name = payload["Name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Name is required".into()))?
            .to_string();

        let campaign_id = uuid::Uuid::new_v4().to_string();
        let arn = format!(
            "arn:aws:mobiletargeting:{REGION}:{ACCOUNT_ID}:apps/{app_id}/campaigns/{campaign_id}"
        );
        let now = chrono::Utc::now().to_rfc3339();

        let campaign = PinpointCampaign {
            id: campaign_id.clone(),
            name: name.clone(),
            arn: arn.clone(),
            application_id: app_id.clone(),
            state: "COMPLETED".to_string(),
            created: now,
        };

        app.campaigns.insert(campaign_id.clone(), campaign);

        Ok(rest_json::created(json!({
            "CampaignResponse": {
                "Id": campaign_id,
                "Name": name,
                "Arn": arn,
                "ApplicationId": app_id,
                "State": { "CampaignStatus": "COMPLETED" },
            }
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn get_campaigns(
    State(state): State<Arc<PinpointState>>,
    Path(app_id): Path<String>,
) -> Response {
    match state.apps.get(&app_id) {
        Some(app) => {
            let items: Vec<Value> = app
                .campaigns
                .iter()
                .map(|entry| {
                    let c = entry.value();
                    json!({
                        "Id": c.id,
                        "Name": c.name,
                        "Arn": c.arn,
                        "ApplicationId": c.application_id,
                        "State": { "CampaignStatus": c.state },
                    })
                })
                .collect();

            rest_json::ok(json!({
                "CampaignsResponse": {
                    "Item": items,
                }
            }))
        }
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("App '{}' not found", app_id)))
        }
    }
}
