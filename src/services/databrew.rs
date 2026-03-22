use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Json;
use chrono::Utc;
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
pub struct BrewProject {
    pub name: String,
    pub recipe_name: String,
    pub dataset_name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub description: String,
    pub steps: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub name: String,
    pub format: String,
    pub input: Value,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct DataBrewState {
    pub projects: DashMap<String, BrewProject>,
    pub recipes: DashMap<String, Recipe>,
    pub datasets: DashMap<String, Dataset>,
}

impl Default for DataBrewState {
    fn default() -> Self {
        Self {
            projects: DashMap::new(),
            recipes: DashMap::new(),
            datasets: DashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: Arc<DataBrewState>) -> axum::Router {
    axum::Router::new()
        .route("/projects", post(create_project).get(list_projects))
        .route("/projects/{name}", delete(delete_project))
        .route("/recipes", post(create_recipe).get(list_recipes))
        .route("/datasets", post(create_dataset).get(list_datasets))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_project(
    State(state): State<Arc<DataBrewState>>,
    Json(body): Json<Value>,
) -> Response {
    let name = match body["Name"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest("Missing Name".into()))
        }
    };

    if state.projects.contains_key(&name) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "Project already exists: {name}"
        )));
    }

    let now = Utc::now().to_rfc3339();
    let project = BrewProject {
        name: name.clone(),
        recipe_name: body["RecipeName"].as_str().unwrap_or("").to_string(),
        dataset_name: body["DatasetName"].as_str().unwrap_or("").to_string(),
        created_at: now,
    };

    state.projects.insert(name.clone(), project);
    rest_json::created(json!({ "Name": name }))
}

async fn list_projects(State(state): State<Arc<DataBrewState>>) -> Response {
    let items: Vec<Value> = state
        .projects
        .iter()
        .map(|entry| {
            let p = entry.value();
            json!({
                "Name": p.name,
                "RecipeName": p.recipe_name,
                "DatasetName": p.dataset_name,
                "CreateDate": p.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "Projects": items }))
}

async fn delete_project(
    State(state): State<Arc<DataBrewState>>,
    Path(name): Path<String>,
) -> Response {
    match state.projects.remove(&name) {
        Some(_) => rest_json::ok(json!({ "Name": name })),
        None => {
            rest_json::error_response(&LawsError::NotFound(format!("Project not found: {name}")))
        }
    }
}

async fn create_recipe(
    State(state): State<Arc<DataBrewState>>,
    Json(body): Json<Value>,
) -> Response {
    let name = match body["Name"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest("Missing Name".into()))
        }
    };

    if state.recipes.contains_key(&name) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "Recipe already exists: {name}"
        )));
    }

    let now = Utc::now().to_rfc3339();
    let recipe = Recipe {
        name: name.clone(),
        description: body["Description"].as_str().unwrap_or("").to_string(),
        steps: body["Steps"].clone(),
        created_at: now,
    };

    state.recipes.insert(name.clone(), recipe);
    rest_json::created(json!({ "Name": name }))
}

async fn list_recipes(State(state): State<Arc<DataBrewState>>) -> Response {
    let items: Vec<Value> = state
        .recipes
        .iter()
        .map(|entry| {
            let r = entry.value();
            json!({
                "Name": r.name,
                "Description": r.description,
                "CreateDate": r.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "Recipes": items }))
}

async fn create_dataset(
    State(state): State<Arc<DataBrewState>>,
    Json(body): Json<Value>,
) -> Response {
    let name = match body["Name"].as_str() {
        Some(n) => n.to_string(),
        None => {
            return rest_json::error_response(&LawsError::InvalidRequest("Missing Name".into()))
        }
    };

    if state.datasets.contains_key(&name) {
        return rest_json::error_response(&LawsError::AlreadyExists(format!(
            "Dataset already exists: {name}"
        )));
    }

    let now = Utc::now().to_rfc3339();
    let dataset = Dataset {
        name: name.clone(),
        format: body["Format"].as_str().unwrap_or("CSV").to_string(),
        input: body["Input"].clone(),
        created_at: now,
    };

    state.datasets.insert(name.clone(), dataset);
    rest_json::created(json!({ "Name": name }))
}

async fn list_datasets(State(state): State<Arc<DataBrewState>>) -> Response {
    let items: Vec<Value> = state
        .datasets
        .iter()
        .map(|entry| {
            let d = entry.value();
            json!({
                "Name": d.name,
                "Format": d.format,
                "CreateDate": d.created_at,
            })
        })
        .collect();

    rest_json::ok(json!({ "Datasets": items }))
}
