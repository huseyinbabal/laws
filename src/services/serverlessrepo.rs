use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Response;
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
pub struct Application {
    pub application_id: String,
    pub arn: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub spdx_license_id: String,
    pub semantic_version: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct ServerlessRepoState {
    pub applications: DashMap<String, Application>,
}

impl Default for ServerlessRepoState {
    fn default() -> Self {
        Self {
            applications: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<ServerlessRepoState>) -> axum::Router {
    axum::Router::new()
        .route(
            "/applications",
            axum::routing::post(create_application).get(list_applications),
        )
        .route(
            "/applications/{id}",
            axum::routing::get(get_application).delete(delete_application),
        )
        .route(
            "/applications/{id}/changesets",
            axum::routing::post(create_cloud_formation_change_set),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn app_to_json(app: &Application) -> Value {
    json!({
        "applicationId": app.application_id,
        "arn": app.arn,
        "name": app.name,
        "description": app.description,
        "author": app.author,
        "spdxLicenseId": app.spdx_license_id,
        "version": {
            "semanticVersion": app.semantic_version,
        },
        "creationTime": app.created_at,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_application(
    State(state): State<Arc<ServerlessRepoState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing name".into()))?
            .to_string();

        let description = payload["description"].as_str().unwrap_or("").to_string();

        let author = payload["author"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing author".into()))?
            .to_string();

        let spdx_license_id = payload["spdxLicenseId"]
            .as_str()
            .unwrap_or("MIT")
            .to_string();

        let semantic_version = payload["semanticVersion"]
            .as_str()
            .unwrap_or("1.0.0")
            .to_string();

        let application_id = uuid::Uuid::new_v4().to_string();
        let arn =
            format!("arn:aws:serverlessrepo:{REGION}:{ACCOUNT_ID}:applications/{application_id}");
        let created_at = chrono::Utc::now().to_rfc3339();

        let app = Application {
            application_id: application_id.clone(),
            arn,
            name,
            description,
            author,
            spdx_license_id,
            semantic_version,
            created_at,
        };

        let resp = app_to_json(&app);
        state.applications.insert(application_id, app);

        Ok(rest_json::created(resp))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_applications(State(state): State<Arc<ServerlessRepoState>>) -> Response {
    let apps: Vec<Value> = state
        .applications
        .iter()
        .map(|entry| {
            let a = entry.value();
            json!({
                "applicationId": a.application_id,
                "name": a.name,
                "description": a.description,
                "author": a.author,
                "creationTime": a.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "applications": apps }))
}

async fn get_application(
    State(state): State<Arc<ServerlessRepoState>>,
    Path(id): Path<String>,
) -> Response {
    match state.applications.get(&id) {
        Some(app) => rest_json::ok(app_to_json(app.value())),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Application '{}' not found",
            id
        ))),
    }
}

async fn delete_application(
    State(state): State<Arc<ServerlessRepoState>>,
    Path(id): Path<String>,
) -> Response {
    match state.applications.remove(&id) {
        Some(_) => rest_json::no_content(),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Application '{}' not found",
            id
        ))),
    }
}

async fn create_cloud_formation_change_set(
    State(state): State<Arc<ServerlessRepoState>>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        if !state.applications.contains_key(&id) {
            return Err(LawsError::NotFound(format!(
                "Application '{}' not found",
                id
            )));
        }

        let stack_name = payload["stackName"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing stackName".into()))?
            .to_string();

        let change_set_id = uuid::Uuid::new_v4().to_string();
        let stack_id = uuid::Uuid::new_v4().to_string();

        Ok(rest_json::created(json!({
            "applicationId": id,
            "changeSetId": format!(
                "arn:aws:cloudformation:{REGION}:{ACCOUNT_ID}:changeSet/{change_set_id}"
            ),
            "stackId": format!(
                "arn:aws:cloudformation:{REGION}:{ACCOUNT_ID}:stack/{stack_name}/{stack_id}"
            ),
            "semanticVersion": "1.0.0",
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}
