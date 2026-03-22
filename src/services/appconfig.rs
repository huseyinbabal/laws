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
pub struct AppConfigApplication {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct AppConfigEnvironment {
    pub application_id: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct AppConfigProfile {
    pub application_id: String,
    pub id: String,
    pub name: String,
    pub location_uri: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AppConfigState {
    pub applications: DashMap<String, AppConfigApplication>,
    pub environments: DashMap<String, AppConfigEnvironment>,
    pub profiles: DashMap<String, AppConfigProfile>,
}

impl Default for AppConfigState {
    fn default() -> Self {
        Self {
            applications: DashMap::new(),
            environments: DashMap::new(),
            profiles: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<AppConfigState>) -> axum::Router {
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
            "/applications/{id}/environments",
            axum::routing::post(create_environment).get(list_environments),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_id() -> String {
    uuid::Uuid::new_v4().to_string()[..8].to_string()
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_application(
    State(state): State<Arc<AppConfigState>>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        let name = payload["Name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
            .to_string();

        let description = payload["Description"].as_str().unwrap_or("").to_string();

        let id = random_id();

        let app = AppConfigApplication {
            id: id.clone(),
            name: name.clone(),
            description: description.clone(),
        };

        state.applications.insert(id.clone(), app);

        Ok(rest_json::created(json!({
            "Id": id,
            "Name": name,
            "Description": description,
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_applications(State(state): State<Arc<AppConfigState>>) -> Response {
    let items: Vec<Value> = state
        .applications
        .iter()
        .map(|entry| {
            let app = entry.value();
            json!({
                "Id": app.id,
                "Name": app.name,
                "Description": app.description,
            })
        })
        .collect();

    rest_json::ok(json!({ "Items": items }))
}

async fn get_application(
    State(state): State<Arc<AppConfigState>>,
    Path(id): Path<String>,
) -> Response {
    match state.applications.get(&id) {
        Some(app) => rest_json::ok(json!({
            "Id": app.id,
            "Name": app.name,
            "Description": app.description,
        })),
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Application '{}' not found",
            id
        ))),
    }
}

async fn delete_application(
    State(state): State<Arc<AppConfigState>>,
    Path(id): Path<String>,
) -> Response {
    match state.applications.remove(&id) {
        Some(_) => {
            // Clean up associated environments and profiles
            state.environments.retain(|_, e| e.application_id != id);
            state.profiles.retain(|_, p| p.application_id != id);
            rest_json::no_content()
        }
        None => rest_json::error_response(&LawsError::NotFound(format!(
            "Application '{}' not found",
            id
        ))),
    }
}

async fn create_environment(
    State(state): State<Arc<AppConfigState>>,
    Path(app_id): Path<String>,
    Json(payload): Json<Value>,
) -> Response {
    let result = (|| -> Result<Response, LawsError> {
        if !state.applications.contains_key(&app_id) {
            return Err(LawsError::NotFound(format!(
                "Application '{}' not found",
                app_id
            )));
        }

        let name = payload["Name"]
            .as_str()
            .ok_or_else(|| LawsError::InvalidRequest("Missing Name".into()))?
            .to_string();

        let description = payload["Description"].as_str().unwrap_or("").to_string();

        let id = random_id();

        let env = AppConfigEnvironment {
            application_id: app_id.clone(),
            id: id.clone(),
            name: name.clone(),
            description: description.clone(),
            state: "READY_FOR_DEPLOYMENT".to_string(),
        };

        state.environments.insert(id.clone(), env);

        Ok(rest_json::created(json!({
            "ApplicationId": app_id,
            "Id": id,
            "Name": name,
            "Description": description,
            "State": "READY_FOR_DEPLOYMENT",
        })))
    })();

    match result {
        Ok(resp) => resp,
        Err(e) => rest_json::error_response(&e),
    }
}

async fn list_environments(
    State(state): State<Arc<AppConfigState>>,
    Path(app_id): Path<String>,
) -> Response {
    if !state.applications.contains_key(&app_id) {
        return rest_json::error_response(&LawsError::NotFound(format!(
            "Application '{}' not found",
            app_id
        )));
    }

    let items: Vec<Value> = state
        .environments
        .iter()
        .filter(|entry| entry.application_id == app_id)
        .map(|entry| {
            let env = entry.value();
            json!({
                "ApplicationId": env.application_id,
                "Id": env.id,
                "Name": env.name,
                "Description": env.description,
                "State": env.state,
            })
        })
        .collect();

    rest_json::ok(json!({ "Items": items }))
}
